use oop_mro::prelude::*;

oop_class! {
    class A: B {}
    class B: A {}
}

fn main() {}
