use super::*;

pub(super) fn generate_downcast_impls(graph: &Graph) -> TokenStream2 {
    let box_impls = generate_box_downcast_impls(graph);
    let borrowed_impls = generate_borrowed_downcast_impls(graph);

    quote! {
        #box_impls
        #borrowed_impls
    }
}

fn generate_box_downcast_impls(graph: &Graph) -> TokenStream2 {
    let mut impls = Vec::new();

    for source in 0..graph.classes.len() {
        for target in 0..graph.classes.len() {
            if source == target {
                continue;
            }
            if !graph.mros[target].contains(&source) {
                continue;
            }

            let candidates = graph
                .classes
                .iter()
                .enumerate()
                .filter(|(complete, class)| {
                    !class.is_abstract
                        && graph.mros[*complete].contains(&target)
                        && compatible_owned_downcast_generics(graph, target, *complete)
                })
                .map(|(complete, _)| complete)
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                continue;
            }

            impls.push(generate_box_downcast_impl(
                graph,
                source,
                target,
                &candidates,
            ));
        }
    }

    quote! {
        #(#impls)*
    }
}

fn compatible_owned_downcast_generics(graph: &Graph, target: usize, complete: usize) -> bool {
    if target == complete {
        return true;
    }

    if graph.classes[target].generics.params.is_empty()
        && graph.classes[complete].generics.params.is_empty()
    {
        return true;
    }

    generic_params_key(&graph.classes[target].generics)
        == generic_params_key(&graph.classes[complete].generics)
}

fn generic_params_key(generics: &Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Type(param) => param.ident.to_string(),
            GenericParam::Lifetime(param) => param.lifetime.ident.to_string(),
            GenericParam::Const(param) => param.ident.to_string(),
        })
        .collect()
}

fn generate_box_downcast_impl(
    graph: &Graph,
    source: usize,
    target: usize,
    candidates: &[usize],
) -> TokenStream2 {
    let class = &graph.classes[target];
    let mut generics = class.generics.clone();
    add_static_type_param_bounds(&mut generics);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let source_ty = ancestor_type(graph, target, source);
    let target_ty = class_type(&graph.classes[target]);
    let source_trait = base_cast_trait_for_actual_class(graph, source, &source_ty);
    let target_trait = base_cast_trait_for_actual_class(graph, target, &target_ty);
    let arms = candidates.iter().copied().map(|complete| {
        let complete_ty = class_type_tokens(&graph.classes[complete]);
        quote! {
            #complete => {
                let raw = ::std::boxed::Box::into_raw(self);
                let data = raw as *mut ();
                let complete = unsafe {
                    ::std::boxed::Box::from_raw(data as *mut #complete_ty)
                };
                let target: ::std::boxed::Box<dyn #target_trait> = complete;
                ::core::result::Result::Ok(target)
            }
        }
    });

    quote! {
        impl #impl_generics ::oop_mro::OopBoxDowncastTarget<dyn #target_trait>
            for dyn #source_trait
            #where_clause
        {
            fn downcast_target(
                self: ::std::boxed::Box<Self>,
            ) -> ::core::result::Result<
                ::std::boxed::Box<dyn #target_trait>,
                ::std::boxed::Box<Self>,
            > {
                match <dyn #source_trait as #source_trait>::__oop_complete_class_id(&*self) {
                    #(#arms,)*
                    _ => ::core::result::Result::Err(self),
                }
            }
        }
    }
}

fn add_static_type_param_bounds(generics: &mut Generics) {
    let type_idents = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(param) => Some(param.ident.clone()),
            GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        })
        .collect::<Vec<_>>();

    let where_clause = generics.make_where_clause();
    for ident in type_idents {
        where_clause.predicates.push(parse_quote!(#ident: 'static));
    }
}

fn generate_borrowed_downcast_impls(graph: &Graph) -> TokenStream2 {
    let mut impls = Vec::new();
    let mut seen = HashSet::new();

    for source in 0..graph.classes.len() {
        if !has_virtual_interface(graph, source) {
            continue;
        }

        for target in 0..graph.classes.len() {
            let Some(types) = borrowed_downcast_types(graph, source, target) else {
                continue;
            };
            let key = format!(
                "{}=>{}",
                types.source_ty.to_token_stream(),
                types.target_ty.to_token_stream()
            );
            if !seen.insert(key) {
                continue;
            }

            impls.push(generate_borrowed_downcast_impl(target, &types));
        }
    }

    quote! {
        #(#impls)*
    }
}

struct BorrowedDowncastTypes<'a> {
    impl_class: &'a ClassDef,
    source_ty: Type,
    target_ty: Type,
}

fn borrowed_downcast_types(
    graph: &Graph,
    source: usize,
    target: usize,
) -> Option<BorrowedDowncastTypes<'_>> {
    if graph.mros[target].contains(&source) {
        return Some(BorrowedDowncastTypes {
            impl_class: &graph.classes[target],
            source_ty: ancestor_type(graph, target, source),
            target_ty: class_type(&graph.classes[target]),
        });
    }

    if !graph.classes[source].generics.params.is_empty()
        || !graph.classes[target].generics.params.is_empty()
    {
        return None;
    }

    let complete = graph
        .classes
        .iter()
        .enumerate()
        .find(|(complete, class)| {
            !class.is_abstract
                && class.generics.params.is_empty()
                && graph.mros[*complete].contains(&source)
                && graph.mros[*complete].contains(&target)
        })
        .map(|(complete, _)| complete)?;

    Some(BorrowedDowncastTypes {
        impl_class: &graph.classes[complete],
        source_ty: ancestor_type(graph, complete, source),
        target_ty: ancestor_type(graph, complete, target),
    })
}

fn generate_borrowed_downcast_impl(
    target: usize,
    types: &BorrowedDowncastTypes<'_>,
) -> TokenStream2 {
    let (impl_generics, _, where_clause) = types.impl_class.generics.split_for_impl();
    let source_ty = &types.source_ty;
    let target_ty = &types.target_ty;
    let cast_ref = vtable_cast_ref_field_ident();
    let cast_mut = vtable_cast_mut_field_ident();

    quote! {
        impl #impl_generics ::oop_mro::OopDowncastRefTarget<#target_ty> for #source_ty #where_clause {
            fn downcast_ref_target(&self) -> ::core::option::Option<&#target_ty> {
                let ptr = unsafe {
                    (self.__oop_vtable.#cast_ref)(self as *const #source_ty, #target)
                }?;
                ::core::option::Option::Some(unsafe { &*(ptr as *const #target_ty) })
            }
        }

        impl #impl_generics ::oop_mro::OopDowncastMutTarget<#target_ty> for #source_ty #where_clause {
            fn downcast_mut_target(&mut self) -> ::core::option::Option<&mut #target_ty> {
                let ptr = unsafe {
                    (self.__oop_vtable.#cast_mut)(self as *mut #source_ty, #target)
                }?;
                ::core::option::Option::Some(unsafe { &mut *(ptr as *mut #target_ty) })
            }
        }
    }
}
