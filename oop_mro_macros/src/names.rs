use quote::format_ident;
use syn::Ident;

use crate::model::{Graph, VtableSlot};

pub(crate) fn base_field_ident(name: &str) -> Ident {
    format_ident!("__oop_base_{}", to_snake(name))
}

pub(crate) fn virtual_base_field_ident(name: &str) -> Ident {
    format_ident!("__oop_vbase_{}", to_snake(name))
}

pub(crate) fn base_cast_trait_ident(name: &str) -> Ident {
    format_ident!("As{}", name)
}

pub(crate) fn private_module_ident(graph: &Graph) -> Ident {
    format_ident!("__oop_private_{}", graph.names[0])
}

pub(crate) fn default_base_trait_ident(name: &str) -> Ident {
    format_ident!("__oop_DefaultBase_{}", name)
}

pub(crate) fn static_field_ident(class_name: &str, field_name: &str) -> Ident {
    format_ident!(
        "__oop_static_{}_{}",
        to_snake(class_name),
        to_snake(field_name)
    )
}

pub(crate) fn vtable_ident(name: &str) -> Ident {
    format_ident!("__oop_VTable_{}", name)
}

pub(crate) fn vtable_factory_ident(graph: &Graph, class_index: usize, slot: &VtableSlot) -> Ident {
    let class_name = &graph.names[class_index];
    let slot_name = vtable_slot_name(graph, slot);
    format_ident!(
        "__oop_vtable_{}_as_{}",
        to_snake(class_name),
        to_snake(&slot_name)
    )
}

pub(crate) fn vtable_field_ident(method: &Ident) -> Ident {
    format_ident!("__oop_vcall_{}", method)
}

pub(crate) fn vtable_cast_ref_field_ident() -> Ident {
    format_ident!("__oop_cast_ref")
}

pub(crate) fn vtable_cast_mut_field_ident() -> Ident {
    format_ident!("__oop_cast_mut")
}

pub(crate) fn vtable_downcast_ref_field_ident() -> Ident {
    format_ident!("__oop_downcast_ref")
}

pub(crate) fn vtable_downcast_mut_field_ident() -> Ident {
    format_ident!("__oop_downcast_mut")
}

pub(crate) fn vtable_cast_ref_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_cast_ref_{}_as_{}", class_name, slot_name)
}

pub(crate) fn vtable_cast_mut_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_cast_mut_{}_as_{}", class_name, slot_name)
}

pub(crate) fn vtable_downcast_ref_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_downcast_ref_{}_as_{}", class_name, slot_name)
}

pub(crate) fn vtable_downcast_mut_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_downcast_mut_{}_as_{}", class_name, slot_name)
}

pub(crate) fn vtable_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
    method: &Ident,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_vcall_{}_as_{}_{}", class_name, slot_name, method)
}

pub(crate) fn virtual_impl_ident(method: &Ident) -> Ident {
    format_ident!("__oop_impl_{}", method)
}

pub(crate) fn vtable_slot_name(graph: &Graph, slot: &VtableSlot) -> String {
    if slot.path.is_empty() {
        return graph.names[slot.ancestor].clone();
    }

    slot.path
        .iter()
        .map(|&index| graph.names[index].as_str())
        .collect::<Vec<_>>()
        .join("_")
}

pub(crate) fn to_snake(name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 && !output.ends_with('_') {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}
