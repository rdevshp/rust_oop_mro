use oop_mro::prelude::*;

oop_class! {
    class Animal {
        #[override]
        fn speak(&self) -> String {
            String::new()
        }
    }
}

fn main() {}
