extern crate self as oop_mro;

pub use oop_mro_macros::oop_class;

pub trait OopClass {
    const NAME: &'static str;
    const MRO: &'static [&'static str];
    const IS_ABSTRACT: bool = false;
    const METHOD_TABLE: &'static MethodTable = &EMPTY_METHOD_TABLE;
    const ABSTRACT_METHODS: &'static [MethodEntry] = &[];
}

pub trait OopObject {
    type Class: OopClass;
}

pub type MethodFn = fn();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodEntry {
    pub name: &'static str,
    pub owner: &'static str,
    pub signature: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodTable {
    pub methods: &'static [MethodEntry],
}

impl MethodTable {
    pub const fn empty() -> Self {
        Self { methods: &[] }
    }

    pub fn find(&self, name: &str) -> Option<&'static MethodEntry> {
        let mut index = 0;
        while index < self.methods.len() {
            let entry = &self.methods[index];
            if str_eq(entry.name, name) {
                return Some(entry);
            }
            index += 1;
        }
        None
    }
}

pub static EMPTY_METHOD_TABLE: MethodTable = MethodTable::empty();

fn str_eq(left: &str, right: &str) -> bool {
    left.as_bytes() == right.as_bytes()
}

pub mod prelude {
    pub use crate::{
        oop_class, super_call, MethodEntry, MethodFn, MethodTable, OopClass, OopObject,
    };
}

#[macro_export]
macro_rules! super_call {
    ($($tokens:tt)*) => {
        compile_error!("super_call! can only be used inside methods declared in oop_class!");
    };
}
