use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, GenericArgument, GenericParam, Lifetime, PathArguments, ReturnType, Token, Type};

use crate::ast::ClassDef;
use crate::model::MethodInfo;
use crate::types::find_base_path_in;

#[derive(Default)]
pub(crate) struct GenericSubstitutions {
    types: HashMap<String, Type>,
    lifetimes: HashMap<String, Lifetime>,
    consts: HashMap<String, Expr>,
}

pub(crate) fn method_signature_key_in_context(
    classes: &[ClassDef],
    bases: &[Vec<usize>],
    mros: &[Vec<usize>],
    context: usize,
    method: &MethodInfo,
) -> String {
    let substitutions = substitutions_from_context(classes, bases, mros, context, method.owner);
    let arg_type_keys = method
        .arg_types
        .iter()
        .map(|ty| {
            substitute_type(ty, &substitutions)
                .to_token_stream()
                .to_string()
        })
        .collect::<Vec<_>>();
    let output = match &method.sig.output {
        ReturnType::Default => String::new(),
        ReturnType::Type(_, ty) => substitute_type(ty, &substitutions)
            .to_token_stream()
            .to_string(),
    };
    let unsafety = if method.sig.unsafety.is_some() {
        "unsafe"
    } else {
        "safe"
    };
    let asyncness = if method.sig.asyncness.is_some() {
        "async"
    } else {
        "sync"
    };

    format!(
        "{asyncness}|{unsafety}|{:?}|{}|{output}",
        method.receiver,
        arg_type_keys.join(",")
    )
}

pub(crate) fn substitutions_from_context(
    classes: &[ClassDef],
    bases: &[Vec<usize>],
    mros: &[Vec<usize>],
    start: usize,
    target: usize,
) -> GenericSubstitutions {
    if start == target {
        return GenericSubstitutions::default();
    }

    let Some(path) = find_base_path_in(bases, mros, start, target) else {
        return GenericSubstitutions::default();
    };

    let mut current = start;
    let mut substitutions = GenericSubstitutions::default();
    for next in path {
        let Some(base_spec) = classes[current]
            .bases
            .iter()
            .find(|base| base.name == classes[next].name)
        else {
            return GenericSubstitutions::default();
        };
        let actual_base = substitute_type(&base_spec.ty, &substitutions);
        substitutions = substitutions_for_class_type(&classes[next], &actual_base);
        current = next;
    }

    substitutions
}

pub(crate) fn substitutions_for_class_type(class: &ClassDef, ty: &Type) -> GenericSubstitutions {
    let mut substitutions = GenericSubstitutions::default();
    let Some(arguments) = class_type_arguments(ty) else {
        return substitutions;
    };

    for (param, argument) in class.generics.params.iter().zip(arguments) {
        match (param, argument) {
            (GenericParam::Type(param), GenericArgument::Type(ty)) => {
                substitutions
                    .types
                    .insert(param.ident.to_string(), ty.clone());
            }
            (GenericParam::Lifetime(param), GenericArgument::Lifetime(lifetime)) => {
                substitutions
                    .lifetimes
                    .insert(param.lifetime.ident.to_string(), lifetime.clone());
            }
            (GenericParam::Const(param), GenericArgument::Const(expr)) => {
                substitutions
                    .consts
                    .insert(param.ident.to_string(), expr.clone());
            }
            _ => {}
        }
    }

    substitutions
}

fn class_type_arguments(ty: &Type) -> Option<&Punctuated<GenericArgument, Token![,]>> {
    let Type::Path(ty_path) = ty else {
        return None;
    };
    if ty_path.qself.is_some() || ty_path.path.segments.len() != 1 {
        return None;
    }

    match &ty_path.path.segments[0].arguments {
        PathArguments::AngleBracketed(arguments) => Some(&arguments.args),
        PathArguments::None | PathArguments::Parenthesized(_) => None,
    }
}

pub(crate) fn substitute_type(ty: &Type, substitutions: &GenericSubstitutions) -> Type {
    let mut ty = ty.clone();
    let mut substituter = GenericSubstituter { substitutions };
    substituter.visit_type_mut(&mut ty);
    ty
}

pub(crate) fn substituted_return_type(
    output: &ReturnType,
    substitutions: &GenericSubstitutions,
) -> ReturnType {
    match output {
        ReturnType::Default => ReturnType::Default,
        ReturnType::Type(arrow, ty) => {
            ReturnType::Type(*arrow, Box::new(substitute_type(ty, substitutions)))
        }
    }
}

pub(crate) fn async_output_type_with_substitutions(
    sig: &syn::Signature,
    lifetime: &Lifetime,
    substitutions: &GenericSubstitutions,
) -> TokenStream2 {
    match &sig.output {
        ReturnType::Default => quote::quote! { () },
        ReturnType::Type(_, ty) => {
            let ty = substitute_type(ty, substitutions);
            crate::types::type_with_elided_refs_lifetime(&ty, lifetime).to_token_stream()
        }
    }
}

struct GenericSubstituter<'a> {
    substitutions: &'a GenericSubstitutions,
}

impl VisitMut for GenericSubstituter<'_> {
    fn visit_type_mut(&mut self, node: &mut Type) {
        if let Type::Path(path) = node {
            if path.qself.is_none() && path.path.segments.len() == 1 {
                let segment = &path.path.segments[0];
                if matches!(segment.arguments, PathArguments::None) {
                    if let Some(replacement) =
                        self.substitutions.types.get(&segment.ident.to_string())
                    {
                        *node = replacement.clone();
                        return;
                    }
                }
            }
        }

        visit_mut::visit_type_mut(self, node);
    }

    fn visit_lifetime_mut(&mut self, node: &mut Lifetime) {
        if let Some(replacement) = self.substitutions.lifetimes.get(&node.ident.to_string()) {
            *node = replacement.clone();
            return;
        }

        visit_mut::visit_lifetime_mut(self, node);
    }

    fn visit_expr_mut(&mut self, node: &mut Expr) {
        if let Expr::Path(path) = node {
            if path.qself.is_none() && path.path.segments.len() == 1 {
                let ident = path.path.segments[0].ident.to_string();
                if let Some(replacement) = self.substitutions.consts.get(&ident) {
                    *node = replacement.clone();
                    return;
                }
            }
        }

        visit_mut::visit_expr_mut(self, node);
    }
}
