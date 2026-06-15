use super::*;

pub(super) fn generate_base_cast_trait(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let vis = public_if_inherited(&class.vis);
    let trait_name = crate::names::base_cast_trait_ident(&graph.names[index]);
    let private_module = private_module_ident(graph);
    let (trait_generics, _, where_clause) = class.generics.split_for_impl();
    let class_ty = class_type_tokens(class);
    let shared_name = base_cast_method_ident(&graph.names[index], false);
    let mutable_name = base_cast_method_ident(&graph.names[index], true);
    let supertraits = class
        .bases
        .iter()
        .map(|base| {
            let base_index = graph.name_to_index[&base.name.to_string()];
            base_cast_trait_for_actual_class(graph, base_index, &base.ty)
        })
        .collect::<Vec<_>>();
    let supertrait_bound = (!supertraits.is_empty()).then(|| {
        quote! {
            : #(#supertraits)+*
        }
    });

    quote! {
        #vis trait #trait_name #trait_generics #supertrait_bound #where_clause {
            fn #shared_name(&self) -> &#class_ty;
            fn #mutable_name(&mut self) -> &mut #class_ty;

            #[doc(hidden)]
            fn __oop_complete_class_id(&self) -> usize;

            #[doc(hidden)]
            fn __oop_cast_seal(&self) -> #private_module::Seal;
        }
    }
}

pub(super) fn generate_base_cast_impls(graph: &Graph) -> TokenStream2 {
    let impls = graph
        .classes
        .iter()
        .enumerate()
        .flat_map(|(class_index, class)| {
            graph.mros[class_index]
                .iter()
                .copied()
                .map(move |base_index| {
                    generate_base_cast_impl(graph, class_index, base_index, class)
                })
        });

    quote! {
        #(#impls)*
    }
}

fn generate_base_cast_impl(
    graph: &Graph,
    class_index: usize,
    base_index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let class_ty = class_type_tokens(class);
    let base_ty = ancestor_type(graph, class_index, base_index);
    let trait_path = base_cast_trait_for_actual_class(graph, base_index, &base_ty);
    let shared_name = base_cast_method_ident(&graph.names[base_index], false);
    let mutable_name = base_cast_method_ident(&graph.names[base_index], true);
    let shared_body = if class_index == base_index {
        quote! { self }
    } else {
        accessor_body(graph, class_index, base_index, false)
    };
    let mutable_body = if class_index == base_index {
        quote! { self }
    } else {
        accessor_body(graph, class_index, base_index, true)
    };
    let private_module = private_module_ident(graph);

    quote! {
        impl #impl_generics #trait_path for #class_ty #where_clause {
            fn #shared_name(&self) -> &#base_ty {
                #shared_body
            }

            fn #mutable_name(&mut self) -> &mut #base_ty {
                #mutable_body
            }

            fn __oop_complete_class_id(&self) -> usize {
                #class_index
            }

            fn __oop_cast_seal(&self) -> #private_module::Seal {
                #private_module::Seal
            }
        }
    }
}
