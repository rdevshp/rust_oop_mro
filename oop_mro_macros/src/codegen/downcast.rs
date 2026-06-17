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

            let target_ty = class_type(&graph.classes[target]);
            for (source_type_index, source_ty) in
                downcast_source_types_for_target(graph, source, target)
                    .into_iter()
                    .enumerate()
            {
                let candidates =
                    owned_downcast_candidates(graph, source, target, &source_ty, &target_ty);
                if candidates.is_empty() {
                    continue;
                }

                impls.push(generate_box_downcast_impl(
                    graph,
                    source,
                    target,
                    source_type_index,
                    &source_ty,
                    &target_ty,
                    &candidates,
                ));
            }
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
    source_type_index: usize,
    source_ty: &Type,
    target_ty: &Type,
    candidates: &[OwnedDowncastCandidate],
) -> TokenStream2 {
    let class = &graph.classes[target];
    let mut generics = class.generics.clone();
    add_static_type_param_bounds(&mut generics);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let source_trait = base_cast_trait_for_actual_class(graph, source, source_ty);
    let target_trait = base_cast_trait_for_actual_class(graph, target, target_ty);
    let mut wrapper_items = Vec::new();
    let arms = candidates
        .iter()
        .enumerate()
        .map(|(candidate_index, candidate)| {
            let complete = candidate.complete;
            let source_id = candidate.source_id;
            let wrapper = owned_downcast_wrapper_ident(
                graph,
                source,
                target,
                complete,
                source_type_index,
                candidate_index,
            );
            wrapper_items.push(generate_owned_downcast_wrapper(
                graph,
                &wrapper,
                complete,
                &candidate.target_path,
                target,
            ));
            let complete_ty = class_type_tokens(&graph.classes[complete]);
            let (_, wrapper_ty_generics, _) = graph.classes[complete].generics.split_for_impl();
            let wrapper_expr_generics = wrapper_ty_generics.as_turbofish();
            quote! {
                (#complete, #source_id) => {
                    let data = <dyn #source_trait as #source_trait>::__oop_into_complete_owned(self);
                    let complete = unsafe {
                        ::std::boxed::Box::from_raw(data as *mut #complete_ty)
                    };
                    let target: ::std::boxed::Box<dyn #target_trait> =
                        ::std::boxed::Box::new(#wrapper #wrapper_expr_generics { complete });
                    ::core::result::Result::Ok(target)
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#wrapper_items)*

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
                let complete_id = <dyn #source_trait as #source_trait>::__oop_complete_class_id(&*self);
                let source_id = <dyn #source_trait as #source_trait>::__oop_source_subobject_id(&*self);
                match (complete_id, source_id) {
                    #(#arms,)*
                    _ => ::core::result::Result::Err(self),
                }
            }
        }
    }
}

fn downcast_source_types_for_target(graph: &Graph, source: usize, target: usize) -> Vec<Type> {
    let mut seen = HashSet::new();
    let mut source_types = Vec::new();

    for view in subobject_views(graph, target) {
        if view.class_index != source {
            continue;
        }

        let key = type_key(&view.actual);
        if seen.insert(key) {
            source_types.push(view.actual);
        }
    }

    source_types
}

#[derive(Clone)]
struct OwnedDowncastCandidate {
    complete: usize,
    source_id: usize,
    target_path: Vec<usize>,
}

fn owned_downcast_candidates(
    graph: &Graph,
    source: usize,
    target: usize,
    source_ty: &Type,
    target_ty: &Type,
) -> Vec<OwnedDowncastCandidate> {
    let source_key = type_key(source_ty);
    let target_key = type_key(target_ty);
    let mut grouped: Vec<(String, Vec<OwnedDowncastCandidate>)> = Vec::new();

    for (complete, class) in graph.classes.iter().enumerate() {
        if class.is_abstract {
            continue;
        }
        if !graph.mros[complete].contains(&source) || !graph.mros[complete].contains(&target) {
            continue;
        }
        if !compatible_owned_downcast_generics(graph, target, complete) {
            continue;
        }

        let source_views = subobject_views(graph, complete)
            .into_iter()
            .filter(|view| view.class_index == source && type_key(&view.actual) == source_key)
            .collect::<Vec<_>>();
        let target_views = subobject_views(graph, complete)
            .into_iter()
            .filter(|view| view.class_index == target && type_key(&view.actual) == target_key)
            .collect::<Vec<_>>();

        for source_view in &source_views {
            let source_id = subobject_id(graph, complete, &source_view.path);
            for target_view in &target_views {
                if !target_path_contains_source_path(
                    graph,
                    complete,
                    target,
                    &target_view.path,
                    &source_view.path,
                ) {
                    continue;
                }

                let key = format!("{}:{}", complete, source_id);
                let candidate = OwnedDowncastCandidate {
                    complete,
                    source_id,
                    target_path: target_view.path.clone(),
                };
                if let Some((_, candidates)) =
                    grouped.iter_mut().find(|(group_key, _)| group_key == &key)
                {
                    candidates.push(candidate);
                } else {
                    grouped.push((key, vec![candidate]));
                }
            }
        }
    }

    grouped
        .into_iter()
        .filter_map(|(_, mut candidates)| {
            if candidates.len() == 1 {
                candidates.pop()
            } else {
                None
            }
        })
        .collect()
}

fn owned_downcast_wrapper_ident(
    graph: &Graph,
    source_index: usize,
    target_index: usize,
    complete_index: usize,
    source_type_index: usize,
    candidate_index: usize,
) -> Ident {
    let prefix = to_snake(&graph.names[0]);
    format_ident!(
        "__oop_OwnedDowncast_{}_{}_{}_{}_{}_{}",
        prefix,
        complete_index,
        source_index,
        target_index,
        source_type_index,
        candidate_index
    )
}

fn generate_owned_downcast_wrapper(
    graph: &Graph,
    wrapper: &Ident,
    complete_index: usize,
    target_path: &[usize],
    target_index: usize,
) -> TokenStream2 {
    let complete_class = &graph.classes[complete_index];
    let wrapper_generics = &complete_class.generics;
    let complete_ty = class_type_tokens(complete_class);
    let (impl_generics, wrapper_ty_generics, where_clause) =
        complete_class.generics.split_for_impl();
    let wrapper_ty = quote! { #wrapper #wrapper_ty_generics };
    let private_module = private_module_ident(graph);
    let trait_impls = ancestor_views(graph, target_index)
        .into_iter()
        .map(|trait_view| {
            let trait_index = trait_view.class_index;
            let mut full_path = target_path.to_vec();
            full_path.extend(trait_view.path);
            let trait_actual = ancestor_type_for_path(graph, complete_index, &full_path);
            let trait_path = base_cast_trait_for_actual_class(graph, trait_index, &trait_actual);
            let shared_body = static_ref_expr_for_path(
                graph,
                complete_index,
                &full_path,
                quote! { self.complete.as_ref() },
                false,
            );
            let mutable_body = static_ref_expr_for_path(
                graph,
                complete_index,
                &full_path,
                quote! { self.complete.as_mut() },
                true,
            );
            let source_id = subobject_id(graph, complete_index, &full_path);
            let oop_shared_body = shared_body.clone();
            let oop_mutable_body = mutable_body.clone();

            quote! {
                impl #impl_generics ::oop_mro::OopBase<#trait_actual> for #wrapper_ty #where_clause {
                    fn __oop_as_base(&self) -> &#trait_actual {
                        #oop_shared_body
                    }

                    fn __oop_as_base_mut(&mut self) -> &mut #trait_actual {
                        #oop_mutable_body
                    }
                }

                impl #impl_generics #trait_path for #wrapper_ty #where_clause {
                    fn __oop_as_self(&self) -> &#trait_actual {
                        #shared_body
                    }

                    fn __oop_as_self_mut(&mut self) -> &mut #trait_actual {
                        #mutable_body
                    }

                    fn __oop_complete_class_id(&self) -> usize {
                        #complete_index
                    }

                    fn __oop_source_subobject_id(&self) -> usize {
                        #source_id
                    }

                    fn __oop_into_complete_owned(self: ::std::boxed::Box<Self>) -> *mut () {
                        let #wrapper { complete } = *self;
                        ::std::boxed::Box::into_raw(complete) as *mut ()
                    }

                    fn __oop_cast_seal(&self) -> #private_module::Seal {
                        #private_module::Seal
                    }
                }
            }
        });

    quote! {
        struct #wrapper #wrapper_generics {
            complete: ::std::boxed::Box<#complete_ty>,
        }

        #(#trait_impls)*
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

    for target in 0..graph.classes.len() {
        for source in 0..graph.classes.len() {
            if !needs_runtime_metadata(graph, source) {
                continue;
            }
            if !graph.mros[target].contains(&source) {
                continue;
            }

            for source_ty in downcast_source_types_for_target(graph, source, target) {
                let types = BorrowedDowncastTypes {
                    impl_class: &graph.classes[target],
                    source_ty,
                    target_ty: class_type(&graph.classes[target]),
                };
                let key = format!(
                    "{}=>{}",
                    types.source_ty.to_token_stream(),
                    types.target_ty.to_token_stream()
                );
                if !seen.insert(key) {
                    continue;
                }

                impls.push(generate_borrowed_downcast_impl(graph, target, &types));
            }
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

fn generate_borrowed_downcast_impl(
    graph: &Graph,
    target: usize,
    types: &BorrowedDowncastTypes<'_>,
) -> TokenStream2 {
    let (impl_generics, _, where_clause) = types.impl_class.generics.split_for_impl();
    let source_ty = &types.source_ty;
    let target_ty = &types.target_ty;
    let target_id = cast_target_id(graph, target, target_ty);
    let downcast_ref = vtable_downcast_ref_field_ident();
    let downcast_mut = vtable_downcast_mut_field_ident();

    quote! {
        impl #impl_generics ::oop_mro::OopDowncastRefTarget<#target_ty> for #source_ty #where_clause {
            fn downcast_ref_target(&self) -> ::core::option::Option<&#target_ty> {
                let ptr = unsafe {
                    (self.__oop_vtable.#downcast_ref)(self as *const #source_ty, #target_id)
                }?;
                ::core::option::Option::Some(unsafe { &*(ptr as *const #target_ty) })
            }
        }

        impl #impl_generics ::oop_mro::OopDowncastMutTarget<#target_ty> for #source_ty #where_clause {
            fn downcast_mut_target(&mut self) -> ::core::option::Option<&mut #target_ty> {
                let ptr = unsafe {
                    (self.__oop_vtable.#downcast_mut)(self as *mut #source_ty, #target_id)
                }?;
                ::core::option::Option::Some(unsafe { &mut *(ptr as *mut #target_ty) })
            }
        }
    }
}
