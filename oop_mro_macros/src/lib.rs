use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{parse_macro_input, Error};

mod ast;
mod attrs;
mod c3;
mod codegen;
mod generics;
mod model;
mod names;
mod rewrite;
mod types;
mod validate;

#[proc_macro]
pub fn oop_class(input: TokenStream) -> TokenStream {
    let block = parse_macro_input!(input as ast::OopBlock);
    match expand(block) {
        Ok(tokens) => tokens.into(),
        Err(errors) => combine_errors(errors).into(),
    }
}

fn combine_errors(errors: Vec<Error>) -> TokenStream2 {
    errors
        .into_iter()
        .map(|error| error.to_compile_error())
        .collect()
}

fn expand(block: ast::OopBlock) -> Result<TokenStream2, Vec<Error>> {
    let mut errors = Vec::new();

    if block.classes.is_empty() {
        errors.push(Error::new(
            Span::call_site(),
            "oop_class! requires at least one class declaration",
        ));
        return Err(errors);
    }

    let mut graph = validate::validate_and_build(block, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    rewrite::rewrite_all_super_calls(&mut graph, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(codegen::generate(&graph))
}
