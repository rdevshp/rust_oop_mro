use oop_mro::prelude::*;

oop_class! {
    class Animal {
        virtual fn speak(&self) -> String {
            String::new()
        }
    }

    class Dog: Animal {
        virtual fn speak(&self) -> String {
            "woof".into()
        }
    }
}

fn main() {}
