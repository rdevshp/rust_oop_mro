use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use proc_macro2::Span;
use quote::ToTokens;
use syn::{parse_quote, Error, FnArg, Pat, Receiver, Visibility};

use crate::ast::{
    AssociatedConstDef, AssociatedTypeDef, ClassDef, ClassItem, ConstructorDef, FieldDef,
    MethodDef, OopBlock,
};
use crate::c3;
use crate::generics::method_signature_key_in_context;
use crate::model::{Graph, MethodInfo, MethodMap, ReceiverKind};
use crate::types::class_constructors;

pub(crate) fn validate_and_build(block: OopBlock, errors: &mut Vec<Error>) -> Graph {
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
            let base_name = base.name.to_string();
            let Some(&base_index) = name_to_index.get(&base_name) else {
                errors.push(Error::new_spanned(
                    &base.name,
                    format!("unknown base class `{base_name}`"),
                ));
                continue;
            };
            if !seen_bases.insert(base_name.clone()) {
                errors.push(Error::new_spanned(
                    &base.name,
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
        resolve_methods(&classes, &bases, &mros, &direct_methods, errors)
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

fn collect_direct_methods(classes: &[ClassDef], errors: &mut Vec<Error>) -> Vec<MethodMap> {
    let mut result = Vec::with_capacity(classes.len());

    for (class_index, class) in classes.iter().enumerate() {
        let mut methods = BTreeMap::new();
        let mut fields = HashSet::new();
        let mut method_names = HashSet::new();
        let mut associated_items = HashSet::new();

        for item in &class.items {
            match item {
                ClassItem::Field(field) => validate_field(field, &mut fields, errors),
                ClassItem::Method(method) => {
                    let name = method.sig.ident.to_string();
                    let is_duplicate_method = !method_names.insert(name.clone());
                    let is_duplicate_associated_item = !associated_items.insert(name.clone());
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
                    if is_duplicate_method {
                        errors.push(Error::new_spanned(
                            &method.sig.ident,
                            format!("duplicate method `{name}`"),
                        ));
                    } else if is_duplicate_associated_item {
                        errors.push(Error::new_spanned(
                            &method.sig.ident,
                            format!("duplicate associated item `{name}`"),
                        ));
                    }
                    match analyze_method(class_index, method) {
                        Ok(Some(info)) => {
                            if !is_duplicate_method && !is_duplicate_associated_item {
                                methods.insert(name.clone(), info);
                            }
                        }
                        Ok(None) => {}
                        Err(method_errors) => errors.extend(method_errors),
                    }
                }
                ClassItem::AssociatedConst(associated_const) => {
                    validate_associated_const(associated_const, &mut associated_items, errors);
                }
                ClassItem::UnsupportedAssociatedType(associated_type) => {
                    validate_unsupported_associated_type(associated_type, errors);
                }
                ClassItem::Constructor(_) => {}
            }
        }

        result.push(methods);
    }

    result
}

fn validate_field(field: &FieldDef, fields: &mut HashSet<String>, errors: &mut Vec<Error>) {
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

fn validate_associated_const(
    associated_const: &AssociatedConstDef,
    associated_items: &mut HashSet<String>,
    errors: &mut Vec<Error>,
) {
    let name = associated_const.item.ident.to_string();
    if associated_const.is_override {
        errors.push(Error::new_spanned(
            &associated_const.item.ident,
            "`#[override]` is only allowed on virtual methods",
        ));
    }
    if name.starts_with("__oop_") {
        errors.push(Error::new_spanned(
            &associated_const.item.ident,
            "associated item names starting with `__oop_` are reserved",
        ));
    }
    if !associated_items.insert(name.clone()) {
        errors.push(Error::new_spanned(
            &associated_const.item.ident,
            format!("duplicate associated item `{name}`"),
        ));
    }
}

fn validate_unsupported_associated_type(
    associated_type: &AssociatedTypeDef,
    errors: &mut Vec<Error>,
) {
    if associated_type.is_override {
        errors.push(Error::new_spanned(
            &associated_type.item.ident,
            "`#[override]` is only allowed on virtual methods",
        ));
    }
    errors.push(Error::new_spanned(
        &associated_type.item.ident,
        "associated types in class bodies are not supported because Rust inherent associated types are unstable",
    ));
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
            if !has_constructor {
                continue;
            }

            match item {
                ClassItem::Method(method) if method.sig.ident == "new" => {
                    errors.push(Error::new_spanned(
                        &method.sig.ident,
                        "constructor generates method `new`, but class already declares method `new`",
                    ));
                }
                ClassItem::AssociatedConst(associated_const)
                    if associated_const.item.ident == "new" =>
                {
                    errors.push(Error::new_spanned(
                        &associated_const.item.ident,
                        "constructor generates method `new`, but class already declares associated item `new`",
                    ));
                }
                ClassItem::UnsupportedAssociatedType(associated_type)
                    if associated_type.item.ident == "new" =>
                {
                    errors.push(Error::new_spanned(
                        &associated_type.item.ident,
                        "constructor generates method `new`, but class already declares associated item `new`",
                    ));
                }
                _ => {}
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
    abstract_methods: &[MethodMap],
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

fn analyze_method(owner: usize, method: &MethodDef) -> Result<Option<MethodInfo>, Vec<Error>> {
    let mut errors = Vec::new();
    let sig = &method.sig;

    if sig.constness.is_some() && sig.asyncness.is_some() {
        errors.push(Error::new_spanned(
            sig.constness,
            "const async methods are not supported",
        ));
    }
    if method.is_virtual && sig.constness.is_some() {
        errors.push(Error::new_spanned(
            sig.constness,
            "virtual const methods are not supported",
        ));
    }
    if sig.abi.is_some() {
        errors.push(Error::new_spanned(
            &sig.abi,
            "extern methods are not supported",
        ));
    }
    let has_method_generics = sig.generics.lt_token.is_some()
        || !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some();
    if method.is_virtual && has_method_generics {
        errors.push(Error::new_spanned(
            &sig.generics,
            "generic virtual methods and virtual method where clauses are not supported",
        ));
    }
    if sig.variadic.is_some() {
        errors.push(Error::new_spanned(
            &sig.variadic,
            "variadic methods are not supported",
        ));
    }

    if !method.is_virtual {
        validate_non_virtual_method_inputs(sig, &mut errors);
        if errors.is_empty() {
            return Ok(None);
        }
        return Err(errors);
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
    for arg in sig.inputs.iter().skip(1) {
        match arg {
            FnArg::Receiver(receiver) => errors.push(Error::new_spanned(
                receiver,
                "only one receiver is supported",
            )),
            FnArg::Typed(typed) => match typed.pat.as_ref() {
                Pat::Ident(pat_ident) => {
                    arg_idents.push(pat_ident.ident.clone());
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

    let signature_display = sig.to_token_stream().to_string();

    Ok(Some(MethodInfo {
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
        signature_display,
    }))
}

fn validate_non_virtual_method_inputs(sig: &syn::Signature, errors: &mut Vec<Error>) {
    for (index, arg) in sig.inputs.iter().enumerate() {
        match arg {
            FnArg::Receiver(receiver) if index == 0 => {
                validate_receiver(receiver, errors);
            }
            FnArg::Receiver(receiver) => errors.push(Error::new_spanned(
                receiver,
                "only one receiver is supported",
            )),
            FnArg::Typed(typed) => {
                if !matches!(typed.pat.as_ref(), Pat::Ident(_)) {
                    errors.push(Error::new_spanned(
                        &typed.pat,
                        "method arguments must use simple identifier patterns",
                    ));
                }
            }
        }
    }
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

pub(crate) fn public_if_inherited(vis: &Visibility) -> Visibility {
    match vis {
        Visibility::Inherited => parse_quote!(pub),
        other => other.clone(),
    }
}

fn resolve_methods(
    classes: &[ClassDef],
    bases: &[Vec<usize>],
    mros: &[Vec<usize>],
    direct_methods: &[MethodMap],
    errors: &mut Vec<Error>,
) -> (Vec<MethodMap>, Vec<MethodMap>) {
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
            let signature_key = |method: &MethodInfo| {
                method_signature_key_in_context(classes, bases, mros, class_index, method)
            };
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
                    let current_key = signature_key(current);
                    for inherited in declarations.iter().skip(1) {
                        if current_key != signature_key(inherited) {
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
                    let current_key = signature_key(current);
                    for inherited in declarations.iter().skip(1) {
                        if current_key != signature_key(inherited) {
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
                let selected_key = signature_key(selected_impl);
                for other in concrete_methods.iter().skip(1) {
                    if selected_key != signature_key(other) {
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
                    if selected_key != signature_key(abstract_method) {
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
                let required_key = signature_key(required_method);
                for other in abstract_methods.iter().skip(1) {
                    if required_key != signature_key(other) {
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
