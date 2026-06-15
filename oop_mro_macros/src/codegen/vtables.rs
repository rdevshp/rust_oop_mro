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
    let fields = interface_methods(graph, index).into_iter().map(|method| {
        let field = vtable_field_ident(&method.name);
        let unsafety = &method.sig.unsafety;

        if method.sig.asyncness.is_some() {
            let lifetime = async_dispatch_lifetime();
            let receiver = match method.receiver {
                ReceiverKind::Shared => quote! { &#lifetime #class_ty },
                ReceiverKind::Mutable => quote! { &#lifetime mut #class_ty },
            };
            let arg_types = method
                .arg_types
                .iter()
                .map(|ty| type_with_elided_refs_lifetime(ty, &lifetime))
                .collect::<Vec<_>>();
            let output = async_output_type(&method.sig, &lifetime);
            let future = boxed_future_type(output, &lifetime);

            quote! {
                #field: for<#lifetime> #unsafety fn(#receiver #(, #arg_types)*) -> #future
            }
        } else {
            let receiver = match method.receiver {
                ReceiverKind::Shared => quote! { &#class_ty },
                ReceiverKind::Mutable => quote! { &mut #class_ty },
            };
            let arg_types = &method.arg_types;
            let output = &method.sig.output;

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

fn generate_vtable_for_class_as(
    graph: &Graph,
    class_index: usize,
    vtable_slot: VtableSlot,
) -> TokenStream2 {
    let vtable_index = vtable_slot.ancestor;
    let class = &graph.classes[class_index];
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let actual_vtable_class = ancestor_type(graph, class_index, vtable_index);
    let vtable_type = vtable_type_for_actual_class(graph, vtable_index, &actual_vtable_class);
    let vtable_constructor = vtable_ident(&graph.names[vtable_index]);
    let vtable_factory = vtable_factory_ident(graph, class_index, &vtable_slot);
    let cast_ref_field = vtable_cast_ref_field_ident();
    let cast_mut_field = vtable_cast_mut_field_ident();
    let cast_ref_function = vtable_cast_ref_function_ident(graph, class_index, &vtable_slot);
    let cast_mut_function = vtable_cast_mut_function_ident(graph, class_index, &vtable_slot);
    let entries = interface_methods(graph, vtable_index)
        .into_iter()
        .map(|method| {
            let field = vtable_field_ident(&method.name);
            let function = vtable_function_ident(graph, class_index, &vtable_slot, &method.name);
            quote! {
                #field: #function
            }
        });
    let functions = interface_methods(graph, vtable_index)
        .into_iter()
        .map(|method| generate_vtable_function(graph, class_index, &vtable_slot, &method));
    let cast_ref = generate_vtable_cast_function(graph, class_index, &vtable_slot, false);
    let cast_mut = generate_vtable_cast_function(graph, class_index, &vtable_slot, true);

    quote! {
        fn #vtable_factory #impl_generics () -> #vtable_type #where_clause {
            #vtable_constructor {
                __oop_complete_class_id: #class_index,
                #cast_ref_field: #cast_ref_function,
                #cast_mut_field: #cast_mut_function,
                #(#entries,)*
            }
        }

        #cast_ref
        #cast_mut
        #(#functions)*
    }
}

fn generate_vtable_cast_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    mutable: bool,
) -> TokenStream2 {
    let vtable_index = vtable_slot.ancestor;
    let class = &graph.classes[class_index];
    let (impl_generics, _, where_clause) = class.generics.split_for_impl();
    let function = if mutable {
        vtable_cast_mut_function_ident(graph, class_index, vtable_slot)
    } else {
        vtable_cast_ref_function_ident(graph, class_index, vtable_slot)
    };
    let receiver_ty = ancestor_type(graph, class_index, vtable_index);
    let complete = complete_from_receiver_expr(graph, class_index, &vtable_slot.path, mutable);
    let arms = graph.mros[class_index].iter().copied().map(|target| {
        let pointer = if target == class_index {
            if mutable {
                quote! { complete as *mut _ as *mut () }
            } else {
                quote! { complete as *const _ as *const () }
            }
        } else {
            let accessor = if mutable {
                format_ident!("__oop_as_mut_{}", graph.names[target])
            } else {
                format_ident!("__oop_as_{}", graph.names[target])
            };
            if mutable {
                quote! { complete.#accessor() as *mut _ as *mut () }
            } else {
                quote! { complete.#accessor() as *const _ as *const () }
            }
        };

        quote! {
            #target => ::core::option::Option::Some(#pointer)
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
    let receiver_ty = ancestor_type(graph, class_index, vtable_index);
    let arg_idents = &method.arg_idents;
    let substitutions = substitutions_from_context(
        &graph.classes,
        &graph.bases,
        &graph.mros,
        class_index,
        vtable_index,
    );
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
    let receiver_ty = ancestor_type(graph, class_index, vtable_index);
    let arg_idents = &method.arg_idents;
    let substitutions = substitutions_from_context(
        &graph.classes,
        &graph.bases,
        &graph.mros,
        class_index,
        vtable_index,
    );
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

    let owner_name = &graph.names[selected.owner];
    let accessor = if mutable {
        format_ident!("__oop_as_mut_{}", owner_name)
    } else {
        format_ident!("__oop_as_{}", owner_name)
    };

    let call = quote! {
        complete.#accessor().#method(#(#arg_idents),*)
    };
    if wrap_unsafe {
        quote! { unsafe { #call } }
    } else {
        call
    }
}

fn offset_expr(graph: &Graph, class_index: usize, path: &[usize]) -> TokenStream2 {
    if path.is_empty() {
        return quote! { 0usize };
    }

    let class_ty = class_type_tokens(&graph.classes[class_index]);
    let mut field_tokens = TokenStream2::new();
    for &base in path {
        let field = base_field_ident(&graph.names[base]);
        field_tokens.extend(quote! { .#field });
    }

    quote! {
        {
            let uninit = ::core::mem::MaybeUninit::<#class_ty>::uninit();
            let base = uninit.as_ptr();
            unsafe {
                let field = ::core::ptr::addr_of!((*base)#field_tokens);
                field as usize - base as usize
            }
        }
    }
}
