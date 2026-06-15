use oop_mro::prelude::*;

oop_class! {
    abstract class Shape {
        virtual fn area(&self) -> usize;
    }
}

fn main() {}
