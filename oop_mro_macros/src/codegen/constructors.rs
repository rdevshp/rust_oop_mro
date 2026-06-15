use super::*;

pub(super) fn generate_constructor_hook(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let constructor = class_constructor(class);
    let inputs = constructor
        .map(|constructor| constructor.inputs.as_slice())
        .unwrap_or(&[]);
    let base_calls = generate_constructor_base_calls(graph, index, constructor);
    let body = constructor.map(|constructor| &constructor.body);

    quote! {
        fn __oop_ctor(&mut self #(, #inputs)*) {
            #base_calls
            #body
        }
    }
}

pub(super) fn generate_constructor_new(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    if class.is_abstract {
        return quote! {};
    }

    let Some(constructor) = class_constructor(class) else {
        return quote! {};
    };

    let attrs = &constructor.attrs;
    let vis = public_if_inherited(&constructor.vis);
    let inputs = &constructor.inputs;
    let args = constructor_arg_idents(constructor);
    let trait_name = default_base_trait_ident(&graph.names[index]);

    quote! {
        #(#attrs)*
        #vis fn new(#(#inputs),*) -> Self {
            let mut __oop_value = <Self as #trait_name>::__oop_default_base();
            __oop_value.__oop_ctor(#(#args),*);
            __oop_value
        }
    }
}

fn generate_constructor_base_calls(
    graph: &Graph,
    index: usize,
    constructor: Option<&ConstructorDef>,
) -> TokenStream2 {
    let calls = graph.bases[index].iter().map(|&base| {
        let base_name = &graph.names[base];
        let accessor = format_ident!("__oop_as_mut_{}", base_name);
        let explicit_call = constructor.and_then(|constructor| {
            constructor
                .base_calls
                .iter()
                .find(|base_call| base_call.base == base_name.as_str())
        });

        if let Some(base_call) = explicit_call {
            let args = &base_call.args;
            quote! {
                self.#accessor().__oop_ctor(#(#args),*);
            }
        } else {
            quote! {
                self.#accessor().__oop_ctor();
            }
        }
    });

    quote! {
        #(#calls)*
    }
}

pub(super) fn generate_default_base_impl(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let name = &class.name;
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();
    let trait_name = default_base_trait_ident(&graph.names[index]);
    let vtable_initializer = has_virtual_interface(graph, index).then(|| {
        let vtable = vtable_factory_ident(
            graph,
            index,
            &VtableSlot {
                ancestor: index,
                path: Vec::new(),
            },
        );
        quote! {
            __oop_vtable: #vtable(),
        }
    });
    let base_initializers = graph.bases[index].iter().map(|&base| {
        let field = base_field_ident(&graph.names[base]);
        let base_ty = ancestor_type(graph, index, base);
        let base_trait = default_base_trait_ident(&graph.names[base]);
        quote! {
            #field: <#base_ty as #base_trait>::__oop_default_base()
        }
    });
    let field_initializers = class.items.iter().filter_map(|item| match item {
        ClassItem::Field(field) => {
            let ident = &field.ident;
            Some(quote! {
                #ident: ::core::default::Default::default()
            })
        }
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });

    quote! {
        trait #trait_name {
            fn __oop_default_base() -> Self;
        }

        impl #impl_generics #trait_name for #name #ty_generics #where_clause {
            fn __oop_default_base() -> Self {
                let mut value = Self {
                    #vtable_initializer
                    #(#base_initializers,)*
                    #(#field_initializers,)*
                };
                value.__oop_init_vtables();
                value
            }
        }
    }
}

pub(super) fn generate_default_impl(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    if class.is_abstract {
        return quote! {};
    }

    let name = &class.name;
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();
    let trait_name = default_base_trait_ident(&graph.names[index]);
    quote! {
        impl #impl_generics ::core::default::Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                <Self as #trait_name>::__oop_default_base()
            }
        }
    }
}

pub(super) fn generate_vtable_init(graph: &Graph, index: usize) -> TokenStream2 {
    let assignments = vtable_slots(graph, index).into_iter().map(|slot| {
        let vtable = vtable_factory_ident(graph, index, &slot);
        let place = place_for_path(graph, &slot.path);

        quote! {
            #place.__oop_vtable = #vtable();
        }
    });

    quote! {
        fn __oop_init_vtables(&mut self) {
            #(#assignments)*
        }
    }
}

fn place_for_path(graph: &Graph, path: &[usize]) -> TokenStream2 {
    let mut tokens = quote! { self };

    for &base in path {
        let field = base_field_ident(&graph.names[base]);
        tokens = quote! { #tokens.#field };
    }

    tokens
}
