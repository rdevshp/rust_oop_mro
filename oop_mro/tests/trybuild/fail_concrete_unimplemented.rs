use oop_mro::prelude::*;

oop_class! {
    class Shape {
        abstract virtual fn area(&self) -> usize;
    }
}

fn main() {}
