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
    let base: Box<dyn AsClass<Base<String>>> = Box::new(Leaf::new("value".to_string()));
    let mid = match base.downcast::<dyn AsClass<Mid<String>>>() {
        Ok(mid) => mid,
        Err(_) => panic!("Leaf should downcast through renamed generic parameter"),
    };
    assert_eq!(mid.as_base::<Mid<String>>().get(), "value");
}
