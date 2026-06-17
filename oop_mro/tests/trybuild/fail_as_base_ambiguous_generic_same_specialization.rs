use oop_mro::prelude::*;

oop_class! {
    class Slot<T> {
        value: Option<T> = None,
    }
    class Left: Slot<i32> {}
    class Right: Slot<i32> {}
    class Diamond: Left, Right {}
}

fn main() {
    let diamond = Diamond::default();
    let _ = diamond.as_base::<Slot<i32>>();
}
