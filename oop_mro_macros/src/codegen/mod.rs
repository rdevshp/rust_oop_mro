use std::collections::{BTreeMap, HashSet};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, GenericParam, Generics, Ident, Type};

use crate::ast::{ClassDef, ClassItem, ConstructorDef, MethodDef};
use crate::generics::{
    async_output_type_with_substitutions, method_signature_key_in_context, substitute_type,
    substituted_return_type, substitutions_from_context,
};
use crate::model::{Graph, MethodInfo, ReceiverKind, VtableSlot};
use crate::names::{
    base_cast_method_ident, base_field_ident, default_base_trait_ident, private_module_ident,
    virtual_impl_ident, vtable_cast_mut_field_ident, vtable_cast_mut_function_ident,
    vtable_cast_ref_field_ident, vtable_cast_ref_function_ident, vtable_factory_ident,
    vtable_field_ident, vtable_function_ident, vtable_ident,
};
use crate::types::{
    ancestor_type, async_dispatch_lifetime, async_output_type, base_cast_trait_for_actual_class,
    boxed_future_type, class_constructor, class_type, class_type_tokens, constructor_arg_idents,
    find_base_path, generics_with_async_lifetime, signature_in_context,
    type_with_elided_refs_lifetime, vtable_type_for_actual_class, vtable_type_for_class,
};
use crate::validate::public_if_inherited;

mod casts;
mod constructors;
mod downcast;
mod impls;
mod metadata;
mod structs;
mod vtables;

pub(crate) fn generate(graph: &Graph) -> TokenStream2 {
    let private_module = structs::generate_private_module(graph);
    let vtable_structs = graph
        .classes
        .iter()
        .enumerate()
        .filter(|(index, _)| has_virtual_interface(graph, *index))
        .map(|(index, class)| vtables::generate_vtable_struct(graph, index, class));
    let structs = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| structs::generate_struct(graph, index, class));
    let base_cast_traits = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| casts::generate_base_cast_trait(graph, index, class));
    let impls = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| impls::generate_impls(graph, index, class));
    let base_cast_impls = casts::generate_base_cast_impls(graph);
    let vtable_items = vtables::generate_vtable_items(graph);
    let downcast_impls = downcast::generate_downcast_impls(graph);

    quote! {
        #private_module
        #(#vtable_structs)*
        #(#structs)*
        #(#base_cast_traits)*
        #(#impls)*
        #base_cast_impls
        #vtable_items
        #downcast_impls
    }
}

fn interface_methods(graph: &Graph, index: usize) -> Vec<MethodInfo> {
    let mut methods = BTreeMap::new();

    for (name, method) in &graph.selected_methods[index] {
        methods.insert(name.clone(), method.clone());
    }

    for (name, method) in &graph.abstract_methods[index] {
        methods.insert(name.clone(), method.clone());
    }

    methods.into_values().collect()
}

fn has_virtual_interface(graph: &Graph, index: usize) -> bool {
    !graph.selected_methods[index].is_empty() || !graph.abstract_methods[index].is_empty()
}

fn vtable_slots(graph: &Graph, index: usize) -> Vec<VtableSlot> {
    let mut slots = Vec::new();
    collect_vtable_slots(graph, index, Vec::new(), &mut slots);
    slots
}

fn collect_vtable_slots(
    graph: &Graph,
    index: usize,
    path: Vec<usize>,
    slots: &mut Vec<VtableSlot>,
) {
    if has_virtual_interface(graph, index) {
        slots.push(VtableSlot {
            ancestor: index,
            path: path.clone(),
        });
    }

    for &base in &graph.bases[index] {
        let mut base_path = path.clone();
        base_path.push(base);
        collect_vtable_slots(graph, base, base_path, slots);
    }
}

fn accessor_body(graph: &Graph, start: usize, target: usize, mutable: bool) -> TokenStream2 {
    let path = find_base_path(graph, start, target).expect("ancestor path must exist");
    let mut tokens = quote! { self };

    for base in path {
        let field = base_field_ident(&graph.names[base]);
        tokens = quote! { #tokens.#field };
    }

    if mutable {
        quote! { &mut #tokens }
    } else {
        quote! { &#tokens }
    }
}
