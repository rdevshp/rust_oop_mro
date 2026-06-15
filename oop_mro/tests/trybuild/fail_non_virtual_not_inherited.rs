use oop_mro::prelude::*;

oop_class! {
    class Base {
        fn local(&self) -> i32 {
            1
        }
    }

    class Derived: Base {}
}

fn main() {
    let derived = Derived::default();
    let _ = derived.local();
}
