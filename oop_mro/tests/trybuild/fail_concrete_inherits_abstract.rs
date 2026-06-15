use oop_mro::prelude::*;

oop_class! {
    abstract class AbstractShape {
        abstract virtual fn area(&self) -> usize;
    }

    class StillAbstract: AbstractShape {}
}

fn main() {}
