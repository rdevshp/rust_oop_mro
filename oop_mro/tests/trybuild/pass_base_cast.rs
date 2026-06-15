use oop_mro::prelude::*;

oop_class! {
    class Base {
        value: usize,

        fn local(&self) -> usize {
            self.value
        }

        fn set_local(&mut self, value: usize) {
            self.value = value;
        }
    }

    class Derived: Base {}
}

fn main() {
    let mut derived = Derived::default();
    derived.as_base_mut().set_local(42);
    assert_eq!(derived.as_base().local(), 42);
}
