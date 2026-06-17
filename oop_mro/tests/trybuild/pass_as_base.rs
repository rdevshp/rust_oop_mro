use oop_mro::prelude::*;

oop_class! {
    class Animal {
        virtual fn speak(&self) -> &'static str {
            "animal"
        }
    }

    class Walker {
        fn legs(&self) -> usize {
            4
        }
    }

    class Dog: Animal, Walker {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "woof"
        }
    }

    class Base {
        value: usize,

        fn set_value(&mut self, value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }
    }

    class Derived: Base {}

    abstract class Repository<T> {
        abstract virtual fn current(&self) -> &T;
    }

    class MemoryRepository<Item>: Repository<Item> where Item: Default {
        value: Item,

        constructor(value: Item): Repository() {
            self.value = value;
        }

        #[override]
        virtual fn current(&self) -> &Item {
            &self.value
        }
    }
}

fn main() {
    let dog = Dog::default();
    assert_eq!(dog.as_base::<Animal>().speak(), "woof");
    assert_eq!(dog.as_base::<Walker>().legs(), 4);

    let animal: Box<dyn AsAnimal> = Box::new(Dog::default());
    assert_eq!(animal.as_base::<Animal>().speak(), "woof");

    let mut derived = Derived::default();
    derived.as_base_mut::<Base>().set_value(42);
    assert_eq!(derived.as_base::<Base>().value(), 42);

    let repository = MemoryRepository::new(String::from("stored"));
    assert_eq!(repository.as_base::<Repository<String>>().current(), "stored");

    let owned: Box<dyn AsRepository<String>> =
        Box::new(MemoryRepository::new(String::from("boxed")));
    assert_eq!(owned.as_base::<Repository<String>>().current(), "boxed");
}
