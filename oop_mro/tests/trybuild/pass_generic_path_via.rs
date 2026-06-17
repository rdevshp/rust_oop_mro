use oop_mro::prelude::*;

oop_class! {
    class Root {
        label: &'static str,

        constructor(label: &'static str) {
            self.label = label;
        }

        virtual fn label(&self) -> &'static str {
            self.label
        }
    }

    class Left: Root {
        constructor(): Root("left") {}
    }

    class Right: Root {
        constructor(): Root("right") {}
    }

    class Branch<T>: Left, Right {
        _marker: core::marker::PhantomData<T> = core::marker::PhantomData,

        constructor(): Left(), Right() {}
    }

    class Nested<T>: Branch<T> {
        constructor(): Branch() {}
    }
}

fn borrowed_generic_path<T>() {
    let nested = Nested::<T>::new();
    let left = nested.as_base_via::<(Branch<T>, Left), Root>();
    let right = nested.as_base_via::<(Branch<T>, Right), Root>();

    assert_eq!(left.label(), "left");
    assert_eq!(right.label(), "right");
    assert!(left.downcast_ref::<Left>().is_some());
    assert!(left.downcast_ref::<Right>().is_none());
    assert!(right.downcast_ref::<Right>().is_some());
    assert!(right.downcast_ref::<Left>().is_none());
    assert!(right.downcast_ref::<Branch<T>>().is_some());
    assert!(right.downcast_ref::<Nested<T>>().is_some());
}

fn main() {
    borrowed_generic_path::<u8>();

    let root: Box<dyn AsRoot> =
        Box::new(Nested::<u8>::new()).into_base_via::<(Branch<u8>, Right), dyn AsRoot>();
    assert_eq!(root.as_base::<Root>().label(), "right");

    let root = match root.downcast::<dyn AsLeft>() {
        Ok(_) => panic!("right path should not downcast to left"),
        Err(root) => root,
    };
    let branch = match root.downcast::<dyn AsBranch<u8>>() {
        Ok(branch) => branch,
        Err(_) => panic!("right path should downcast to branch"),
    };
    let nested = match branch.downcast::<dyn AsNested<u8>>() {
        Ok(nested) => nested,
        Err(_) => panic!("branch path should downcast to complete nested object"),
    };

    assert_eq!(
        nested
            .as_base::<Nested<u8>>()
            .as_base_via::<(Branch<u8>, Right), Root>()
            .label(),
        "right"
    );
}
