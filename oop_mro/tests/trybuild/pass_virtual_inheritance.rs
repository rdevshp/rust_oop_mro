use oop_mro::prelude::*;

oop_class! {
    class Root {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }

        fn set_value(&mut self, value: usize) {
            self.value = value;
        }

        virtual fn dispatched(&self) -> usize {
            self.value
        }
    }

    class Left: virtual Root {
        constructor(): Root(1) {}
    }

    class Right: virtual Root {
        constructor(): Root(2) {}
    }

    class Diamond: Left, Right {
        constructor(): Root(3), Left(), Right() {}

        #[override]
        virtual fn dispatched(&self) -> usize {
            self.as_base::<Root>().value() + 10
        }
    }
}

fn main() {
    let mut diamond = Diamond::new();

    assert!(core::ptr::eq(
        diamond.as_base::<Left>().as_base::<Root>(),
        diamond.as_base::<Right>().as_base::<Root>(),
    ));
    assert_eq!(diamond.as_base::<Root>().value(), 3);
    diamond.as_base_mut::<Right>().as_base_mut::<Root>().set_value(4);
    assert_eq!(diamond.as_base::<Left>().as_base::<Root>().value(), 4);
    assert_eq!(diamond.as_base::<Root>().dispatched(), 14);
}
