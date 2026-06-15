use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{ParseStream, Parser};
use syn::{bracketed, Attribute, Token};

pub(crate) mod kw {
    syn::custom_keyword!(class);
    syn::custom_keyword!(constructor);
}

pub(crate) struct ParsedAttrs {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) is_override: bool,
}

pub(crate) fn parse_oop_attrs(input: ParseStream<'_>) -> syn::Result<ParsedAttrs> {
    let mut attrs = Vec::new();
    let mut is_override = false;

    while input.peek(Token![#]) {
        input.parse::<Token![#]>()?;

        let content;
        bracketed!(content in input);
        let attr_tokens: TokenStream2 = content.parse()?;

        if attr_tokens.to_string() == "override" {
            is_override = true;
            continue;
        }

        let parsed_attrs = Attribute::parse_outer.parse2(quote! { #[#attr_tokens] })?;
        attrs.extend(parsed_attrs);
    }

    Ok(ParsedAttrs { attrs, is_override })
}
