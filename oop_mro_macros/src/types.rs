use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote, FnArg, GenericParam, Generics, Ident, Lifetime, Pat, Signature, Type,
    TypeReference,
};

use crate::ast::{ClassDef, ConstructorDef};
use crate::generics::{
    substitute_type, substituted_return_type, substitutions_for_class_type,
    substitutions_from_context, GenericSubstitutions,
};
use crate::model::{Graph, MethodInfo};
use crate::names::{base_cast_trait_ident, vtable_ident};

pub(crate) fn class_type(class: &ClassDef) -> Type {
    let name = &class.name;
    let (_, ty_generics, _) = class.generics.split_for_impl();
    parse_quote!(#name #ty_generics)
}

pub(crate) fn class_type_tokens(class: &ClassDef) -> TokenStream2 {
    let ty = class_type(class);
    quote! { #ty }
}

pub(crate) fn ancestor_type(graph: &Graph, start: usize, target: usize) -> Type {
    ancestor_type_in(&graph.classes, &graph.bases, &graph.mros, start, target)
}

pub(crate) fn ancestor_type_in(
    classes: &[ClassDef],
    bases: &[Vec<usize>],
    mros: &[Vec<usize>],
    start: usize,
    target: usize,
) -> Type {
    if start == target {
        return class_type(&classes[start]);
    }

    let Some(path) = find_base_path_in(bases, mros, start, target) else {
        return class_type(&classes[target]);
    };

    let mut current = start;
    let mut substitutions = GenericSubstitutions::default();
    let mut actual = class_type(&classes[target]);
    for next in path {
        let Some(base_spec) = classes[current]
            .bases
            .iter()
            .find(|base| base.name == classes[next].name)
        else {
            return actual;
        };
        actual = substitute_type(&base_spec.ty, &substitutions);
        substitutions = substitutions_for_class_type(&classes[next], &actual);
        current = next;
    }

    actual
}

pub(crate) fn vtable_type_for_class(graph: &Graph, index: usize) -> TokenStream2 {
    let vtable_name = vtable_ident(&graph.names[index]);
    let (_, ty_generics, _) = graph.classes[index].generics.split_for_impl();
    quote! { #vtable_name #ty_generics }
}

pub(crate) fn vtable_type_for_actual_class(
    graph: &Graph,
    class_index: usize,
    actual: &Type,
) -> TokenStream2 {
    type_with_replaced_ident(actual, vtable_ident(&graph.names[class_index]))
}

pub(crate) fn base_cast_trait_for_actual_class(
    graph: &Graph,
    class_index: usize,
    actual: &Type,
) -> TokenStream2 {
    type_with_replaced_ident(actual, base_cast_trait_ident(&graph.names[class_index]))
}

pub(crate) fn type_with_replaced_ident(ty: &Type, ident: Ident) -> TokenStream2 {
    let mut ty = ty.clone();
    if let Type::Path(path) = &mut ty {
        if path.qself.is_none() && path.path.segments.len() == 1 {
            path.path.segments[0].ident = ident;
        }
    }

    quote! { #ty }
}

pub(crate) fn async_dispatch_lifetime() -> Lifetime {
    parse_quote!('__oop_async)
}

pub(crate) fn async_output_type(sig: &Signature, lifetime: &Lifetime) -> TokenStream2 {
    match &sig.output {
        syn::ReturnType::Default => quote! { () },
        syn::ReturnType::Type(_, ty) => {
            type_with_elided_refs_lifetime(ty, lifetime).into_token_stream()
        }
    }
}

pub(crate) fn signature_in_context(
    graph: &Graph,
    context: usize,
    method: &MethodInfo,
) -> Signature {
    let substitutions = substitutions_from_context(
        &graph.classes,
        &graph.bases,
        &graph.mros,
        context,
        method.owner,
    );
    let mut sig = method.sig.clone();

    for input in &mut sig.inputs {
        if let FnArg::Typed(typed) = input {
            *typed.ty = substitute_type(&typed.ty, &substitutions);
        }
    }
    sig.output = substituted_return_type(&sig.output, &substitutions);

    sig
}

pub(crate) fn boxed_future_type(output: TokenStream2, lifetime: &Lifetime) -> TokenStream2 {
    quote! {
        ::core::pin::Pin<
            ::std::boxed::Box<
                dyn ::core::future::Future<Output = #output> + #lifetime
            >
        >
    }
}

pub(crate) fn type_with_elided_refs_lifetime(ty: &Type, lifetime: &Lifetime) -> Type {
    let mut ty = ty.clone();
    let mut binder = ElidedReferenceLifetimeBinder { lifetime };
    binder.visit_type_mut(&mut ty);
    ty
}

struct ElidedReferenceLifetimeBinder<'a> {
    lifetime: &'a Lifetime,
}

impl VisitMut for ElidedReferenceLifetimeBinder<'_> {
    fn visit_type_reference_mut(&mut self, node: &mut TypeReference) {
        if node.lifetime.is_none() {
            node.lifetime = Some(self.lifetime.clone());
        }

        visit_mut::visit_type_reference_mut(self, node);
    }
}

pub(crate) fn generics_with_async_lifetime(generics: &Generics) -> Generics {
    let mut generics = generics.clone();
    let lifetime = async_dispatch_lifetime();
    generics
        .params
        .insert(0, GenericParam::Lifetime(parse_quote!(#lifetime)));
    generics
}

pub(crate) fn class_constructors(class: &ClassDef) -> impl Iterator<Item = &ConstructorDef> {
    class.items.iter().filter_map(|item| match item {
        crate::ast::ClassItem::Constructor(constructor) => Some(constructor),
        crate::ast::ClassItem::Field(_)
        | crate::ast::ClassItem::Method(_)
        | crate::ast::ClassItem::AssociatedConst(_)
        | crate::ast::ClassItem::StaticField(_)
        | crate::ast::ClassItem::UnsupportedAssociatedType(_) => None,
    })
}

pub(crate) fn class_constructor(class: &ClassDef) -> Option<&ConstructorDef> {
    class_constructors(class).next()
}

pub(crate) fn constructor_arg_idents(constructor: &ConstructorDef) -> Vec<Ident> {
    constructor
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}

pub(crate) fn find_base_path(graph: &Graph, start: usize, target: usize) -> Option<Vec<usize>> {
    find_base_path_in(&graph.bases, &graph.mros, start, target)
}

pub(crate) fn find_base_path_in(
    bases: &[Vec<usize>],
    mros: &[Vec<usize>],
    start: usize,
    target: usize,
) -> Option<Vec<usize>> {
    for &base in &bases[start] {
        if base == target {
            return Some(vec![base]);
        }

        if mros[base].contains(&target) {
            let mut path = vec![base];
            path.extend(find_base_path_in(bases, mros, base, target)?);
            return Some(path);
        }
    }

    None
}
