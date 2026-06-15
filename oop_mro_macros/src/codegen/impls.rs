use super::*;

pub(super) fn generate_impls(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let name = &class.name;
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();
    let metadata_impl = metadata::generate_metadata_impl(graph, index, class);
    let default_impl = constructors::generate_default_impl(graph, index, class);
    let default_base_impl = constructors::generate_default_base_impl(graph, index, class);
    let accessors = generate_accessors(graph, index, class);
    let vtable_init = constructors::generate_vtable_init(graph, index);
    let constructor_hook = constructors::generate_constructor_hook(graph, index, class);
    let constructor_new = constructors::generate_constructor_new(graph, index, class);
    let direct_items = class.items.iter().filter_map(|item| match item {
        ClassItem::AssociatedConst(associated_const) => {
            Some(generate_associated_const(associated_const))
        }
        ClassItem::StaticField(static_field) => {
            Some(generate_static_field_accessor(graph, index, static_field))
        }
        ClassItem::Method(method) if !method.is_virtual => generate_direct_method(method),
        ClassItem::Field(_) => None,
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
        ClassItem::UnsupportedAssociatedType(_) => None,
    });
    let virtual_impl_methods = class.items.iter().filter_map(|item| match item {
        ClassItem::Method(method) if method.is_virtual => generate_virtual_impl_method(method),
        ClassItem::Field(_) => None,
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
        ClassItem::AssociatedConst(_) => None,
        ClassItem::StaticField(_) => None,
        ClassItem::UnsupportedAssociatedType(_) => None,
    });
    let virtual_wrappers = interface_methods(graph, index)
        .into_iter()
        .map(|method| generate_virtual_wrapper(graph, index, &method));

    quote! {
        #metadata_impl

        #default_base_impl

        impl #impl_generics #name #ty_generics #where_clause {
            #vtable_init
            #accessors
            #constructor_hook
            #constructor_new
            #(#direct_items)*
            #(#virtual_impl_methods)*
            #(#virtual_wrappers)*
        }

        #default_impl
    }
}

fn generate_static_field_accessor(
    graph: &Graph,
    class_index: usize,
    static_field: &StaticFieldDef,
) -> TokenStream2 {
    let attrs = &static_field.attrs;
    let vis = public_if_inherited(&static_field.vis);
    let ident = &static_field.ident;
    let hidden = static_field_ident(&graph.names[class_index], &static_field.ident.to_string());
    let ty = &static_field.ty;

    quote! {
        #(#attrs)*
        #vis const #ident: &'static #ty = &#hidden;
    }
}

fn generate_associated_const(associated_const: &AssociatedConstDef) -> TokenStream2 {
    let mut item = associated_const.item.clone();
    item.vis = public_if_inherited(&item.vis);
    quote! {
        #item
    }
}

fn generate_direct_method(method: &MethodDef) -> Option<TokenStream2> {
    let body = method.body.as_ref()?;
    let attrs = &method.attrs;
    let vis = public_if_inherited(&method.vis);
    let sig = &method.sig;

    Some(quote! {
        #(#attrs)*
        #vis #sig #body
    })
}

fn generate_virtual_impl_method(method: &MethodDef) -> Option<TokenStream2> {
    let body = method.body.as_ref()?;
    let attrs = &method.attrs;
    let mut sig = method.sig.clone();
    sig.ident = virtual_impl_ident(&method.sig.ident);

    Some(quote! {
        #(#attrs)*
        #sig #body
    })
}

fn generate_virtual_wrapper(graph: &Graph, index: usize, method: &MethodInfo) -> TokenStream2 {
    let sig = signature_in_context(graph, index, method);
    let vis = &method.vis;
    let field = vtable_field_ident(&method.name);
    let args = &method.arg_idents;
    let call = quote! {
        (self.__oop_vtable.#field)(self, #(#args),*)
    };
    let body = if method.sig.unsafety.is_some() {
        if method.sig.asyncness.is_some() {
            quote! {
                unsafe { #call }.await
            }
        } else {
            quote! {
                unsafe { #call }
            }
        }
    } else if method.sig.asyncness.is_some() {
        quote! {
            #call.await
        }
    } else {
        call
    };

    quote! {
        #vis #sig {
            #body
        }
    }
}

fn generate_accessors(graph: &Graph, index: usize, _class: &ClassDef) -> TokenStream2 {
    let mut generated = Vec::new();
    let mut seen = HashSet::new();

    for &ancestor in graph.mros[index].iter().skip(1) {
        if !seen.insert(ancestor) {
            continue;
        }

        let ancestor_name = &graph.names[ancestor];
        let ancestor_ty = ancestor_type(graph, index, ancestor);
        let shared_name = format_ident!("__oop_as_{}", ancestor_name);
        let mutable_name = format_ident!("__oop_as_mut_{}", ancestor_name);
        let shared_body = accessor_body(graph, index, ancestor, false);
        let mutable_body = accessor_body(graph, index, ancestor, true);

        generated.push(quote! {
            fn #shared_name(&self) -> &#ancestor_ty {
                #shared_body
            }

            fn #mutable_name(&mut self) -> &mut #ancestor_ty {
                #mutable_body
            }
        });
    }

    quote! {
        #(#generated)*
    }
}
