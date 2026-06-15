use oop_mro::prelude::*;

oop_class! {
    class Animal {
        virtual fn speak(&self) -> String {
            String::new()
        }
    }

    class BadDog: Animal {
        #[override]
        virtual fn speak(&self) -> usize {
            1
        }
    }
}

fn main() {}
