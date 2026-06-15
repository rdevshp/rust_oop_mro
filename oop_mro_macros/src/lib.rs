use proc_macro::TokenStream;
use proc_macro2::{Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{
    braced, bracketed, parenthesized, parse_macro_input, parse_quote, Attribute, Block, Error,
    Expr, ExprMacro, FnArg, Ident, Pat, Receiver, ReturnType, Signature, Token, Type, Visibility,
};

mod c3;

mod kw {
    syn::custom_keyword!(class);
    syn::custom_keyword!(constructor);
}

#[proc_macro]
pub fn oop_class(input: TokenStream) -> TokenStream {
    let block = parse_macro_input!(input as OopBlock);
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

#[derive(Debug)]
struct OopBlock {
    classes: Vec<ClassDef>,
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
struct ClassDef {
    attrs: Vec<Attribute>,
    is_override: bool,
    vis: Visibility,
    is_abstract: bool,
    name: Ident,
    bases: Vec<Ident>,
    items: Vec<ClassItem>,
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
        let bases = if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let bases = Punctuated::<Ident, Token![,]>::parse_separated_nonempty(input)?;
            bases.into_iter().collect()
        } else {
            Vec::new()
        };

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
            bases,
            items,
        })
    }
}

#[derive(Debug)]
enum ClassItem {
    Field(FieldDef),
    Method(MethodDef),
    Constructor(ConstructorDef),
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
struct ConstructorDef {
    attrs: Vec<Attribute>,
    is_override: bool,
    vis: Visibility,
    constructor_token: kw::constructor,
    inputs: Vec<FnArg>,
    base_calls: Vec<ConstructorBaseCall>,
    body: Block,
}

#[derive(Debug)]
struct ConstructorBaseCall {
    base: Ident,
    args: Vec<Expr>,
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
struct FieldDef {
    attrs: Vec<Attribute>,
    is_override: bool,
    vis: Visibility,
    ident: Ident,
    colon_token: Token![:],
    ty: Type,
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

struct ParsedAttrs {
    attrs: Vec<Attribute>,
    is_override: bool,
}

fn parse_oop_attrs(input: ParseStream<'_>) -> syn::Result<ParsedAttrs> {
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

#[derive(Debug)]
struct MethodDef {
    attrs: Vec<Attribute>,
    vis: Visibility,
    is_abstract_decl: bool,
    is_virtual: bool,
    is_override: bool,
    sig: Signature,
    body: Option<Block>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiverKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone)]
struct MethodInfo {
    owner: usize,
    name: Ident,
    vis: Visibility,
    sig: Signature,
    is_virtual: bool,
    is_abstract: bool,
    is_override: bool,
    receiver: ReceiverKind,
    arg_idents: Vec<Ident>,
    arg_types: Vec<Type>,
    signature_key: String,
    signature_display: String,
}

#[derive(Debug)]
struct Graph {
    classes: Vec<ClassDef>,
    names: Vec<String>,
    name_to_index: HashMap<String, usize>,
    bases: Vec<Vec<usize>>,
    mros: Vec<Vec<usize>>,
    selected_methods: Vec<BTreeMap<String, MethodInfo>>,
    abstract_methods: Vec<BTreeMap<String, MethodInfo>>,
}

#[derive(Debug, Clone)]
struct VtableSlot {
    ancestor: usize,
    path: Vec<usize>,
}

fn expand(block: OopBlock) -> Result<TokenStream2, Vec<Error>> {
    let mut errors = Vec::new();

    if block.classes.is_empty() {
        errors.push(Error::new(
            Span::call_site(),
            "oop_class! requires at least one class declaration",
        ));
        return Err(errors);
    }

    let mut graph = validate_and_build(block, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    rewrite_all_super_calls(&mut graph, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(generate(&graph))
}

fn validate_and_build(block: OopBlock, errors: &mut Vec<Error>) -> Graph {
    let classes = block.classes;
    let mut names = Vec::new();
    let mut name_to_index = HashMap::new();

    for (index, class) in classes.iter().enumerate() {
        let name = class.name.to_string();
        if class.is_override {
            errors.push(Error::new_spanned(
                &class.name,
                "`#[override]` is only allowed on virtual methods",
            ));
        }
        if name_to_index.insert(name.clone(), index).is_some() {
            errors.push(Error::new_spanned(
                &class.name,
                format!("duplicate class `{name}`"),
            ));
        }
        names.push(name);
    }

    for class in &classes {
        let cast_name = format!("As{}", class.name);
        if name_to_index.contains_key(&cast_name) {
            errors.push(Error::new_spanned(
                &class.name,
                format!(
                    "generated base-cast trait name `{cast_name}` conflicts with an existing class name"
                ),
            ));
        }
    }

    let mut bases = vec![Vec::new(); classes.len()];
    for (index, class) in classes.iter().enumerate() {
        let mut seen_bases = HashSet::new();
        for base in &class.bases {
            let base_name = base.to_string();
            let Some(&base_index) = name_to_index.get(&base_name) else {
                errors.push(Error::new_spanned(
                    base,
                    format!("unknown base class `{base_name}`"),
                ));
                continue;
            };
            if !seen_bases.insert(base_name.clone()) {
                errors.push(Error::new_spanned(
                    base,
                    format!("duplicate base class `{base_name}`"),
                ));
                continue;
            }
            bases[index].push(base_index);
        }
    }

    let direct_methods = collect_direct_methods(&classes, errors);
    validate_constructors(&classes, &bases, &name_to_index, errors);

    let mros = if errors.is_empty() {
        match c3::linearize_all(&bases) {
            Ok(mros) => mros,
            Err(error) => {
                let message = error.message(&names);
                let span = match error {
                    c3::C3Error::Cycle { class } | c3::C3Error::Inconsistent { class } => {
                        classes[class].name.span()
                    }
                };
                errors.push(Error::new(span, message));
                vec![Vec::new(); classes.len()]
            }
        }
    } else {
        vec![Vec::new(); classes.len()]
    };

    let (selected_methods, abstract_methods) = if errors.is_empty() {
        resolve_methods(&classes, &mros, &direct_methods, errors)
    } else {
        (
            vec![BTreeMap::new(); classes.len()],
            vec![BTreeMap::new(); classes.len()],
        )
    };
    validate_concrete_classes(&classes, &abstract_methods, errors);

    Graph {
        classes,
        names,
        name_to_index,
        bases,
        mros,
        selected_methods,
        abstract_methods,
    }
}

fn collect_direct_methods(
    classes: &[ClassDef],
    errors: &mut Vec<Error>,
) -> Vec<BTreeMap<String, MethodInfo>> {
    let mut result = Vec::with_capacity(classes.len());

    for (class_index, class) in classes.iter().enumerate() {
        let mut methods = BTreeMap::new();
        let mut fields = HashSet::new();

        for item in &class.items {
            match item {
                ClassItem::Field(field) => {
                    let field_name = field.ident.to_string();
                    if field.is_override {
                        errors.push(Error::new_spanned(
                            &field.ident,
                            "`#[override]` is only allowed on virtual methods",
                        ));
                    }
                    if field_name.starts_with("__oop_") {
                        errors.push(Error::new_spanned(
                            &field.ident,
                            "field names starting with `__oop_` are reserved",
                        ));
                    }
                    if !fields.insert(field_name.clone()) {
                        errors.push(Error::new_spanned(
                            &field.ident,
                            format!("duplicate field `{field_name}`"),
                        ));
                    }
                }
                ClassItem::Method(method) => {
                    let name = method.sig.ident.to_string();
                    if method.is_override && !method.is_virtual {
                        errors.push(Error::new_spanned(
                            &method.sig.ident,
                            format!(
                                "method `{name}` is marked `#[override]` but is not declared `virtual fn`"
                            ),
                        ));
                    }
                    if method.body.is_none() {
                        if !method.is_abstract_decl || !method.is_virtual {
                            errors.push(Error::new_spanned(
                                &method.sig.ident,
                                format!(
                                    "unimplemented method `{name}` must be declared as `abstract virtual fn`"
                                ),
                            ));
                        }
                        if !class.is_abstract {
                            errors.push(Error::new_spanned(
                                &method.sig.ident,
                                format!(
                                    "unimplemented method `{name}` is only allowed in an abstract class; declare `abstract class {}`",
                                    class.name
                                ),
                            ));
                        }
                    } else if method.is_abstract_decl {
                        errors.push(Error::new_spanned(
                            &method.sig.ident,
                            format!(
                                "method `{name}` is declared `abstract` but has an implementation"
                            ),
                        ));
                    }
                    if name.starts_with("__oop_") {
                        errors.push(Error::new_spanned(
                            &method.sig.ident,
                            "method names starting with `__oop_` are reserved",
                        ));
                    }
                    match analyze_method(class_index, method) {
                        Ok(info) => {
                            if methods.insert(name.clone(), info).is_some() {
                                errors.push(Error::new_spanned(
                                    &method.sig.ident,
                                    format!("duplicate method `{name}`"),
                                ));
                            }
                        }
                        Err(method_errors) => errors.extend(method_errors),
                    }
                }
                ClassItem::Constructor(_) => {}
            }
        }

        result.push(methods);
    }

    result
}

fn validate_constructors(
    classes: &[ClassDef],
    bases: &[Vec<usize>],
    name_to_index: &HashMap<String, usize>,
    errors: &mut Vec<Error>,
) {
    for (class_index, class) in classes.iter().enumerate() {
        let constructors = class_constructors(class).collect::<Vec<_>>();
        if constructors.len() > 1 {
            for constructor in constructors.iter().skip(1) {
                errors.push(Error::new(
                    constructor_keyword_span(constructor),
                    format!("duplicate constructor for class `{}`", class.name),
                ));
            }
        }

        let has_constructor = !constructors.is_empty();
        for item in &class.items {
            if let ClassItem::Method(method) = item {
                if has_constructor && method.sig.ident == "new" {
                    errors.push(Error::new_spanned(
                        &method.sig.ident,
                        "constructor generates method `new`, but class already declares method `new`",
                    ));
                }
            }
        }

        for constructor in constructors {
            if constructor.is_override {
                errors.push(Error::new(
                    constructor_keyword_span(constructor),
                    "`#[override]` is only allowed on virtual methods",
                ));
            }

            validate_constructor_inputs(constructor, errors);

            let mut seen_bases = HashSet::new();
            for base_call in &constructor.base_calls {
                let base_name = base_call.base.to_string();
                if !seen_bases.insert(base_name.clone()) {
                    errors.push(Error::new_spanned(
                        &base_call.base,
                        format!("duplicate constructor initializer for base `{base_name}`"),
                    ));
                    continue;
                }

                let Some(&base_index) = name_to_index.get(&base_name) else {
                    errors.push(Error::new_spanned(
                        &base_call.base,
                        format!("unknown base class `{base_name}`"),
                    ));
                    continue;
                };

                if !bases[class_index].contains(&base_index) {
                    errors.push(Error::new_spanned(
                        &base_call.base,
                        format!(
                            "constructor initializer `{base_name}` must name a direct base class of `{}`",
                            class.name
                        ),
                    ));
                }
            }
        }
    }
}

fn validate_constructor_inputs(constructor: &ConstructorDef, errors: &mut Vec<Error>) {
    for input in &constructor.inputs {
        match input {
            FnArg::Receiver(receiver) => errors.push(Error::new_spanned(
                receiver,
                "constructors do not support self receivers",
            )),
            FnArg::Typed(typed) => {
                if !matches!(typed.pat.as_ref(), Pat::Ident(_)) {
                    errors.push(Error::new_spanned(
                        &typed.pat,
                        "constructor arguments must use simple identifier patterns",
                    ));
                }
            }
        }
    }
}

fn constructor_keyword_span(constructor: &ConstructorDef) -> Span {
    constructor.constructor_token.span
}

fn validate_concrete_classes(
    classes: &[ClassDef],
    abstract_methods: &[BTreeMap<String, MethodInfo>],
    errors: &mut Vec<Error>,
) {
    for (class_index, class) in classes.iter().enumerate() {
        if class.is_abstract || abstract_methods[class_index].is_empty() {
            continue;
        }

        let method_list = abstract_methods[class_index]
            .keys()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(Error::new_spanned(
            &class.name,
            format!(
                "concrete class `{}` must implement abstract method(s) {method_list}; declare `abstract class {}` to keep it abstract",
                class.name, class.name
            ),
        ));
    }
}

fn analyze_method(owner: usize, method: &MethodDef) -> Result<MethodInfo, Vec<Error>> {
    let mut errors = Vec::new();
    let sig = &method.sig;

    if sig.constness.is_some() {
        errors.push(Error::new_spanned(
            sig.constness,
            "const methods are not supported",
        ));
    }
    if sig.asyncness.is_some() {
        errors.push(Error::new_spanned(
            sig.asyncness,
            "async methods are not supported",
        ));
    }
    if sig.unsafety.is_some() {
        errors.push(Error::new_spanned(
            sig.unsafety,
            "unsafe methods are not supported",
        ));
    }
    if sig.abi.is_some() {
        errors.push(Error::new_spanned(
            &sig.abi,
            "extern methods are not supported",
        ));
    }
    if sig.generics.lt_token.is_some()
        || !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some()
    {
        errors.push(Error::new_spanned(
            &sig.generics,
            "generic methods and where clauses are not supported",
        ));
    }
    if sig.variadic.is_some() {
        errors.push(Error::new_spanned(
            &sig.variadic,
            "variadic methods are not supported",
        ));
    }

    let Some(first_arg) = sig.inputs.first() else {
        errors.push(Error::new_spanned(
            sig,
            "methods must have a `&self` or `&mut self` receiver",
        ));
        return Err(errors);
    };

    let receiver = match first_arg {
        FnArg::Receiver(receiver) => validate_receiver(receiver, &mut errors),
        FnArg::Typed(arg) => {
            errors.push(Error::new_spanned(
                arg,
                "methods must start with a `&self` or `&mut self` receiver",
            ));
            None
        }
    };

    let mut arg_idents = Vec::new();
    let mut arg_types = Vec::new();
    let mut arg_type_keys = Vec::new();
    for arg in sig.inputs.iter().skip(1) {
        match arg {
            FnArg::Receiver(receiver) => errors.push(Error::new_spanned(
                receiver,
                "only one receiver is supported",
            )),
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat_ident) => {
                    arg_idents.push(pat_ident.ident.clone());
                    arg_type_keys.push(typed.ty.to_token_stream().to_string());
                    arg_types.push((*typed.ty).clone());
                }
                pattern => errors.push(Error::new_spanned(
                    pattern,
                    "method arguments must use simple identifier patterns",
                )),
            },
        }
    }

    let Some(receiver) = receiver else {
        return Err(errors);
    };

    if !errors.is_empty() {
        return Err(errors);
    }

    let output = match &sig.output {
        ReturnType::Default => String::new(),
        ReturnType::Type(_, ty) => ty.to_token_stream().to_string(),
    };
    let signature_key = format!("{receiver:?}|{}|{output}", arg_type_keys.join(","));
    let signature_display = sig.to_token_stream().to_string();

    Ok(MethodInfo {
        owner,
        name: sig.ident.clone(),
        vis: public_if_inherited(&method.vis),
        sig: sig.clone(),
        is_virtual: method.is_virtual,
        is_abstract: method.body.is_none(),
        is_override: method.is_override,
        receiver,
        arg_idents,
        arg_types,
        signature_key,
        signature_display,
    })
}

fn validate_receiver(receiver: &Receiver, errors: &mut Vec<Error>) -> Option<ReceiverKind> {
    if receiver.reference.is_none() {
        errors.push(Error::new_spanned(
            receiver,
            "by-value self receivers are not supported; use `&self` or `&mut self`",
        ));
        return None;
    }

    if receiver.colon_token.is_some() {
        errors.push(Error::new_spanned(
            receiver,
            "typed self receivers are not supported; use `&self` or `&mut self`",
        ));
        return None;
    }

    Some(if receiver.mutability.is_some() {
        ReceiverKind::Mutable
    } else {
        ReceiverKind::Shared
    })
}

fn public_if_inherited(vis: &Visibility) -> Visibility {
    match vis {
        Visibility::Inherited => parse_quote!(pub),
        other => other.clone(),
    }
}

fn resolve_methods(
    classes: &[ClassDef],
    mros: &[Vec<usize>],
    direct_methods: &[BTreeMap<String, MethodInfo>],
    errors: &mut Vec<Error>,
) -> (
    Vec<BTreeMap<String, MethodInfo>>,
    Vec<BTreeMap<String, MethodInfo>>,
) {
    let mut selected_by_class = Vec::with_capacity(classes.len());
    let mut abstract_by_class = Vec::with_capacity(classes.len());

    for (class_index, mro) in mros.iter().enumerate() {
        let mut selected = BTreeMap::new();
        let mut unresolved_abstract = BTreeMap::new();
        let current_direct = &direct_methods[class_index];
        let mut seen_names = BTreeSet::new();

        for &mro_class in mro {
            for (name, method) in &direct_methods[mro_class] {
                if method.is_virtual {
                    seen_names.insert(name.clone());
                }
            }
        }

        for name in seen_names {
            let declarations: Vec<_> = mro
                .iter()
                .filter_map(|&mro_class| direct_methods[mro_class].get(&name))
                .filter(|method| method.is_virtual)
                .collect();

            if declarations.is_empty() {
                continue;
            }

            if let Some(current) = current_direct.get(&name).filter(|method| method.is_virtual) {
                if current.is_override && declarations.len() == 1 {
                    errors.push(Error::new_spanned(
                        &current.name,
                        format!(
                            "method `{}` is marked `#[override]` but does not override an inherited virtual method",
                            name
                        ),
                    ));
                }
                if !current.is_override && declarations.len() > 1 {
                    errors.push(Error::new_spanned(
                        &current.name,
                        format!(
                            "method `{}` overrides an inherited virtual method but is missing `#[override]`",
                            name
                        ),
                    ));
                }

                if current.is_abstract {
                    for inherited in declarations.iter().skip(1) {
                        if current.signature_key != inherited.signature_key {
                            errors.push(Error::new_spanned(
                                &classes[class_index].name,
                                format!(
                                    "abstract method `{}` is incompatible with an inherited virtual declaration from `{}`",
                                    name, classes[inherited.owner].name
                                ),
                            ));
                            break;
                        }
                    }
                    unresolved_abstract.insert(name, current.clone());
                } else {
                    for inherited in declarations.iter().skip(1) {
                        if current.signature_key != inherited.signature_key {
                            errors.push(Error::new_spanned(
                                &current.name,
                                format!(
                                    "method `{}` does not satisfy the virtual signature declared in `{}`",
                                    name, classes[inherited.owner].name
                                ),
                            ));
                        }
                    }
                    selected.insert(name, current.clone());
                }

                continue;
            }

            let concrete_methods: Vec<_> = declarations
                .iter()
                .copied()
                .filter(|method| !method.is_abstract)
                .collect();
            let abstract_methods: Vec<_> = declarations
                .iter()
                .copied()
                .filter(|method| method.is_abstract)
                .collect();

            if let Some(selected_impl) = concrete_methods.first() {
                for other in concrete_methods.iter().skip(1) {
                    if selected_impl.signature_key != other.signature_key {
                        errors.push(Error::new_spanned(
                            &classes[class_index].name,
                            format!(
                                "method `{}` has incompatible inherited signatures; override it in `{}` to resolve the conflict",
                                name, classes[class_index].name
                            ),
                        ));
                        break;
                    }
                }

                for abstract_method in &abstract_methods {
                    if selected_impl.signature_key != abstract_method.signature_key {
                        errors.push(Error::new_spanned(
                            &classes[class_index].name,
                            format!(
                                "inherited method `{}` from `{}` does not satisfy the abstract signature declared in `{}`",
                                name,
                                classes[selected_impl.owner].name,
                                classes[abstract_method.owner].name
                            ),
                        ));
                    }
                }

                selected.insert(name, (*selected_impl).clone());
            } else if let Some(required_method) = abstract_methods.first() {
                for other in abstract_methods.iter().skip(1) {
                    if required_method.signature_key != other.signature_key {
                        errors.push(Error::new_spanned(
                            &classes[class_index].name,
                            format!(
                                "abstract method `{}` has incompatible inherited signatures; override it in `{}` to resolve the conflict",
                                name, classes[class_index].name
                            ),
                        ));
                        break;
                    }
                }

                unresolved_abstract.insert(name, (*required_method).clone());
            }
        }

        selected_by_class.push(selected);
        abstract_by_class.push(unresolved_abstract);
    }

    (selected_by_class, abstract_by_class)
}

fn rewrite_all_super_calls(graph: &mut Graph, errors: &mut Vec<Error>) {
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
                ClassItem::Field(_) => {}
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
    selected_methods: &'a [BTreeMap<String, MethodInfo>],
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

fn generate(graph: &Graph) -> TokenStream2 {
    let vtable_structs = graph
        .classes
        .iter()
        .enumerate()
        .filter(|(index, _)| has_virtual_interface(graph, *index))
        .map(|(index, class)| generate_vtable_struct(graph, index, class));
    let structs = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| generate_struct(graph, index, class));
    let base_cast_traits = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| generate_base_cast_trait(graph, index, class));
    let impls = graph
        .classes
        .iter()
        .enumerate()
        .map(|(index, class)| generate_impls(graph, index, class));
    let base_cast_impls = generate_base_cast_impls(graph);
    let vtable_items = generate_vtable_items(graph);

    quote! {
        #(#vtable_structs)*
        #(#structs)*
        #(#base_cast_traits)*
        #(#impls)*
        #base_cast_impls
        #vtable_items
    }
}

fn generate_struct(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let attrs = &class.attrs;
    let vis = public_if_inherited(&class.vis);
    let name = &class.name;
    let vtable_field = has_virtual_interface(graph, index).then(|| {
        let vtable_name = vtable_ident(&graph.names[index]);
        quote! {
            __oop_vtable: &'static #vtable_name,
        }
    });
    let base_fields = graph.bases[index].iter().map(|&base| {
        let field = base_field_ident(&graph.names[base]);
        let base_ident = &graph.classes[base].name;
        quote! {
            #field: #base_ident
        }
    });
    let fields = class.items.iter().filter_map(|item| match item {
        ClassItem::Field(field) => Some(quote! { #field }),
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });

    quote! {
        #(#attrs)*
        #[repr(C)]
        #vis struct #name {
            #vtable_field
            #(#base_fields,)*
            #(#fields,)*
        }
    }
}

fn generate_impls(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let name = &class.name;
    let metadata_impl = generate_metadata_impl(graph, index, class);
    let default_impl = generate_default_impl(graph, index, class);
    let default_base_impl = generate_default_base_impl(graph, index, class);
    let accessors = generate_accessors(graph, index, class);
    let vtable_init = generate_vtable_init(graph, index);
    let constructor_hook = generate_constructor_hook(graph, index, class);
    let constructor_new = generate_constructor_new(graph, index, class);
    let direct_methods = class.items.iter().filter_map(|item| match item {
        ClassItem::Method(method) if !method.is_virtual => generate_direct_method(method),
        ClassItem::Field(_) => None,
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });
    let virtual_impl_methods = class.items.iter().filter_map(|item| match item {
        ClassItem::Method(method) if method.is_virtual => generate_virtual_impl_method(method),
        ClassItem::Field(_) => None,
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });
    let virtual_wrappers = interface_methods(graph, index)
        .into_iter()
        .map(|method| generate_virtual_wrapper(&method));

    quote! {
        #metadata_impl

        #default_base_impl

        impl #name {
            #vtable_init
            #accessors
            #constructor_hook
            #constructor_new
            #(#direct_methods)*
            #(#virtual_impl_methods)*
            #(#virtual_wrappers)*
        }

        #default_impl
    }
}

fn generate_vtable_struct(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let vtable_name = vtable_ident(&graph.names[index]);
    let class_name = &class.name;
    let fields = interface_methods(graph, index).into_iter().map(|method| {
        let field = vtable_field_ident(&method.name);
        let receiver = match method.receiver {
            ReceiverKind::Shared => quote! { &#class_name },
            ReceiverKind::Mutable => quote! { &mut #class_name },
        };
        let arg_types = &method.arg_types;
        let output = &method.sig.output;

        quote! {
            #field: fn(#receiver #(, #arg_types)*) #output
        }
    });

    quote! {
        struct #vtable_name {
            #(#fields,)*
        }
    }
}

fn generate_base_cast_trait(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let vis = public_if_inherited(&class.vis);
    let trait_name = base_cast_trait_ident(&graph.names[index]);
    let class_name = &class.name;
    let shared_name = base_cast_method_ident(&graph.names[index], false);
    let mutable_name = base_cast_method_ident(&graph.names[index], true);

    quote! {
        #vis trait #trait_name {
            fn #shared_name(&self) -> &#class_name;
            fn #mutable_name(&mut self) -> &mut #class_name;
        }
    }
}

fn generate_base_cast_impls(graph: &Graph) -> TokenStream2 {
    let impls = graph
        .classes
        .iter()
        .enumerate()
        .flat_map(|(class_index, class)| {
            graph.mros[class_index]
                .iter()
                .copied()
                .map(move |base_index| {
                    generate_base_cast_impl(graph, class_index, base_index, class)
                })
        });

    quote! {
        #(#impls)*
    }
}

fn generate_base_cast_impl(
    graph: &Graph,
    class_index: usize,
    base_index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let class_name = &class.name;
    let base_name = &graph.classes[base_index].name;
    let trait_name = base_cast_trait_ident(&graph.names[base_index]);
    let shared_name = base_cast_method_ident(&graph.names[base_index], false);
    let mutable_name = base_cast_method_ident(&graph.names[base_index], true);
    let shared_body = if class_index == base_index {
        quote! { self }
    } else {
        accessor_body(graph, class_index, base_index, false)
    };
    let mutable_body = if class_index == base_index {
        quote! { self }
    } else {
        accessor_body(graph, class_index, base_index, true)
    };

    quote! {
        impl #trait_name for #class_name {
            fn #shared_name(&self) -> &#base_name {
                #shared_body
            }

            fn #mutable_name(&mut self) -> &mut #base_name {
                #mutable_body
            }
        }
    }
}

fn interface_methods(graph: &Graph, index: usize) -> Vec<MethodInfo> {
    let mut methods = BTreeMap::new();

    for (name, method) in &graph.selected_methods[index] {
        methods.insert(name.clone(), method.clone());
    }

    for (name, method) in &graph.abstract_methods[index] {
        methods.insert(name.clone(), method.clone());
    }

    methods.into_values().collect()
}

fn class_constructors(class: &ClassDef) -> impl Iterator<Item = &ConstructorDef> {
    class.items.iter().filter_map(|item| match item {
        ClassItem::Constructor(constructor) => Some(constructor),
        ClassItem::Field(_) | ClassItem::Method(_) => None,
    })
}

fn class_constructor(class: &ClassDef) -> Option<&ConstructorDef> {
    class_constructors(class).next()
}

fn has_virtual_interface(graph: &Graph, index: usize) -> bool {
    !graph.selected_methods[index].is_empty() || !graph.abstract_methods[index].is_empty()
}

fn vtable_slots(graph: &Graph, index: usize) -> Vec<VtableSlot> {
    let mut slots = Vec::new();
    collect_vtable_slots(graph, index, Vec::new(), &mut slots);
    slots
}

fn collect_vtable_slots(
    graph: &Graph,
    index: usize,
    path: Vec<usize>,
    slots: &mut Vec<VtableSlot>,
) {
    if has_virtual_interface(graph, index) {
        slots.push(VtableSlot {
            ancestor: index,
            path: path.clone(),
        });
    }

    for &base in &graph.bases[index] {
        let mut base_path = path.clone();
        base_path.push(base);
        collect_vtable_slots(graph, base, base_path, slots);
    }
}

fn generate_vtable_items(graph: &Graph) -> TokenStream2 {
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
    let vtable_type = vtable_ident(&graph.names[vtable_index]);
    let vtable_static = vtable_static_ident(graph, class_index, &vtable_slot);
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

    quote! {
        static #vtable_static: #vtable_type = #vtable_type {
            #(#entries,)*
        };

        #(#functions)*
    }
}

fn generate_vtable_function(
    graph: &Graph,
    class_index: usize,
    vtable_slot: &VtableSlot,
    method: &MethodInfo,
) -> TokenStream2 {
    let vtable_index = vtable_slot.ancestor;
    let function = vtable_function_ident(graph, class_index, vtable_slot, &method.name);
    let receiver_ty = &graph.classes[vtable_index].name;
    let arg_idents = &method.arg_idents;
    let arg_types = &method.arg_types;
    let output = &method.sig.output;
    let method_name = method.name.to_string();
    let vtable_class_name = &graph.names[vtable_index];

    let selected = graph.selected_methods[class_index]
        .get(&method.name.to_string())
        .filter(|selected| selected.signature_key == method.signature_key);

    match method.receiver {
        ReceiverKind::Shared => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, false);
                let call =
                    selected_virtual_impl_call(graph, class_index, selected, false, arg_idents);
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
                fn #function(receiver: &#receiver_ty #(, #arg_idents: #arg_types)*) #output {
                    #body
                }
            }
        }
        ReceiverKind::Mutable => {
            let body = if let Some(selected) = selected {
                let complete =
                    complete_from_receiver_expr(graph, class_index, &vtable_slot.path, true);
                let call =
                    selected_virtual_impl_call(graph, class_index, selected, true, arg_idents);
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
                fn #function(receiver: &mut #receiver_ty #(, #arg_idents: #arg_types)*) #output {
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
    let class_name = &graph.classes[class_index].name;
    if path.is_empty() {
        return quote! { receiver };
    }

    let offset = offset_expr(graph, class_index, path);
    if mutable {
        quote! {
            unsafe {
                let offset = #offset;
                &mut *((receiver as *mut _ as *mut u8).sub(offset) as *mut #class_name)
            }
        }
    } else {
        quote! {
            unsafe {
                let offset = #offset;
                &*((receiver as *const _ as *const u8).sub(offset) as *const #class_name)
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
) -> TokenStream2 {
    let method = virtual_impl_ident(&selected.name);
    if selected.owner == class_index {
        return quote! {
            complete.#method(#(#arg_idents),*)
        };
    }

    let owner_name = &graph.names[selected.owner];
    let accessor = if mutable {
        format_ident!("__oop_as_mut_{}", owner_name)
    } else {
        format_ident!("__oop_as_{}", owner_name)
    };

    quote! {
        complete.#accessor().#method(#(#arg_idents),*)
    }
}

fn offset_expr(graph: &Graph, class_index: usize, path: &[usize]) -> TokenStream2 {
    if path.is_empty() {
        return quote! { 0usize };
    }

    let class_name = &graph.classes[class_index].name;
    let mut field_tokens = TokenStream2::new();
    for &base in path {
        let field = base_field_ident(&graph.names[base]);
        field_tokens.extend(quote! { .#field });
    }

    quote! {
        {
            let uninit = ::core::mem::MaybeUninit::<#class_name>::uninit();
            let base = uninit.as_ptr();
            unsafe {
                let field = ::core::ptr::addr_of!((*base)#field_tokens);
                field as usize - base as usize
            }
        }
    }
}

fn generate_constructor_hook(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let constructor = class_constructor(class);
    let inputs = constructor
        .map(|constructor| constructor.inputs.as_slice())
        .unwrap_or(&[]);
    let base_calls = generate_constructor_base_calls(graph, index, constructor);
    let body = constructor.map(|constructor| &constructor.body);

    quote! {
        fn __oop_ctor(&mut self #(, #inputs)*) {
            #base_calls
            #body
        }
    }
}

fn generate_constructor_new(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    if class.is_abstract {
        return quote! {};
    }

    let Some(constructor) = class_constructor(class) else {
        return quote! {};
    };

    let attrs = &constructor.attrs;
    let vis = public_if_inherited(&constructor.vis);
    let inputs = &constructor.inputs;
    let args = constructor_arg_idents(constructor);
    let trait_name = default_base_trait_ident(&graph.names[index]);

    quote! {
        #(#attrs)*
        #vis fn new(#(#inputs),*) -> Self {
            let mut value = <Self as #trait_name>::__oop_default_base();
            value.__oop_ctor(#(#args),*);
            value
        }
    }
}

fn generate_constructor_base_calls(
    graph: &Graph,
    index: usize,
    constructor: Option<&ConstructorDef>,
) -> TokenStream2 {
    let calls = graph.bases[index].iter().map(|&base| {
        let base_name = &graph.names[base];
        let accessor = format_ident!("__oop_as_mut_{}", base_name);
        let explicit_call = constructor.and_then(|constructor| {
            constructor
                .base_calls
                .iter()
                .find(|base_call| base_call.base == base_name.as_str())
        });

        if let Some(base_call) = explicit_call {
            let args = &base_call.args;
            quote! {
                self.#accessor().__oop_ctor(#(#args),*);
            }
        } else {
            quote! {
                self.#accessor().__oop_ctor();
            }
        }
    });

    quote! {
        #(#calls)*
    }
}

fn constructor_arg_idents(constructor: &ConstructorDef) -> Vec<Ident> {
    constructor
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}

fn generate_default_base_impl(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let name = &class.name;
    let trait_name = default_base_trait_ident(&graph.names[index]);
    let vtable_initializer = has_virtual_interface(graph, index).then(|| {
        let vtable = vtable_static_ident(
            graph,
            index,
            &VtableSlot {
                ancestor: index,
                path: Vec::new(),
            },
        );
        quote! {
            __oop_vtable: &#vtable,
        }
    });
    let base_initializers = graph.bases[index].iter().map(|&base| {
        let field = base_field_ident(&graph.names[base]);
        let base_ident = &graph.classes[base].name;
        let base_trait = default_base_trait_ident(&graph.names[base]);
        quote! {
            #field: <#base_ident as #base_trait>::__oop_default_base()
        }
    });
    let field_initializers = class.items.iter().filter_map(|item| match item {
        ClassItem::Field(field) => {
            let ident = &field.ident;
            Some(quote! {
                #ident: ::core::default::Default::default()
            })
        }
        ClassItem::Method(_) => None,
        ClassItem::Constructor(_) => None,
    });

    quote! {
        trait #trait_name {
            fn __oop_default_base() -> Self;
        }

        impl #trait_name for #name {
            fn __oop_default_base() -> Self {
                let mut value = Self {
                    #vtable_initializer
                    #(#base_initializers,)*
                    #(#field_initializers,)*
                };
                value.__oop_init_vtables();
                value
            }
        }
    }
}

fn generate_default_impl(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    if class.is_abstract {
        return quote! {};
    }

    let name = &class.name;
    let trait_name = default_base_trait_ident(&graph.names[index]);
    quote! {
        impl ::core::default::Default for #name {
            fn default() -> Self {
                <Self as #trait_name>::__oop_default_base()
            }
        }
    }
}

fn generate_vtable_init(graph: &Graph, index: usize) -> TokenStream2 {
    let assignments = vtable_slots(graph, index).into_iter().map(|slot| {
        let vtable = vtable_static_ident(graph, index, &slot);
        let place = place_for_path(graph, &slot.path);

        quote! {
            #place.__oop_vtable = &#vtable;
        }
    });

    quote! {
        fn __oop_init_vtables(&mut self) {
            #(#assignments)*
        }
    }
}

fn generate_direct_method(method: &MethodDef) -> Option<TokenStream2> {
    let body = method.body.as_ref()?;
    let attrs = &method.attrs;
    let vis = public_if_inherited(&method.vis);
    let sig = &method.sig;

    Some(quote! {
        #(#attrs)*
        #vis #sig #body
    })
}

fn generate_virtual_impl_method(method: &MethodDef) -> Option<TokenStream2> {
    let body = method.body.as_ref()?;
    let attrs = &method.attrs;
    let mut sig = method.sig.clone();
    sig.ident = virtual_impl_ident(&method.sig.ident);

    Some(quote! {
        #(#attrs)*
        #sig #body
    })
}

fn generate_virtual_wrapper(method: &MethodInfo) -> TokenStream2 {
    let sig = &method.sig;
    let vis = &method.vis;
    let field = vtable_field_ident(&method.name);
    let args = &method.arg_idents;

    quote! {
        #vis #sig {
            (self.__oop_vtable.#field)(self, #(#args),*)
        }
    }
}

fn generate_metadata_impl(graph: &Graph, index: usize, class: &ClassDef) -> TokenStream2 {
    let name_ident = &class.name;
    let name = &graph.names[index];
    let mro = graph.mros[index].iter().map(|&mro_index| {
        let mro_name = &graph.names[mro_index];
        quote! { #mro_name }
    });
    let methods = graph.selected_methods[index].values().map(|method| {
        let method_name = method.name.to_string();
        let owner_name = &graph.names[method.owner];
        let signature = &method.signature_display;
        quote! {
            ::oop_mro::MethodEntry {
                name: #method_name,
                owner: #owner_name,
                signature: #signature,
            }
        }
    });
    let abstract_methods = graph.abstract_methods[index].values().map(|method| {
        let method_name = method.name.to_string();
        let owner_name = &graph.names[method.owner];
        let signature = &method.signature_display;
        quote! {
            ::oop_mro::MethodEntry {
                name: #method_name,
                owner: #owner_name,
                signature: #signature,
            }
        }
    });
    let is_abstract = class.is_abstract;

    quote! {
        impl ::oop_mro::OopClass for #name_ident {
            const NAME: &'static str = #name;
            const MRO: &'static [&'static str] = &[#(#mro),*];
            const IS_ABSTRACT: bool = #is_abstract;
            const METHOD_TABLE: &'static ::oop_mro::MethodTable = &::oop_mro::MethodTable {
                methods: &[#(#methods),*],
            };
            const ABSTRACT_METHODS: &'static [::oop_mro::MethodEntry] = &[#(#abstract_methods),*];
        }

        impl ::oop_mro::OopObject for #name_ident {
            type Class = Self;
        }
    }
}

fn generate_accessors(graph: &Graph, index: usize, _class: &ClassDef) -> TokenStream2 {
    let mut generated = Vec::new();
    let mut seen = HashSet::new();

    for &ancestor in graph.mros[index].iter().skip(1) {
        if !seen.insert(ancestor) {
            continue;
        }

        let ancestor_name = &graph.names[ancestor];
        let ancestor_ident = &graph.classes[ancestor].name;
        let shared_name = format_ident!("__oop_as_{}", ancestor_name);
        let mutable_name = format_ident!("__oop_as_mut_{}", ancestor_name);
        let shared_body = accessor_body(graph, index, ancestor, false);
        let mutable_body = accessor_body(graph, index, ancestor, true);

        generated.push(quote! {
            fn #shared_name(&self) -> &#ancestor_ident {
                #shared_body
            }

            fn #mutable_name(&mut self) -> &mut #ancestor_ident {
                #mutable_body
            }
        });
    }

    quote! {
        #(#generated)*
    }
}

fn accessor_body(graph: &Graph, start: usize, target: usize, mutable: bool) -> TokenStream2 {
    let path = find_base_path(graph, start, target).expect("ancestor path must exist");
    let mut tokens = if mutable {
        quote! { self }
    } else {
        quote! { self }
    };

    for base in path {
        let field = base_field_ident(&graph.names[base]);
        tokens = quote! { #tokens.#field };
    }

    if mutable {
        quote! { &mut #tokens }
    } else {
        quote! { &#tokens }
    }
}

fn place_for_path(graph: &Graph, path: &[usize]) -> TokenStream2 {
    let mut tokens = quote! { self };

    for &base in path {
        let field = base_field_ident(&graph.names[base]);
        tokens = quote! { #tokens.#field };
    }

    tokens
}

fn find_base_path(graph: &Graph, start: usize, target: usize) -> Option<Vec<usize>> {
    for &base in &graph.bases[start] {
        if base == target {
            return Some(vec![base]);
        }

        if graph.mros[base].contains(&target) {
            let mut path = vec![base];
            path.extend(find_base_path(graph, base, target)?);
            return Some(path);
        }
    }

    None
}

fn base_field_ident(name: &str) -> Ident {
    format_ident!("__oop_base_{}", to_snake(name))
}

fn base_cast_trait_ident(name: &str) -> Ident {
    format_ident!("As{}", name)
}

fn base_cast_method_ident(name: &str, mutable: bool) -> Ident {
    let name = to_snake(name);
    if mutable {
        format_ident!("as_{}_mut", name)
    } else {
        format_ident!("as_{}", name)
    }
}

fn default_base_trait_ident(name: &str) -> Ident {
    format_ident!("__oop_DefaultBase_{}", name)
}

fn vtable_ident(name: &str) -> Ident {
    format_ident!("__oop_VTable_{}", name)
}

fn vtable_static_ident(graph: &Graph, class_index: usize, slot: &VtableSlot) -> Ident {
    let class_name = &graph.names[class_index];
    let slot_name = vtable_slot_name(graph, slot);
    format_ident!("__OOP_VTABLE_{}_AS_{}", class_name, slot_name)
}

fn vtable_field_ident(method: &Ident) -> Ident {
    format_ident!("__oop_vcall_{}", method)
}

fn vtable_function_ident(
    graph: &Graph,
    class_index: usize,
    slot: &VtableSlot,
    method: &Ident,
) -> Ident {
    let class_name = to_snake(&graph.names[class_index]);
    let slot_name = to_snake(&vtable_slot_name(graph, slot));
    format_ident!("__oop_vcall_{}_as_{}_{}", class_name, slot_name, method)
}

fn virtual_impl_ident(method: &Ident) -> Ident {
    format_ident!("__oop_impl_{}", method)
}

fn vtable_slot_name(graph: &Graph, slot: &VtableSlot) -> String {
    if slot.path.is_empty() {
        return graph.names[slot.ancestor].clone();
    }

    slot.path
        .iter()
        .map(|&index| graph.names[index].as_str())
        .collect::<Vec<_>>()
        .join("_")
}

fn to_snake(name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 && !output.ends_with('_') {
                output.push('_');
            }
            output.push(ch.to_ascii_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}
