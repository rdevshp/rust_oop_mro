use oop_mro::prelude::*;

oop_class! {
    class Root<T> {
        marker: ::core::marker::PhantomData<T>,
    }

    class Left<T>: Root<T> {
        marker: ::core::marker::PhantomData<T>,
    }

    class Right<T>: Root<T> {
        marker: ::core::marker::PhantomData<T>,
    }

    class Branch<U>: Left<U>, Right<U> {
        marker: ::core::marker::PhantomData<U>,
    }

    class Leaf<V>: Branch<V> {
        marker: ::core::marker::PhantomData<V>,
    }
}

fn main() {
    let branch: Box<dyn AsClass<Branch<String>>> = Box::new(Leaf::<String>::default());
    let root = branch.into_base_via::<Right<String>, dyn AsClass<Root<String>>>();
    let _ = root.as_base::<Root<String>>();
}
