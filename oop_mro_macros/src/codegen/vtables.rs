use super::*;

pub(super) fn generate_vtable_struct(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let vtable_name = vtable_ident(&graph.names[index]);
    let generics = &class.generics;
    let class_ty = class_type_tokens(class);
    let cast_ref = vtable_cast_ref_field_ident();
    let cast_mut = vtable_cast_mut_field_ident();
    let downcast_ref = vtable_downcast_ref_field_ident();
    let downcast_mut = vtable_downcast_mut_field_ident();
    let fields = interface_methods(graph, index).into_iter().map(|method| {
        let sig = signature_in_context(graph, index, &method);
        let field = vtable_field_ident(&method.name);
        let unsafety = &sig.unsafety;

        if sig.asyncness.is_some() {
            let lifetime = async_dispatch_lifetime();
            let receiver = match method.receiver {
                ReceiverKind::Shared => quote! { &#lifetime #class_ty },
                ReceiverKind::Mutable => quote! { &#lifetime mut #class_ty },
            };
            let arg_types = signature_arg_types(&sig)
                .iter()
                .map(|ty| type_with_elided_refs_lifetime(ty, &lifetime))
                .collect::<Vec<_>>();
            let output = async_output_type(&sig, &lifetime);
            let future = boxed_future_type(output, &lifetime);

            quote! {
                #field: for<#lifetime> #unsafety fn(#receiver #(, #arg_types)*) -> #future
            }
        } else {
            let receiver = match method.receiver {
                ReceiverKind::Shared => quote! { &#class_ty },
                ReceiverKind::Mutable => quote! { &mut #class_ty },
            };
            let arg_types = signature_arg_types(&sig);
            let output = &sig.output;

            quote! {
                #field: #unsafety fn(#receiver #(, #arg_types)*) #output
            }
        }
    });

    quote! {
        struct #vtable_name #generics {
            __oop_complete_class_id: usize,
            #cast_ref: unsafe fn(*const #class_ty, usize) -> ::core::option::Option<*const ()>,
            #cast_mut: unsafe fn(*mut #class_ty, usize) -> ::core::option::Option<*mut ()>,
            #downcast_ref: unsafe fn(*const #class_ty, usize) -> ::core::option::Option<*const ()>,
            #downcast_mut: unsafe fn(*mut #class_ty, usize) -> ::core::option::Option<*mut ()>,
            #(#fields,)*
        }
    }
}

pub(super) fn generate_vtable_items(graph: &Graph) -> TokenStream2 {
    let items = graph
        .classes
        .iter()
        .enumerate()
        .flat_map(|(class_index, _)| {
            vtable_slots(graph, class_index)
                .into_iter()
                .map(move |vtable_index| {
                    generate_vtable_for_class_as(graph, class_index, vtable_index)
                })
        });

    quote! {
        #(#items)*
    }
}

fn signature_arg_types(sig: &syn::Signature) -> Vec<Type> {
    sig.inputs
        .iter()
        .skip(1)
        .filter_map(|arg| match arg {
            syn::FnArg::Typed(typed) => Some((*typed.ty).clone()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect()
}

fn generate_vtable_for_class_as(
    graph: &Graph,
    class_index: usize,
    vtable_slot: VtableSlot,
) -> TokenStream2 {
    let vtable_index = vtable_slot.ancestor;
    let class = &graph.classes[class_index];
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();
    let turbofish = ty_generics.as_turbofish();
    let actual_vtable_class = ancestor_type_for_path(graph, class_index, &vtable_slot.path);
    let vtable_type = vtable_type_for_actual_class(graph, vtable_index, &actual_vtable_class);
    let vtable_constructor = vtable_ident(&graph.names[vtable_index]);
    let vtable_factory = vtable_factory_ident(graph, class_index, &vtable_slot);
    let cast_ref_field = vtable_cast_ref_field_ident();
    let cast_mut_field = vtable_cast_mut_field_ident();
    let downcast_ref_field = vtable_downcast_ref_field_ident();
    let downcast_mut_field = vtable_downcast_mut_field_ident();
    let cast_ref_function = vtable_cast_ref_function_ident(graph, class_index, &vtable_slot);
    let cast_mut_function = vtable_cast_mut_function_ident(graph, class_index, &vtable_slot);
    let downcast_ref_function =
        vtable_downcast_ref_function_ident(graph, class_index, &vtable_slot);
    let downcast_mut_function =
        vtable_downcast_mut_function_ident(graph, class_index, &vtable_slot);
    let entries = interface_methods(graph, vtable_index)
        .into_iter()
        .map(|method| {
            let field = vtable_field_ident(&method.name);
            let function = vtable_function_ident(graph, class_index, &vtable_slot, &method.name);
            quote! {
                #field: #function #turbofish
            }
        });
    let functions = interface_methods(graph, vtable_index)
        .into_iter()
        .map(|method| generate_vtable_function(graph, class_index, &vtable_slot, &method));
    let cast_ref = generate_vtable_cast_function(graph, class_index, &vtable_slot, false);
    let cast_mut = generate_vtable_cast_function(graph, class_index, &vtable_slot, true);
    let downcast_ref = generate_vtable_downcast_function(graph, class_index, &vtable_slot, false);
    let downcast_mut = generate_vtable_downcast_function(graph, class_index, &vtable_slot, true);

    quote! {
        fn #vtable_factory #impl_generics () -> #vtable_type #where_clause {
            #vtable_constructor {
                __oop_complete_class_id: #class_index,
                #cast_ref_field: #cast_ref_function #turbofish,
                #cast_mut_field: #cast_mut_function #turbofish,
                #downcast_ref_field: #downcast_ref_function #turbofish,
                #downcast_mut_field: #downcast_mut_function #turbofish,
                #(#entries,)*
            }
        }

        #cast_ref
        #cast_mut
        #downcast_ref
        #downcast_mut
        #(#functions)*
    }
}

fn generate_vtable_cast_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    mutable: bool,
) -> TokenStream2 {
    let class = &graph.classes[class_index];
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let function = if mutable {
        vtable_cast_mut_function_ident(graph, class_index, vtable_slot)
    } else {
        vtable_cast_ref_function_ident(graph, class_index, vtable_slot)
    };
    let receiver_ty = ancestor_type_for_path(graph, class_index, &vtable_slot.path);
    let complete = complete_from_receiver_expr(graph, class_index, &vtable_slot.path, mutable);
    let arms = vtable_cast_views(graph, class_index, vtable_slot)
        .into_iter()
        .map(|view| {
            let target = view.class_index;
            let target_id = cast_target_id(graph, target, &view.actual);
            let pointer = if target == class_index && view.path.is_empty() {
                if mutable {
                    quote! { complete as *mut _ as *mut () }
                } else {
                    quote! { complete as *const _ as *const () }
                }
            } else {
                let target_ref = static_ref_expr_for_path(
                    graph,
                    class_index,
                    &view.path,
                    quote! { complete },
                    mutable,
                );
                if mutable {
                    quote! { #target_ref as *mut _ as *mut () }
                } else {
                    quote! { #target_ref as *const _ as *const () }
                }
            };

            quote! {
                #target_id => ::core::option::Option::Some(#pointer)
            }
        });

    if mutable {
        quote! {
            unsafe fn #function #impl_generics (
                receiver: *mut #receiver_ty,
                target: usize,
            ) -> ::core::option::Option<*mut ()> #where_clause {
                let receiver = unsafe { &mut *receiver };
                let complete = #complete;
                match target {
                    #(#arms,)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    } else {
        quote! {
            unsafe fn #function #impl_generics (
                receiver: *const #receiver_ty,
                target: usize,
            ) -> ::core::option::Option<*const ()> #where_clause {
                let receiver = unsafe { &*receiver };
                let complete = #complete;
                match target {
                    #(#arms,)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    }
}

fn generate_vtable_downcast_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    mutable: bool,
) -> TokenStream2 {
    let class = &graph.classes[class_index];
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let function = if mutable {
        vtable_downcast_mut_function_ident(graph, class_index, vtable_slot)
    } else {
        vtable_downcast_ref_function_ident(graph, class_index, vtable_slot)
    };
    let receiver_ty = ancestor_type_for_path(graph, class_index, &vtable_slot.path);
    let complete = complete_from_receiver_expr(graph, class_index, &vtable_slot.path, mutable);
    let arms = downcast_target_views_for_source_path(graph, class_index, &vtable_slot.path)
        .into_iter()
        .map(|view| {
            let target = view.class_index;
            let target_id = cast_target_id(graph, target, &view.actual);
            let pointer = if target == class_index && view.path.is_empty() {
                if mutable {
                    quote! { complete as *mut _ as *mut () }
                } else {
                    quote! { complete as *const _ as *const () }
                }
            } else {
                let target_ref = static_ref_expr_for_path(
                    graph,
                    class_index,
                    &view.path,
                    quote! { complete },
                    mutable,
                );
                if mutable {
                    quote! { #target_ref as *mut _ as *mut () }
                } else {
                    quote! { #target_ref as *const _ as *const () }
                }
            };

            quote! {
                #target_id => ::core::option::Option::Some(#pointer)
            }
        });

    if mutable {
        quote! {
            unsafe fn #function #impl_generics (
                receiver: *mut #receiver_ty,
                target: usize,
            ) -> ::core::option::Option<*mut ()> #where_clause {
                let receiver = unsafe { &mut *receiver };
                let complete = #complete;
                match target {
                    #(#arms,)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    } else {
        quote! {
            unsafe fn #function #impl_generics (
                receiver: *const #receiver_ty,
                target: usize,
            ) -> ::core::option::Option<*const ()> #where_clause {
                let receiver = unsafe { &*receiver };
                let complete = #complete;
                match target {
                    #(#arms,)*
                    _ => ::core::option::Option::None,
                }
            }
        }
    }
}

fn generate_vtable_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    method: &MethodInfo,
) -> TokenStream2 {
    if method.sig.asyncness.is_some() {
        return generate_async_vtable_function(graph, class_index, vtable_slot, method);
    }

    let vtable_index = vtable_slot.ancestor;
    let class = &graph.classes[class_index];
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let function = vtable_function_ident(graph, class_index, vtable_slot, &method.name);
    let receiver_ty = ancestor_type_for_path(graph, class_index, &vtable_slot.path);
    let arg_idents = &method.arg_idents;
    let owner_path = selected_owner_path(graph, class_index, vtable_slot, method.owner);
    let owner_ty = ancestor_type_for_path(graph, class_index, &owner_path);
    let substitutions = substitutions_for_class_type(&graph.classes[method.owner], &owner_ty);
    let arg_types = method
        .arg_types
        .iter()
        .map(|ty| substitute_type(ty, &substitutions))
        .collect::<Vec<_>>();
    let output = substituted_return_type(&method.sig.output, &substitutions);
    let unsafety = &method.sig.unsafety;
    let method_name = method.name.to_string();
    let vtable_class_name = &graph.names[vtable_index];

    let selected = graph.selected_methods[class_index]
        .get(&method.name.to_string())
        .filter(|selected| {
            method_signature_key_in_context(
                &graph.classes,
                &graph.bases,
                &graph.mros,
                class_index,
                selected,
            ) == method_signature_key_in_context(
                &graph.classes,
                &graph.bases,
                &graph.mros,
                class_index,
                method,
            )
        });

    match method.receiver {
        ReceiverKind::Shared => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, false);
                let call = selected_virtual_impl_call(
                    graph,
                    class_index,
                    vtable_slot,
                    selected,
                    false,
                    arg_idents,
                    selected.sig.unsafety.is_some(),
                );
                quote! {
                    let complete = #complete;
                    #call
                }
            } else {
                quote! {
                    panic!("abstract virtual method `{}::{}` was called", #vtable_class_name, #method_name)
                }
            };

            quote! {
                #unsafety fn #function #impl_generics (
                    receiver: &#receiver_ty
                    #(, #arg_idents: #arg_types)*
                ) #output #where_clause {
                    #body
                }
            }
        }
        ReceiverKind::Mutable => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, true);
                let call = selected_virtual_impl_call(
                    graph,
                    class_index,
                    vtable_slot,
                    selected,
                    true,
                    arg_idents,
                    selected.sig.unsafety.is_some(),
                );
                quote! {
                    let complete = #complete;
                    #call
                }
            } else {
                quote! {
                    panic!("abstract virtual method `{}::{}` was called", #vtable_class_name, #method_name)
                }
            };

            quote! {
                #unsafety fn #function #impl_generics (
                    receiver: &mut #receiver_ty
                    #(, #arg_idents: #arg_types)*
                ) #output #where_clause {
                    #body
                }
            }
        }
    }
}

fn generate_async_vtable_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    method: &MethodInfo,
) -> TokenStream2 {
    let vtable_index = vtable_slot.ancestor;
    let class = &graph.classes[class_index];
    let function_generics = generics_with_async_lifetime(&class.generics);
    let (impl_generics, _, where_clause) = function_generics.split_for_impl();
    let function = vtable_function_ident(graph, class_index, vtable_slot, &method.name);
    let receiver_ty = ancestor_type_for_path(graph, class_index, &vtable_slot.path);
    let arg_idents = &method.arg_idents;
    let owner_path = selected_owner_path(graph, class_index, vtable_slot, method.owner);
    let owner_ty = ancestor_type_for_path(graph, class_index, &owner_path);
    let substitutions = substitutions_for_class_type(&graph.classes[method.owner], &owner_ty);
    let lifetime = async_dispatch_lifetime();
    let arg_types = method
        .arg_types
        .iter()
        .map(|ty| {
            let ty = substitute_type(ty, &substitutions);
            type_with_elided_refs_lifetime(&ty, &lifetime)
        })
        .collect::<Vec<_>>();
    let unsafety = &method.sig.unsafety;
    let method_name = method.name.to_string();
    let vtable_class_name = &graph.names[vtable_index];
    let output = async_output_type_with_substitutions(&method.sig, &lifetime, &substitutions);
    let future = boxed_future_type(output, &lifetime);

    let selected = graph.selected_methods[class_index]
        .get(&method.name.to_string())
        .filter(|selected| {
            method_signature_key_in_context(
                &graph.classes,
                &graph.bases,
                &graph.mros,
                class_index,
                selected,
            ) == method_signature_key_in_context(
                &graph.classes,
                &graph.bases,
                &graph.mros,
                class_index,
                method,
            )
        });

    match method.receiver {
        ReceiverKind::Shared => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, false);
                let call = selected_virtual_impl_call(
                    graph,
                    class_index,
                    vtable_slot,
                    selected,
                    false,
                    arg_idents,
                    selected.sig.unsafety.is_some(),
                );
                quote! {
                    ::std::boxed::Box::pin(async move {
                        let complete = #complete;
                        #call.await
                    })
                }
            } else {
                quote! {
                    ::std::boxed::Box::pin(async move {
                        panic!("abstract virtual method `{}::{}` was called", #vtable_class_name, #method_name)
                    })
                }
            };

            quote! {
                #unsafety fn #function #impl_generics (
                    receiver: &#lifetime #receiver_ty
                    #(, #arg_idents: #arg_types)*
                ) -> #future #where_clause {
                    #body
                }
            }
        }
        ReceiverKind::Mutable => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, true);
                let call = selected_virtual_impl_call(
                    graph,
                    class_index,
                    vtable_slot,
                    selected,
                    true,
                    arg_idents,
                    selected.sig.unsafety.is_some(),
                );
                quote! {
                    ::std::boxed::Box::pin(async move {
                        let complete = #complete;
                        #call.await
                    })
                }
            } else {
                quote! {
                    ::std::boxed::Box::pin(async move {
                        panic!("abstract virtual method `{}::{}` was called", #vtable_class_name, #method_name)
                    })
                }
            };

            quote! {
                #unsafety fn #function #impl_generics (
                    receiver: &#lifetime mut #receiver_ty
                    #(, #arg_idents: #arg_types)*
                ) -> #future #where_clause {
                    #body
                }
            }
        }
    }
}

fn complete_from_receiver_expr(
    graph: &Graph,
    class_index: usize,
    path: &[usize],
    mutable: bool,
) -> TokenStream2 {
    let class_ty = class_type_tokens(&graph.classes[class_index]);
    if path.is_empty() {
        return quote! { receiver };
    }

    let offset = offset_expr(graph, class_index, path);
    if mutable {
        quote! {
            unsafe {
                let offset = #offset;
                &mut *((receiver as *mut _ as *mut u8).sub(offset) as *mut #class_ty)
            }
        }
    } else {
        quote! {
            unsafe {
                let offset = #offset;
                &*((receiver as *const _ as *const u8).sub(offset) as *const #class_ty)
            }
        }
    }
}

fn selected_virtual_impl_call(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    selected: &MethodInfo,
    mutable: bool,
    arg_idents: &[Ident],
    wrap_unsafe: bool,
) -> TokenStream2 {
    let method = virtual_impl_ident(&selected.name);
    if selected.owner == class_index {
        let call = quote! {
            complete.#method(#(#arg_idents),*)
        };
        return if wrap_unsafe {
            quote! { unsafe { #call } }
        } else {
            call
        };
    }

    let owner_path = selected_owner_path(graph, class_index, vtable_slot, selected.owner);
    let owner_ref = static_ref_expr_for_path(
        graph,
        class_index,
        &owner_path,
        quote! { complete },
        mutable,
    );

    let call = quote! {
        (#owner_ref).#method(#(#arg_idents),*)
    };
    if wrap_unsafe {
        quote! { unsafe { #call } }
    } else {
        call
    }
}

fn selected_owner_path(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    owner: usize,
) -> Vec<usize> {
    let vtable_index = vtable_slot.ancestor;
    if owner == vtable_index {
        return vtable_slot.path.clone();
    }

    if graph.mros[vtable_index].contains(&owner) {
        let mut path = vtable_slot.path.clone();
        if let Some(owner_suffix) = find_base_path(graph, vtable_index, owner) {
            path.extend(owner_suffix);
            return path;
        }
    }

    find_base_path(graph, class_index, owner).unwrap_or_default()
}

fn offset_expr(graph: &Graph, class_index: usize, path: &[usize]) -> TokenStream2 {
    if path.is_empty() {
        return quote! { 0usize };
    }

    let class_ty = class_type_tokens(&graph.classes[class_index]);
    let mut statements = Vec::new();
    let mut current = class_index;
    let mut previous = format_ident!("__oop_offset_ptr_0");

    for (index, &base) in path.iter().enumerate() {
        let ident = format_ident!("__oop_offset_ptr_{}", index + 1);
        let base_ty = ancestor_type(graph, class_index, base);
        if edge_is_virtual(graph, current, base) {
            let field = virtual_base_field_ident(&graph.names[base]);
            statements.push(quote! {
                let #ident = ::core::ptr::addr_of!((*#previous).#field.__oop_value)
                    as *const #base_ty;
            });
        } else {
            let field = base_field_ident(&graph.names[base]);
            statements.push(quote! {
                let #ident = ::core::ptr::addr_of!((*#previous).#field)
                    as *const #base_ty;
            });
        }
        previous = ident;
        current = base;
    }

    quote! {
        {
            let uninit = ::core::mem::MaybeUninit::<#class_ty>::uninit();
            let __oop_offset_base = uninit.as_ptr();
            unsafe {
                let __oop_offset_ptr_0 = __oop_offset_base;
                #(#statements)*
                #previous as usize - __oop_offset_base as usize
            }
        }
    }
}
