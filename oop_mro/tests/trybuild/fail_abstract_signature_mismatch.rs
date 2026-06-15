use oop_mro::prelude::*;

oop_class! {
    abstract class AbstractShape {
        abstract virtual fn area(&self) -> usize;
    }

    class BadShape: AbstractShape {
        #[override]
        virtual fn area(&self) -> String {
            String::new()
        }
    }
}

fn main() {}
