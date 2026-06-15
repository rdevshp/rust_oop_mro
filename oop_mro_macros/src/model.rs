use std::collections::{BTreeMap, HashMap};

use syn::{Ident, Signature, Type, Visibility};

use crate::ast::ClassDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReceiverKind {
    Shared,
    Mutable,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodInfo {
    pub(crate) owner: usize,
    pub(crate) name: Ident,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
    pub(crate) is_virtual: bool,
    pub(crate) is_abstract: bool,
    pub(crate) is_override: bool,
    pub(crate) receiver: ReceiverKind,
    pub(crate) arg_idents: Vec<Ident>,
    pub(crate) arg_types: Vec<Type>,
    pub(crate) signature_display: String,
}

pub(crate) type MethodMap = BTreeMap<String, MethodInfo>;

#[derive(Debug)]
pub(crate) struct Graph {
    pub(crate) classes: Vec<ClassDef>,
    pub(crate) names: Vec<String>,
    pub(crate) name_to_index: HashMap<String, usize>,
    pub(crate) bases: Vec<Vec<usize>>,
    pub(crate) mros: Vec<Vec<usize>>,
    pub(crate) selected_methods: Vec<MethodMap>,
    pub(crate) abstract_methods: Vec<MethodMap>,
}

#[derive(Debug, Clone)]
pub(crate) struct VtableSlot {
    pub(crate) ancestor: usize,
    pub(crate) path: Vec<usize>,
}
