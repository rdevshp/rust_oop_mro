use oop_mro::prelude::*;

oop_class! {
    class Animal {}
    class Walker {}
    class Cat: Animal {}
}

fn main() {
    let cat = Cat::default();
    let _ = cat.as_walker();
}
