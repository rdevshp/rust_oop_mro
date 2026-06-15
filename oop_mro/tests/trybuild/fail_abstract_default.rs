use oop_mro::prelude::*;

oop_class! {
    abstract class AbstractShape {
        abstract virtual fn area(&self) -> usize;
    }

    abstract class StillAbstract: AbstractShape {}
}

fn main() {
    let _ = StillAbstract::default();
}
