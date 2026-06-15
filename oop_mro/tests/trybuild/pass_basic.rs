use oop_mro::prelude::*;

oop_class! {
    class Base {
        virtual fn value(&self) -> i32 {
            1
        }
    }

    class Derived: Base {}
}

fn main() {
    let derived = Derived::default();
    assert_eq!(derived.value(), 1);
    assert_eq!(<Derived as OopClass>::MRO, &["Derived", "Base"]);
}
