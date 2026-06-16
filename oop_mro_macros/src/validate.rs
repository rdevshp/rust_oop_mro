use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use proc_macro2::Span;
use quote::ToTokens;
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote, Error, ExprPath, FnArg, GenericParam, Generics, Lifetime, Pat, Receiver, TypePath,
    Visibility,
};

use crate::ast::{
    AssociatedConstDef, AssociatedTypeDef, ClassDef, ClassItem, ConstructorDef, FieldDef,
    MethodDef, OopBlock, StaticFieldDef,
};
use crate::c3;
use crate::generics::method_signature_key_in_context;
use crate::model::{BaseEdge, Graph, MethodInfo, MethodMap, ReceiverKind};
use crate::types::{
    ancestor_type_for_path_in, cast_target_key, class_constructors, class_type, type_key,
};

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
    let mut base_edges = vec![Vec::new(); classes.len()];
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
            base_edges[index].push(BaseEdge {
                base: base_index,
                is_virtual: base.is_virtual,
            });
        }
    }

    let direct_methods = collect_direct_methods(&classes, errors);
    validate_constructors(&classes, &bases, &base_edges, &name_to_index, errors);

    let mros = if errors.is_empty() {
        let mro_bases = virtual_aware_mro_bases(&base_edges, &bases);
        match c3::linearize_all(&mro_bases) {
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
    let warnings = Vec::new();
    let cast_target_ids = if errors.is_empty() {
        build_cast_target_ids(&classes, &base_edges)
    } else {
        HashMap::new()
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
        base_edges,
        bases,
        mros,
        cast_target_ids,
        warnings,
        selected_methods,
        abstract_methods,
    }
}

fn virtual_aware_mro_bases(base_edges: &[Vec<BaseEdge>], bases: &[Vec<usize>]) -> Vec<Vec<usize>> {
    bases
        .iter()
        .enumerate()
        .map(|(class_index, direct_bases)| {
            direct_bases
                .iter()
                .copied()
                .filter(|&base| {
                    let Some(edge) = base_edges[class_index]
                        .iter()
                        .find(|edge| edge.base == base)
                    else {
                        return true;
                    };
                    if !edge.is_virtual {
                        return true;
                    }

                    !base_edges[class_index]
                        .iter()
                        .filter(|other| other.base != base)
                        .any(|other| {
                            let mut visited = HashSet::new();
                            reaches_base(base_edges, other.base, base, &mut visited)
                        })
                })
                .collect()
        })
        .collect()
}

fn reaches_base(
    base_edges: &[Vec<BaseEdge>],
    current: usize,
    target: usize,
    visited: &mut HashSet<usize>,
) -> bool {
    if !visited.insert(current) {
        return false;
    }

    base_edges[current]
        .iter()
        .any(|edge| edge.base == target || reaches_base(base_edges, edge.base, target, visited))
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
                ClassItem::StaticField(static_field) => {
                    validate_static_field(
                        static_field,
                        &class.generics,
                        &mut associated_items,
                        errors,
                    );
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

fn validate_static_field(
    static_field: &StaticFieldDef,
    class_generics: &Generics,
    associated_items: &mut HashSet<String>,
    errors: &mut Vec<Error>,
) {
    let name = static_field.ident.to_string();
    if static_field.is_override {
        errors.push(Error::new_spanned(
            &static_field.ident,
            "`#[override]` is only allowed on virtual methods",
        ));
    }
    if let Some(mutability) = &static_field.mutability {
        errors.push(Error::new_spanned(
            mutability,
            "mutable static class fields are not supported; use a type with interior mutability such as Atomic*, Mutex, RwLock, or OnceLock",
        ));
    }
    if name.starts_with("__oop_") {
        errors.push(Error::new_spanned(
            &static_field.ident,
            "associated item names starting with `__oop_` are reserved",
        ));
    }
    if !associated_items.insert(name.clone()) {
        errors.push(Error::new_spanned(
            &static_field.ident,
            format!("duplicate associated item `{name}`"),
        ));
    }

    validate_static_field_generics(static_field, class_generics, errors);
}

fn validate_static_field_generics(
    static_field: &StaticFieldDef,
    class_generics: &Generics,
    errors: &mut Vec<Error>,
) {
    let mut value_generics = HashSet::new();
    let mut lifetime_generics = HashSet::new();

    for param in &class_generics.params {
        match param {
            GenericParam::Type(param) => {
                value_generics.insert(param.ident.to_string());
            }
            GenericParam::Const(param) => {
                value_generics.insert(param.ident.to_string());
            }
            GenericParam::Lifetime(param) => {
                lifetime_generics.insert(param.lifetime.ident.to_string());
            }
        }
    }

    if value_generics.is_empty() && lifetime_generics.is_empty() {
        return;
    }

    let mut checker = StaticFieldGenericChecker {
        value_generics,
        lifetime_generics,
        reported: HashSet::new(),
        errors: Vec::new(),
    };
    let mut ty = static_field.ty.clone();
    checker.visit_type_mut(&mut ty);
    let mut expr = static_field.expr.clone();
    checker.visit_expr_mut(&mut expr);
    errors.extend(checker.errors);
}

struct StaticFieldGenericChecker {
    value_generics: HashSet<String>,
    lifetime_generics: HashSet<String>,
    reported: HashSet<String>,
    errors: Vec<Error>,
}

impl StaticFieldGenericChecker {
    fn check_value_ident(&mut self, ident: &syn::Ident) {
        let name = ident.to_string();
        if self.value_generics.contains(&name) && self.reported.insert(name) {
            self.errors.push(Error::new_spanned(
                ident,
                "static class fields cannot reference class generic parameters",
            ));
        }
    }

    fn check_lifetime(&mut self, lifetime: &Lifetime) {
        let name = lifetime.ident.to_string();
        if self.lifetime_generics.contains(&name) && self.reported.insert(name) {
            self.errors.push(Error::new_spanned(
                lifetime,
                "static class fields cannot reference class generic parameters",
            ));
        }
    }
}

impl VisitMut for StaticFieldGenericChecker {
    fn visit_type_path_mut(&mut self, node: &mut TypePath) {
        if node.qself.is_none() {
            if let Some(segment) = node.path.segments.first() {
                self.check_value_ident(&segment.ident);
            }
        }

        visit_mut::visit_type_path_mut(self, node);
    }

    fn visit_expr_path_mut(&mut self, node: &mut ExprPath) {
        if node.qself.is_none() {
            if let Some(segment) = node.path.segments.first() {
                self.check_value_ident(&segment.ident);
            }
        }

        visit_mut::visit_expr_path_mut(self, node);
    }

    fn visit_lifetime_mut(&mut self, node: &mut Lifetime) {
        self.check_lifetime(node);

        visit_mut::visit_lifetime_mut(self, node);
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
    base_edges: &[Vec<BaseEdge>],
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
                ClassItem::StaticField(static_field) if static_field.ident == "new" => {
                    errors.push(Error::new_spanned(
                        &static_field.ident,
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
            let mut initialized_direct_bases = HashSet::new();
            for base_call in &constructor.base_calls {
                let base_name = base_call.base.to_string();
                let base_call_key = constructor_base_call_key(base_call);
                if !seen_bases.insert(base_call_key) {
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

                let is_direct = bases[class_index].contains(&base_index);
                let virtual_target_count = matching_virtual_base_target_count(
                    classes,
                    base_edges,
                    class_index,
                    base_index,
                    base_call,
                );
                if !is_direct && virtual_target_count == 0 {
                    errors.push(Error::new_spanned(
                        &base_call.base,
                        format!(
                            "constructor initializer `{base_name}` must name a direct base class or virtual base class of `{}`",
                            class.name
                        ),
                    ));
                } else if !is_direct && virtual_target_count > 1 {
                    errors.push(Error::new_spanned(
                        &base_call.base,
                        format!(
                            "constructor initializer `{base_name}` is ambiguous; use `{}<...>(...)` to name the virtual base specialization",
                            base_call.base
                        ),
                    ));
                } else {
                    if is_direct {
                        initialized_direct_bases.insert(base_index);
                    }
                }
            }

            for &base_index in &bases[class_index] {
                if !initialized_direct_bases.contains(&base_index) {
                    errors.push(Error::new(
                        constructor_keyword_span(constructor),
                        format!(
                            "constructor for class `{}` must explicitly initialize direct base `{}`",
                            class.name, classes[base_index].name
                        ),
                    ));
                }
            }
        }
    }
}

fn constructor_base_call_key(base_call: &crate::ast::ConstructorBaseCall) -> String {
    if type_has_explicit_generics(&base_call.ty) {
        type_key(&base_call.ty)
    } else {
        base_call.base.to_string()
    }
}

fn matching_virtual_base_target_count(
    classes: &[ClassDef],
    base_edges: &[Vec<BaseEdge>],
    start: usize,
    target: usize,
    base_call: &crate::ast::ConstructorBaseCall,
) -> usize {
    let mut matches = HashSet::new();
    collect_matching_virtual_base_targets(
        classes,
        base_edges,
        start,
        start,
        target,
        base_call,
        Vec::new(),
        &mut matches,
    );
    matches.len()
}

#[allow(clippy::too_many_arguments)]
fn collect_matching_virtual_base_targets(
    classes: &[ClassDef],
    base_edges: &[Vec<BaseEdge>],
    root: usize,
    current: usize,
    target: usize,
    base_call: &crate::ast::ConstructorBaseCall,
    path: Vec<usize>,
    matches: &mut HashSet<String>,
) {
    for edge in &base_edges[current] {
        let mut next_path = path.clone();
        next_path.push(edge.base);
        if edge.is_virtual && edge.base == target {
            let actual = ancestor_type_for_path_in(classes, root, &next_path);
            if constructor_base_call_matches(base_call, &actual) {
                matches.insert(type_key(&actual));
            }
        }
        collect_matching_virtual_base_targets(
            classes, base_edges, root, edge.base, target, base_call, next_path, matches,
        );
    }
}

fn constructor_base_call_matches(
    base_call: &crate::ast::ConstructorBaseCall,
    actual: &syn::Type,
) -> bool {
    !type_has_explicit_generics(&base_call.ty) || type_key(&base_call.ty) == type_key(actual)
}

fn type_has_explicit_generics(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.first() else {
        return false;
    };
    matches!(
        &segment.arguments,
        syn::PathArguments::AngleBracketed(arguments) if !arguments.args.is_empty()
    )
}

fn build_cast_target_ids(
    classes: &[ClassDef],
    base_edges: &[Vec<BaseEdge>],
) -> HashMap<String, usize> {
    let mut ids = HashMap::new();

    for (index, class) in classes.iter().enumerate() {
        let actual = class_type(class);
        ids.insert(cast_target_key(index, &actual), index);
    }

    let mut next_id = classes.len();
    for start in 0..classes.len() {
        let mut paths = Vec::new();
        collect_all_inheritance_paths(base_edges, start, Vec::new(), &mut paths);
        for path in paths {
            let Some(&target) = path.last() else {
                continue;
            };
            let actual = ancestor_type_for_path_in(classes, start, &path);
            let key = cast_target_key(target, &actual);
            ids.entry(key).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
        }
    }

    ids
}

fn collect_all_inheritance_paths(
    base_edges: &[Vec<BaseEdge>],
    current: usize,
    steps: Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    for edge in &base_edges[current] {
        let mut next_steps = steps.clone();
        next_steps.push(edge.base);
        paths.push(next_steps.clone());
        collect_all_inheritance_paths(base_edges, edge.base, next_steps, paths);
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
