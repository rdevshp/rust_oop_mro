use oop_mro::prelude::*;

oop_class! {
    class A {
        virtual fn value(&self) -> i32 {
            1
        }
    }

    class B {
        virtual fn value(&self) -> String {
            String::new()
        }
    }

    class C: A, B {}
}

fn main() {}
