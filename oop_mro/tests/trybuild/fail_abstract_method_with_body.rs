use oop_mro::prelude::*;

oop_class! {
    abstract class Shape {
        abstract virtual fn area(&self) -> usize {
            0
        }
    }
}

fn main() {}
