use oop_mro::prelude::*;

oop_class! {
    class Object {}
    class Animal: Object {}

    class Dog: Animal {
        constructor(): Object() {}
    }
}

fn main() {}
