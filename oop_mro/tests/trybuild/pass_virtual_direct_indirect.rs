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

    class Branch: virtual Root {
        constructor(): Root(1) {}
    }

    class Diamond: virtual Root, Branch {
        constructor(): Root(5), Branch() {}

        #[override]
        virtual fn dispatched(&self) -> usize {
            self.as_base::<Root>().value() + 20
        }
    }
}

fn main() {
    let mut diamond = Diamond::new();

    assert!(core::ptr::eq(
        diamond.as_base::<Root>(),
        diamond.as_base::<Branch>().as_base::<Root>(),
    ));
    assert_eq!(diamond.as_base::<Root>().value(), 5);
    assert_eq!(diamond.as_base::<Root>().dispatched(), 25);

    diamond.as_base_mut::<Branch>().as_base_mut::<Root>().set_value(9);
    assert_eq!(diamond.as_base::<Root>().value(), 9);
    assert_eq!(diamond.as_base::<Root>().dispatched(), 29);
}
