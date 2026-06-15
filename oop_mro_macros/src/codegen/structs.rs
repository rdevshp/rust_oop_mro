use super::*;

pub(super) fn generate_private_module(graph: &Graph) -> TokenStream2 {
    let module = private_module_ident(graph);

    quote! {
        mod #module {
            pub struct Seal;
        }
    }
}

pub(super) fn generate_struct(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let attrs = &class.attrs;
    let vis = public_if_inherited(&class.vis);
    let name = &class.name;
    let generics = &class.generics;
    let vtable_field = has_virtual_interface(graph, index).then(|| {
        let vtable_ty = vtable_type_for_class(graph, index);
        quote! {
            __oop_vtable: #vtable_ty,
        }
    });
    let base_fields = class.bases.iter().map(|base| {
        let field = base_field_ident(&base.name.to_string());
        let base_ty = &base.ty;
        quote! {
            #field: #base_ty
        }
    });
    let fields = class.items.iter().filter_map(|item| match item {
        ClassItem::Field(field) => Some(quote! { #field }),
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });

    quote! {
        #(#attrs)*
        #[repr(C)]
        #vis struct #name #generics {
            #vtable_field
            #(#base_fields,)*
            #(#fields,)*
        }
    }
}
