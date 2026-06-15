use oop_mro::prelude::*;

oop_class! {
    class Animal {
        #[override]
        virtual fn speak(&self) -> String {
            String::new()
        }
    }
}

fn main() {}
