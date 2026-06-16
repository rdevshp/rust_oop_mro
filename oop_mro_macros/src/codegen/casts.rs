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
        .filter_map(|base| {
            let base_index = graph.name_to_index[&base.name.to_string()];
            if base_supertrait_is_unambiguous_for_all_impls(graph, index, base_index) {
                Some(base_cast_trait_for_actual_class(
                    graph, base_index, &base.ty,
                ))
            } else {
                None
            }
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
            fn __oop_source_subobject_id(&self) -> usize;

            #[doc(hidden)]
            fn __oop_into_complete_owned(self: ::std::boxed::Box<Self>) -> *mut ();

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
                .flat_map(move |view| {
                    [
                        generate_concrete_base_via_impl(graph, class_index, &view, class),
                        generate_dyn_base_via_impl(graph, class_index, &view, class),
                    ]
                })
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
    let trait_path = base_cast_trait_for_actual_class(graph, base_index, base_ty);
    let shared_name = base_cast_method_ident(&graph.names[base_index], false);
    let mutable_name = base_cast_method_ident(&graph.names[base_index], true);
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
    let private_module = private_module_ident(graph);
    let source_id = subobject_id(graph, class_index, &view.path);

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

            fn __oop_source_subobject_id(&self) -> usize {
                #source_id
            }

            fn __oop_into_complete_owned(self: ::std::boxed::Box<Self>) -> *mut () {
                ::std::boxed::Box::into_raw(self) as *mut ()
            }

            fn __oop_cast_seal(&self) -> #private_module::Seal {
                #private_module::Seal
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

fn generate_dyn_base_via_impl(
    graph: &Graph,
    class_index: usize,
    view: &BaseViaView,
    class: &ClassDef,
) -> TokenStream2 {
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let class_ty = class_type_tokens(class);
    let class_actual = class_type(class);
    let trait_path = base_cast_trait_for_actual_class(graph, class_index, &class_actual);
    let shared_name = base_cast_method_ident(&graph.names[class_index], false);
    let mutable_name = base_cast_method_ident(&graph.names[class_index], true);
    let via_ty = &view.via;
    let target_ty = &view.actual;

    quote! {
        impl #impl_generics ::oop_mro::OopBaseVia<#via_ty, #target_ty> for dyn #trait_path + '_ #where_clause {
            fn __oop_as_base_via(&self) -> &#target_ty {
                <#class_ty as ::oop_mro::OopBaseVia<#via_ty, #target_ty>>::__oop_as_base_via(
                    self.#shared_name(),
                )
            }

            fn __oop_as_base_via_mut(&mut self) -> &mut #target_ty {
                <#class_ty as ::oop_mro::OopBaseVia<#via_ty, #target_ty>>::__oop_as_base_via_mut(
                    self.#mutable_name(),
                )
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
    let wrapper = owned_base_via_wrapper_ident("Concrete", source_index, source_index, via_index);
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
    let target_trait = base_cast_trait_for_actual_class(graph, view.class_index, &view.actual);

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
    let source_trait = base_cast_trait_for_actual_class(graph, source_index, &source_actual);
    let target_trait = base_cast_trait_for_actual_class(graph, view.class_index, &view.actual);
    let via_ty = &view.via;
    let mut wrapper_items = Vec::new();
    let arms = candidates
        .into_iter()
        .enumerate()
        .map(|(candidate_index, candidate)| {
            let wrapper = owned_base_via_dyn_wrapper_ident(
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
            let complete_ty = class_type_tokens(&graph.classes[complete]);
            let (_, wrapper_ty_generics, _) = graph.classes[complete].generics.split_for_impl();
            let wrapper_expr_generics = wrapper_ty_generics.as_turbofish();
            quote! {
                (#complete, #source_id) => {
                    let raw =
                        <dyn #source_trait as #source_trait>::__oop_into_complete_owned(self);
                    let complete = unsafe {
                        ::std::boxed::Box::from_raw(raw as *mut #complete_ty)
                    };
                    let target: ::std::boxed::Box<dyn #target_trait> =
                        ::std::boxed::Box::new(#wrapper #wrapper_expr_generics { complete });
                    target
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #(#wrapper_items)*

        impl #impl_generics ::oop_mro::OopBoxBaseVia<#via_ty, dyn #target_trait>
            for dyn #source_trait
            #where_clause
        {
            fn __oop_into_base_via(
                self: ::std::boxed::Box<Self>,
            ) -> ::std::boxed::Box<dyn #target_trait> {
                let complete_id = <dyn #source_trait as #source_trait>::__oop_complete_class_id(&*self);
                let source_id = <dyn #source_trait as #source_trait>::__oop_source_subobject_id(&*self);
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
}

fn owned_base_via_complete_candidates(
    graph: &Graph,
    source_index: usize,
    view: &BaseViaView,
) -> Vec<OwnedBaseViaCandidate> {
    let source_actual = class_type(&graph.classes[source_index]);
    let target_key = type_key(&view.actual);
    let mut candidates = Vec::new();

    for (complete, class) in graph.classes.iter().enumerate() {
        if class.is_abstract {
            continue;
        }
        if !graph.mros[complete].contains(&source_index) {
            continue;
        }
        if !compatible_owned_cast_generics(graph, source_index, complete) {
            continue;
        }

        for source_view in subobject_views(graph, complete)
            .into_iter()
            .filter(|source_view| {
                source_view.class_index == source_index
                    && type_key(&source_view.actual) == type_key(&source_actual)
            })
        {
            let source_path = source_view.path;
            let source_id = subobject_id(graph, complete, &source_path);
            let mut path = source_path;
            path.extend(view.path.clone());
            let target_actual = ancestor_type_for_path(graph, complete, &path);
            if type_key(&target_actual) != target_key {
                continue;
            }

            candidates.push(OwnedBaseViaCandidate {
                complete,
                source_id,
                path,
            });
        }
    }

    candidates
}

fn compatible_owned_cast_generics(graph: &Graph, source: usize, complete: usize) -> bool {
    if source == complete {
        return true;
    }

    if graph.classes[source].generics.params.is_empty()
        && graph.classes[complete].generics.params.is_empty()
    {
        return true;
    }

    generic_params_key(&graph.classes[source].generics)
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
    kind: &str,
    complete_index: usize,
    source_index: usize,
    via_index: usize,
) -> Ident {
    format_ident!(
        "__oop_OwnedVia_{}_{}_{}_{}",
        kind,
        complete_index,
        source_index,
        via_index
    )
}

fn owned_base_via_dyn_wrapper_ident(
    complete_index: usize,
    source_index: usize,
    via_index: usize,
    candidate_index: usize,
) -> Ident {
    format_ident!(
        "__oop_OwnedVia_Dyn_{}_{}_{}_{}",
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
    let private_module = private_module_ident(graph);
    let trait_impls = ancestor_views(graph, target_index)
        .into_iter()
        .map(|trait_view| {
            let trait_index = trait_view.class_index;
            let mut full_path = target_path.to_vec();
            full_path.extend(trait_view.path);
            let trait_actual = ancestor_type_for_path(graph, complete_index, &full_path);
            let trait_path = base_cast_trait_for_actual_class(graph, trait_index, &trait_actual);
            let shared_name = base_cast_method_ident(&graph.names[trait_index], false);
            let mutable_name = base_cast_method_ident(&graph.names[trait_index], true);
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

            quote! {
                impl #impl_generics #trait_path for #wrapper_ty #where_clause {
                    fn #shared_name(&self) -> &#trait_actual {
                        #shared_body
                    }

                    fn #mutable_name(&mut self) -> &mut #trait_actual {
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
