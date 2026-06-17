use super::*;

pub(super) fn generate_base_cast_impls(graph: &Graph) -> TokenStream2 {
    let impls = graph
        .classes
        .iter()
        .enumerate()
        .flat_map(|(class_index, class)| {
            ancestor_views(graph, class_index)
                .into_iter()
                .map(move |view| generate_base_cast_impl(graph, class_index, &view, class))
        });
    let via_impls = graph
        .classes
        .iter()
        .enumerate()
        .flat_map(|(class_index, class)| {
            base_via_views(graph, class_index)
                .into_iter()
                .map(move |view| generate_concrete_base_via_impl(graph, class_index, &view, class))
        });
    let owned_via_items = generate_owned_base_via_items(graph);

    quote! {
        #(#impls)*
        #(#via_impls)*
        #owned_via_items
    }
}

fn generate_base_cast_impl(
    graph: &Graph,
    class_index: usize,
    view: &AncestorView,
    class: &ClassDef,
) -> TokenStream2 {
    let base_index = view.class_index;
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let class_ty = class_type_tokens(class);
    let base_ty = &view.actual;
    let trait_path = as_class_trait_for_actual(base_ty);
    let has_virtual_edge = path_has_virtual_edge_for_path(graph, class_index, &view.path);
    let shared_body = if class_index == base_index && view.path.is_empty() {
        quote! { self }
    } else if has_virtual_edge {
        dynamic_accessor_body_for_actual(graph, class_index, base_index, base_ty, false)
    } else {
        static_ref_expr_for_path(graph, class_index, &view.path, quote! { self }, false)
    };
    let mutable_body = if class_index == base_index && view.path.is_empty() {
        quote! { self }
    } else if has_virtual_edge {
        dynamic_accessor_body_for_actual(graph, class_index, base_index, base_ty, true)
    } else {
        static_ref_expr_for_path(graph, class_index, &view.path, quote! { self }, true)
    };
    let source_id = subobject_id(graph, class_index, &view.path);
    let oop_shared_body = shared_body.clone();
    let oop_mutable_body = mutable_body.clone();

    quote! {
        impl #impl_generics ::oop_mro::OopBase<#base_ty> for #class_ty #where_clause {
            fn __oop_as_base(&self) -> &#base_ty {
                #oop_shared_body
            }

            fn __oop_as_base_mut(&mut self) -> &mut #base_ty {
                #oop_mutable_body
            }
        }

        unsafe impl #impl_generics #trait_path for #class_ty #where_clause {
            fn __oop_as_self(&self) -> &#base_ty {
                #shared_body
            }

            fn __oop_as_self_mut(&mut self) -> &mut #base_ty {
                #mutable_body
            }

            fn __oop_complete_class_id(&self) -> usize {
                #class_index
            }

            fn __oop_source_subobject_id(&self) -> usize {
                #source_id
            }

            fn __oop_into_complete_owned(self: ::std::boxed::Box<Self>) -> *mut () {
                ::std::boxed::Box::into_raw(self) as *mut ()
            }
        }
    }
}

fn generate_concrete_base_via_impl(
    graph: &Graph,
    class_index: usize,
    view: &BaseViaView,
    class: &ClassDef,
) -> TokenStream2 {
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let class_ty = class_type_tokens(class);
    let via_ty = &view.via;
    let target_ty = &view.actual;
    let shared_body =
        static_ref_expr_for_path(graph, class_index, &view.path, quote! { self }, false);
    let mutable_body =
        static_ref_expr_for_path(graph, class_index, &view.path, quote! { self }, true);

    quote! {
        impl #impl_generics ::oop_mro::OopBaseVia<#via_ty, #target_ty> for #class_ty #where_clause {
            fn __oop_as_base_via(&self) -> &#target_ty {
                #shared_body
            }

            fn __oop_as_base_via_mut(&mut self) -> &mut #target_ty {
                #mutable_body
            }
        }
    }
}

fn generate_owned_base_via_items(graph: &Graph) -> TokenStream2 {
    let mut items = Vec::new();

    for (source_index, source_class) in graph.classes.iter().enumerate() {
        for (via_index, view) in base_via_views(graph, source_index).into_iter().enumerate() {
            items.push(generate_concrete_owned_base_via_items(
                graph,
                source_index,
                via_index,
                &view,
                source_class,
            ));
            items.push(generate_dyn_owned_base_via_items(
                graph,
                source_index,
                via_index,
                &view,
                source_class,
            ));
        }
    }

    quote! {
        #(#items)*
    }
}

fn generate_concrete_owned_base_via_items(
    graph: &Graph,
    source_index: usize,
    via_index: usize,
    view: &BaseViaView,
    source_class: &ClassDef,
) -> TokenStream2 {
    let wrapper =
        owned_base_via_wrapper_ident(graph, "Concrete", source_index, source_index, via_index);
    let wrapper_items = generate_owned_base_via_wrapper(
        graph,
        &wrapper,
        source_index,
        &view.path,
        view.class_index,
    );
    let mut generics = source_class.generics.clone();
    add_static_type_param_bounds(&mut generics);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, wrapper_ty_generics, _) = source_class.generics.split_for_impl();
    let source_ty = class_type_tokens(source_class);
    let wrapper_expr_generics = wrapper_ty_generics.as_turbofish();
    let via_ty = &view.via;
    let target_trait = as_class_trait_for_actual(&view.actual);

    quote! {
        #wrapper_items

        impl #impl_generics ::oop_mro::OopBoxBaseVia<#via_ty, dyn #target_trait>
            for #source_ty
            #where_clause
        {
            fn __oop_into_base_via(
                self: ::std::boxed::Box<Self>,
            ) -> ::std::boxed::Box<dyn #target_trait> {
                ::std::boxed::Box::new(#wrapper #wrapper_expr_generics { complete: self })
            }
        }
    }
}

fn generate_dyn_owned_base_via_items(
    graph: &Graph,
    source_index: usize,
    via_index: usize,
    view: &BaseViaView,
    source_class: &ClassDef,
) -> TokenStream2 {
    let candidates = owned_base_via_complete_candidates(graph, source_index, view);
    if candidates.is_empty() {
        return TokenStream2::new();
    }

    let mut generics = source_class.generics.clone();
    add_static_type_param_bounds(&mut generics);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let source_actual = class_type(source_class);
    let source_trait = as_class_trait_for_actual(&source_actual);
    let target_actual = &view.actual;
    let target_trait = as_class_trait_for_actual(&view.actual);
    let via_ty = &view.via;
    let mut wrapper_items = Vec::new();
    let arms = candidates
        .into_iter()
        .enumerate()
        .map(|(candidate_index, candidate)| {
            let wrapper = owned_base_via_dyn_wrapper_ident(
                graph,
                candidate.complete,
                source_index,
                via_index,
                candidate_index,
            );
            wrapper_items.push(generate_owned_base_via_wrapper(
                graph,
                &wrapper,
                candidate.complete,
                &candidate.path,
                view.class_index,
            ));
            let complete = candidate.complete;
            let source_id = candidate.source_id;
            let complete_ty = &candidate.complete_actual;
            let wrapper_ty = type_with_replaced_ident_expr_path(complete_ty, wrapper);
            quote! {
                (#complete, #source_id) => {
                    let raw =
                        <dyn #source_trait as #source_trait>::__oop_into_complete_owned(source);
                    let complete = unsafe {
                        ::std::boxed::Box::from_raw(raw as *mut #complete_ty)
                    };
                    let target: ::std::boxed::Box<dyn #target_trait> =
                        ::std::boxed::Box::new(#wrapper_ty { complete });
                    target
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#wrapper_items)*

        impl #impl_generics ::oop_mro::OopDynBoxBaseVia<#via_ty, #target_actual>
            for #source_actual
            #where_clause
        {
            fn __oop_dyn_into_base_via(
                source: ::std::boxed::Box<dyn #source_trait>,
            ) -> ::std::boxed::Box<dyn #target_trait> {
                let complete_id = <dyn #source_trait as #source_trait>::__oop_complete_class_id(&*source);
                let source_id = <dyn #source_trait as #source_trait>::__oop_source_subobject_id(&*source);
                match (complete_id, source_id) {
                    #(#arms,)*
                    _ => unreachable!("owned via cast candidate must match complete class id and source subobject id"),
                }
            }
        }
    }
}

struct OwnedBaseViaCandidate {
    complete: usize,
    source_id: usize,
    path: Vec<usize>,
    complete_actual: Type,
}

fn owned_base_via_complete_candidates(
    graph: &Graph,
    source_index: usize,
    view: &BaseViaView,
) -> Vec<OwnedBaseViaCandidate> {
    let source_actual = class_type(&graph.classes[source_index]);
    let mut candidates = Vec::new();

    for (complete, class) in graph.classes.iter().enumerate() {
        if class.is_abstract {
            continue;
        }
        if !graph.mros[complete].contains(&source_index) {
            continue;
        }

        for source_view in subobject_views(graph, complete)
            .into_iter()
            .filter(|source_view| source_view.class_index == source_index)
        {
            let source_path = source_view.path;
            let source_id = subobject_id(graph, complete, &source_path);
            let mut path = source_path;
            path.extend(view.path.clone());
            let target_actual = ancestor_type_for_path(graph, complete, &path);
            let Some(complete_actual) = actual_class_type_for_candidate(
                graph,
                complete,
                &[
                    (source_view.actual.clone(), source_actual.clone()),
                    (target_actual, view.actual.clone()),
                ],
            ) else {
                continue;
            };

            candidates.push(OwnedBaseViaCandidate {
                complete,
                source_id,
                path,
                complete_actual,
            });
        }
    }

    candidates
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

fn owned_base_via_wrapper_ident(
    graph: &Graph,
    kind: &str,
    complete_index: usize,
    source_index: usize,
    via_index: usize,
) -> Ident {
    let prefix = to_snake(&graph.names[0]);
    format_ident!(
        "__oop_OwnedVia_{}_{}_{}_{}_{}",
        prefix,
        kind,
        complete_index,
        source_index,
        via_index
    )
}

fn owned_base_via_dyn_wrapper_ident(
    graph: &Graph,
    complete_index: usize,
    source_index: usize,
    via_index: usize,
    candidate_index: usize,
) -> Ident {
    let prefix = to_snake(&graph.names[0]);
    format_ident!(
        "__oop_OwnedVia_{}_Dyn_{}_{}_{}_{}",
        prefix,
        complete_index,
        source_index,
        via_index,
        candidate_index
    )
}

fn generate_owned_base_via_wrapper(
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
    let trait_impls = ancestor_views(graph, target_index)
        .into_iter()
        .map(|trait_view| {
            let mut full_path = target_path.to_vec();
            full_path.extend(trait_view.path);
            let trait_actual = ancestor_type_for_path(graph, complete_index, &full_path);
            let trait_path = as_class_trait_for_actual(&trait_actual);
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

                unsafe impl #impl_generics #trait_path for #wrapper_ty #where_clause {
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
