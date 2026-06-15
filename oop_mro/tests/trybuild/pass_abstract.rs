use oop_mro::prelude::*;

oop_class! {
    abstract class AbstractShape {
        abstract virtual fn area(&self) -> usize;
    }

    class Square: AbstractShape {
        #[override]
        virtual fn area(&self) -> usize {
            16
        }
    }
}

fn main() {
    assert!(<AbstractShape as OopClass>::IS_ABSTRACT);
    assert!(!<Square as OopClass>::IS_ABSTRACT);
    assert_eq!(Square::default().area(), 16);
}
