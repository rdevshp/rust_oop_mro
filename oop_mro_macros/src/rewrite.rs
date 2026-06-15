use std::collections::HashMap;

use proc_macro2::{Group, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, Error, Expr, ExprMacro, Ident, Token};

use crate::ast::ClassItem;
use crate::model::{Graph, MethodMap, ReceiverKind};
use crate::names::virtual_impl_ident;

pub(crate) fn rewrite_all_super_calls(graph: &mut Graph, errors: &mut Vec<Error>) {
    let selected_methods = graph.selected_methods.clone();
    let mros = graph.mros.clone();
    let name_to_index = graph.name_to_index.clone();
    let names = graph.names.clone();

    for (class_index, class) in graph.classes.iter_mut().enumerate() {
        for item in &mut class.items {
            match item {
                ClassItem::Method(method) => {
                    let Some(body) = &mut method.body else {
                        continue;
                    };

                    let current_method = match selected_methods[class_index]
                        .get(&method.sig.ident.to_string())
                        .cloned()
                    {
                        Some(info) => info,
                        None => continue,
                    };

                    let mut rewriter = SuperCallRewriter {
                        current_class: class_index,
                        current_method_receiver: current_method.receiver,
                        current_class_name: names[class_index].clone(),
                        names: &names,
                        name_to_index: &name_to_index,
                        mros: &mros,
                        selected_methods: &selected_methods,
                        errors: Vec::new(),
                    };
                    rewriter.visit_block_mut(body);
                    errors.extend(rewriter.errors);
                }
                ClassItem::Constructor(constructor) => {
                    let mut rewriter = SuperCallRewriter {
                        current_class: class_index,
                        current_method_receiver: ReceiverKind::Mutable,
                        current_class_name: names[class_index].clone(),
                        names: &names,
                        name_to_index: &name_to_index,
                        mros: &mros,
                        selected_methods: &selected_methods,
                        errors: Vec::new(),
                    };
                    rewriter.visit_block_mut(&mut constructor.body);
                    errors.extend(rewriter.errors);
                }
                ClassItem::Field(_)
                | ClassItem::AssociatedConst(_)
                | ClassItem::StaticField(_)
                | ClassItem::UnsupportedAssociatedType(_) => {}
            }
        }
    }
}

struct SuperCallRewriter<'a> {
    current_class: usize,
    current_method_receiver: ReceiverKind,
    current_class_name: String,
    names: &'a [String],
    name_to_index: &'a HashMap<String, usize>,
    mros: &'a [Vec<usize>],
    selected_methods: &'a [MethodMap],
    errors: Vec<Error>,
}

impl VisitMut for SuperCallRewriter<'_> {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        if let Expr::Macro(expr_macro) = node {
            if expr_macro.mac.path.is_ident("super_call") {
                match self.rewrite_super_call(expr_macro) {
                    Ok(expr) => {
                        *node = expr;
                    }
                    Err(error) => self.errors.push(error),
                }
                return;
            }

            expr_macro.mac.tokens = self.rewrite_token_stream(expr_macro.mac.tokens.clone());
            return;
        }

        visit_mut::visit_expr_mut(self, node);
    }
}

impl SuperCallRewriter<'_> {
    fn rewrite_super_call(&self, expr_macro: &ExprMacro) -> syn::Result<Expr> {
        let input: SuperCallInput = syn::parse2(expr_macro.mac.tokens.clone())?;
        self.rewrite_super_call_input(input)
    }

    fn rewrite_super_call_input(&self, input: SuperCallInput) -> syn::Result<Expr> {
        let target_name = input.class.to_string();
        let method_name = input.method.to_string();

        let Some(&target_index) = self.name_to_index.get(&target_name) else {
            return Err(Error::new_spanned(
                input.class,
                format!("unknown super_call! target class `{target_name}`"),
            ));
        };

        if target_index == self.current_class {
            return Err(Error::new_spanned(
                input.class,
                "super_call! target must be an inherited class, not the current class",
            ));
        }

        if !self.mros[self.current_class].contains(&target_index) {
            return Err(Error::new_spanned(
                input.class,
                format!(
                    "class `{}` is not in the MRO of `{}`",
                    target_name, self.current_class_name
                ),
            ));
        }

        let Some(target_method) = self.selected_methods[target_index].get(&method_name) else {
            return Err(Error::new_spanned(
                input.method,
                format!("class `{target_name}` has no method `{method_name}`"),
            ));
        };

        if target_method.receiver == ReceiverKind::Mutable
            && self.current_method_receiver == ReceiverKind::Shared
        {
            return Err(Error::new_spanned(
                input.method,
                "cannot call a `&mut self` super method from a `&self` method",
            ));
        }

        if !matches!(input.receiver, Expr::Path(ref path) if path.path.is_ident("self")) {
            return Err(Error::new_spanned(
                input.receiver,
                "super_call! currently supports only `self` as the receiver expression",
            ));
        }

        let owner_name = &self.names[target_method.owner];
        let accessor = match target_method.receiver {
            ReceiverKind::Shared => format_ident!("__oop_as_{}", owner_name),
            ReceiverKind::Mutable => format_ident!("__oop_as_mut_{}", owner_name),
        };
        let method = virtual_impl_ident(&input.method);
        let args = input.args;

        Ok(parse_quote! {
            self.#accessor().#method(#(#args),*)
        })
    }

    fn rewrite_token_stream(&mut self, tokens: TokenStream2) -> TokenStream2 {
        let mut output = TokenStream2::new();
        let mut iter = tokens.into_iter().peekable();

        while let Some(token) = iter.next() {
            match token {
                TokenTree::Ident(ident) if ident == "super_call" => {
                    let Some(TokenTree::Punct(punct)) = iter.peek() else {
                        output.extend([TokenTree::Ident(ident)]);
                        continue;
                    };

                    if punct.as_char() != '!' {
                        output.extend([TokenTree::Ident(ident)]);
                        continue;
                    }

                    let bang = iter.next().expect("peeked token must exist");
                    let Some(TokenTree::Group(group)) = iter.next() else {
                        output.extend([TokenTree::Ident(ident), bang]);
                        continue;
                    };

                    match syn::parse2::<SuperCallInput>(group.stream()) {
                        Ok(input) => match self.rewrite_super_call_input(input) {
                            Ok(expr) => output.extend(expr.to_token_stream()),
                            Err(error) => {
                                self.errors.push(error);
                                output.extend([
                                    TokenTree::Ident(ident),
                                    bang,
                                    TokenTree::Group(group),
                                ]);
                            }
                        },
                        Err(error) => {
                            self.errors.push(error);
                            output.extend([TokenTree::Ident(ident), bang, TokenTree::Group(group)]);
                        }
                    }
                }
                TokenTree::Group(group) => {
                    let stream = self.rewrite_token_stream(group.stream());
                    let mut rewritten = Group::new(group.delimiter(), stream);
                    rewritten.set_span(group.span());
                    output.extend([TokenTree::Group(rewritten)]);
                }
                other => output.extend([other]),
            }
        }

        output
    }
}

struct SuperCallInput {
    class: Ident,
    method: Ident,
    receiver: Expr,
    args: Vec<Expr>,
}

impl Parse for SuperCallInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let class: Ident = input.parse()?;
        input.parse::<Token![::]>()?;
        let method: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let receiver: Expr = input.parse()?;
        let mut args = Vec::new();

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            args.push(input.parse()?);
        }

        Ok(Self {
            class,
            method,
            receiver,
            args,
        })
    }
}
