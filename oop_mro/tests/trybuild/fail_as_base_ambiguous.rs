use oop_mro::prelude::*;

oop_class! {
    class Root {}
    class Left: Root {}
    class Right: Root {}
    class Diamond: Left, Right {}
}

fn main() {
    let diamond = Diamond::default();
    let _ = diamond.as_base::<Root>();
}
