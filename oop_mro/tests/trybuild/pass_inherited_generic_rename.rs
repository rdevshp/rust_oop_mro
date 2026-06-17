use oop_mro::prelude::*;

oop_class! {
    abstract class Base<T> {
        abstract virtual fn get(&self) -> &T;
    }

    class Mid<U>: Base<U> where U: Default {
        value: U,

        constructor(value: U): Base() {
            self.value = value;
        }

        #[override]
        virtual fn get(&self) -> &U {
            &self.value
        }
    }

    class Leaf<V>: Mid<V> where V: Default {
        constructor(value: V): Mid(value) {}
    }
}

fn main() {
    let leaf = Leaf::new("value".to_string());
    assert_eq!(leaf.as_base::<Base<String>>().get(), "value");
}
