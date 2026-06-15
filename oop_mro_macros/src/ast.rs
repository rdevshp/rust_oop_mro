use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    braced, parenthesized, Attribute, Block, Error, Expr, FnArg, Generics, Ident, Signature, Token,
    Type, TypePath, Visibility,
};

use crate::attrs::{kw, parse_oop_attrs};

#[derive(Debug)]
pub(crate) struct OopBlock {
    pub(crate) classes: Vec<ClassDef>,
}

impl Parse for OopBlock {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut classes = Vec::new();
        while !input.is_empty() {
            classes.push(input.parse()?);
        }
        Ok(Self { classes })
    }
}

#[derive(Debug)]
pub(crate) struct ClassDef {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) is_override: bool,
    pub(crate) vis: Visibility,
    pub(crate) is_abstract: bool,
    pub(crate) name: Ident,
    pub(crate) generics: Generics,
    pub(crate) bases: Vec<BaseSpec>,
    pub(crate) items: Vec<ClassItem>,
}

impl Parse for ClassDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let is_abstract = if input.peek(Token![abstract]) {
            input.parse::<Token![abstract]>()?;
            true
        } else {
            false
        };
        input.parse::<kw::class>()?;
        let name: Ident = input.parse()?;
        let mut generics: Generics = input.parse()?;
        let bases = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let bases = Punctuated::<BaseSpec, Token![,]>::parse_separated_nonempty(input)?;
            bases.into_iter().collect()
        } else {
            Vec::new()
        };
        if generics.where_clause.is_none() && input.peek(Token![where]) {
            generics.where_clause = Some(input.parse()?);
        }

        let content;
        braced!(content in input);
        let mut items = Vec::new();
        while !content.is_empty() {
            items.push(content.parse()?);
        }

        Ok(Self {
            attrs: parsed_attrs.attrs,
            is_override: parsed_attrs.is_override,
            vis,
            is_abstract,
            name,
            generics,
            bases,
            items,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BaseSpec {
    pub(crate) name: Ident,
    pub(crate) ty: Type,
}

impl Parse for BaseSpec {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ty_path: TypePath = input.parse()?;

        if ty_path.qself.is_some() || ty_path.path.segments.len() != 1 {
            return Err(Error::new_spanned(
                ty_path,
                "base classes must be declared as `Base` or `Base<...>`",
            ));
        }

        let name = ty_path.path.segments[0].ident.clone();
        Ok(Self {
            name,
            ty: Type::Path(ty_path),
        })
    }
}

#[derive(Debug)]
pub(crate) enum ClassItem {
    Field(FieldDef),
    Method(MethodDef),
    Constructor(ConstructorDef),
    AssociatedConst(AssociatedConstDef),
    StaticField(StaticFieldDef),
    UnsupportedAssociatedType(AssociatedTypeDef),
}

impl Parse for ClassItem {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let fork = input.fork();
        parse_oop_attrs(&fork)?;
        let _: Visibility = fork.parse()?;

        if fork.peek(kw::constructor) {
            return input.parse().map(Self::Constructor);
        }
        if fork.peek(Token![abstract]) {
            fork.parse::<Token![abstract]>()?;
        }
        if fork.peek(Token![virtual]) {
            fork.parse::<Token![virtual]>()?;
        }

        if fork.peek(Token![type]) {
            return input.parse().map(Self::UnsupportedAssociatedType);
        }

        if fork.peek(Token![static]) {
            return input.parse().map(Self::StaticField);
        }

        if fork.peek(Token![const]) {
            let after_const = fork.fork();
            after_const.parse::<Token![const]>()?;
            if !(after_const.peek(Token![fn])
                || after_const.peek(Token![async])
                || after_const.peek(Token![unsafe])
                || after_const.peek(Token![extern]))
            {
                return input.parse().map(Self::AssociatedConst);
            }
        }

        if fork.peek(Token![fn])
            || fork.peek(Token![async])
            || fork.peek(Token![const])
            || fork.peek(Token![unsafe])
            || fork.peek(Token![extern])
        {
            return input.parse().map(Self::Method);
        }

        input.parse().map(Self::Field)
    }
}

#[derive(Debug)]
pub(crate) struct AssociatedConstDef {
    pub(crate) item: syn::ImplItemConst,
    pub(crate) is_override: bool,
}

impl Parse for AssociatedConstDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let const_token = input.parse::<Token![const]>()?;
        let ident: Ident = input.parse()?;
        let generics: Generics = input.parse()?;
        let colon_token = input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        let eq_token = input.parse::<Token![=]>()?;
        let expr: Expr = input.parse()?;
        let semi_token = input.parse::<Token![;]>()?;

        Ok(Self {
            item: syn::ImplItemConst {
                attrs: parsed_attrs.attrs,
                vis,
                defaultness: None,
                const_token,
                ident,
                generics,
                colon_token,
                ty,
                eq_token,
                expr,
                semi_token,
            },
            is_override: parsed_attrs.is_override,
        })
    }
}

impl ToTokens for AssociatedConstDef {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.item.to_tokens(tokens);
    }
}

#[derive(Debug)]
pub(crate) struct StaticFieldDef {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) is_override: bool,
    pub(crate) vis: Visibility,
    pub(crate) static_token: Token![static],
    pub(crate) mutability: Option<Token![mut]>,
    pub(crate) ident: Ident,
    pub(crate) colon_token: Token![:],
    pub(crate) ty: Type,
    pub(crate) eq_token: Token![=],
    pub(crate) expr: Expr,
    pub(crate) semi_token: Token![;],
}

impl Parse for StaticFieldDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let static_token = input.parse::<Token![static]>()?;
        let mutability = if input.peek(Token![mut]) {
            Some(input.parse::<Token![mut]>()?)
        } else {
            None
        };
        let ident: Ident = input.parse()?;
        let colon_token = input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        let eq_token = input.parse::<Token![=]>()?;
        let expr: Expr = input.parse()?;
        let semi_token = input.parse::<Token![;]>()?;

        Ok(Self {
            attrs: parsed_attrs.attrs,
            is_override: parsed_attrs.is_override,
            vis,
            static_token,
            mutability,
            ident,
            colon_token,
            ty,
            eq_token,
            expr,
            semi_token,
        })
    }
}

#[derive(Debug)]
pub(crate) struct AssociatedTypeDef {
    pub(crate) item: syn::ImplItemType,
    pub(crate) is_override: bool,
}

impl Parse for AssociatedTypeDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let mut item: syn::ImplItemType = input.parse()?;
        item.attrs = parsed_attrs.attrs;

        Ok(Self {
            item,
            is_override: parsed_attrs.is_override,
        })
    }
}

#[derive(Debug)]
pub(crate) struct ConstructorDef {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) is_override: bool,
    pub(crate) vis: Visibility,
    pub(crate) constructor_token: kw::constructor,
    pub(crate) inputs: Vec<FnArg>,
    pub(crate) base_calls: Vec<ConstructorBaseCall>,
    pub(crate) body: Block,
}

#[derive(Debug)]
pub(crate) struct ConstructorBaseCall {
    pub(crate) base: Ident,
    pub(crate) args: Vec<Expr>,
}

impl Parse for ConstructorDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let constructor_token = input.parse::<kw::constructor>()?;

        let content;
        parenthesized!(content in input);
        let inputs = Punctuated::<FnArg, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();

        let base_calls = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            Punctuated::<ConstructorBaseCall, Token![,]>::parse_separated_nonempty(input)?
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            attrs: parsed_attrs.attrs,
            is_override: parsed_attrs.is_override,
            vis,
            constructor_token,
            inputs,
            base_calls,
            body: input.parse()?,
        })
    }
}

impl Parse for ConstructorBaseCall {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let base: Ident = input.parse()?;

        let content;
        parenthesized!(content in input);
        let args = Punctuated::<Expr, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();

        Ok(Self { base, args })
    }
}

#[derive(Debug)]
pub(crate) struct FieldDef {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) is_override: bool,
    pub(crate) vis: Visibility,
    pub(crate) ident: Ident,
    pub(crate) colon_token: Token![:],
    pub(crate) ty: Type,
}

impl Parse for FieldDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let ident: Ident = input.parse()?;
        let colon_token: Token![:] = input.parse()?;
        let ty: Type = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
        }

        Ok(Self {
            attrs: parsed_attrs.attrs,
            is_override: parsed_attrs.is_override,
            vis,
            ident,
            colon_token,
            ty,
        })
    }
}

impl ToTokens for FieldDef {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let attrs = &self.attrs;
        let vis = &self.vis;
        let ident = &self.ident;
        let colon = &self.colon_token;
        let ty = &self.ty;
        tokens.extend(quote! {
            #(#attrs)*
            #vis #ident #colon #ty
        });
    }
}

#[derive(Debug)]
pub(crate) struct MethodDef {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) is_abstract_decl: bool,
    pub(crate) is_virtual: bool,
    pub(crate) is_override: bool,
    pub(crate) sig: Signature,
    pub(crate) body: Option<Block>,
}

impl Parse for MethodDef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parsed_attrs = parse_oop_attrs(input)?;
        let vis: Visibility = input.parse()?;
        let is_abstract_decl = if input.peek(Token![abstract]) {
            input.parse::<Token![abstract]>()?;
            true
        } else {
            false
        };
        let is_virtual = if input.peek(Token![virtual]) {
            input.parse::<Token![virtual]>()?;
            true
        } else {
            false
        };
        let sig: Signature = input.parse()?;
        let body = if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
            None
        } else {
            Some(input.parse()?)
        };

        Ok(Self {
            attrs: parsed_attrs.attrs,
            vis,
            is_abstract_decl,
            is_virtual,
            is_override: parsed_attrs.is_override,
            sig,
            body,
        })
    }
}
