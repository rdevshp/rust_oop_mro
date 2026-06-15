use oop_mro::prelude::*;

oop_class! {
    class Bad {
        virtual fn identity<T>(&self, value: T) -> T {
            value
        }
    }
}

fn main() {}
