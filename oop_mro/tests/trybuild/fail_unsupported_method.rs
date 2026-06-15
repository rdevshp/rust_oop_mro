use oop_mro::prelude::*;

oop_class! {
    class Animal {
        async fn speak(&self) -> String {
            "generic".into()
        }
    }
}

fn main() {}
