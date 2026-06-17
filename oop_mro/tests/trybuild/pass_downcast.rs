use oop_mro::prelude::*;

oop_class! {
    abstract class Animal {
        abstract virtual fn speak(&self) -> &'static str;
    }

    class Mammal: Animal {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "mammal"
        }
    }

    class Dog: Mammal {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "woof"
        }
    }

    class Cat: Animal {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "meow"
        }
    }

    class Walker {
        virtual fn walk(&self) -> &'static str {
            "walk"
        }
    }

    class Kangaroo: Mammal, Walker {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "chuff"
        }
    }

    class Counter {
        value: usize,

        virtual fn inc(&mut self) -> usize {
            self.value += 1;
            self.value
        }
    }

    class LoudCounter: Counter {
        #[override]
        virtual fn inc(&mut self) -> usize {
            super_call!(Counter::inc, self) + 10
        }
    }

    abstract class Slot<T> {
        abstract virtual fn get(&self) -> &T;
    }

    class Leaf<T>: Slot<T> where T: Default {
        value: T,

        constructor(value: T): Slot() {
            self.value = value;
        }

        #[override]
        virtual fn get(&self) -> &T {
            &self.value
        }
    }
}

fn main() {
    let animal: Box<dyn AsAnimal> = Box::new(Dog::default());
    let mammal = match animal.downcast::<dyn AsMammal>() {
        Ok(mammal) => mammal,
        Err(_) => panic!("Dog should downcast to Mammal"),
    };
    assert_eq!(mammal.as_base::<Mammal>().speak(), "woof");

    let dog = match mammal.downcast::<dyn AsDog>() {
        Ok(dog) => dog,
        Err(_) => panic!("Dog should downcast to Dog"),
    };
    assert_eq!(dog.as_base::<Dog>().speak(), "woof");

    let animal: Box<dyn AsAnimal> = Box::new(Cat::default());
    let animal = match animal.downcast::<dyn AsDog>() {
        Ok(_) => panic!("Cat should not downcast to Dog"),
        Err(animal) => animal,
    };
    assert_eq!(animal.as_base::<Animal>().speak(), "meow");

    let walker: Box<dyn AsWalker> = Box::new(Kangaroo::default());
    let kangaroo = match walker.downcast::<dyn AsKangaroo>() {
        Ok(kangaroo) => kangaroo,
        Err(_) => panic!("Kangaroo should downcast from Walker"),
    };
    assert_eq!(kangaroo.as_base::<Kangaroo>().speak(), "chuff");

    let kangaroo = Kangaroo::default();
    assert_eq!(kangaroo.as_base::<Animal>().downcast_ref::<Mammal>().unwrap().speak(), "chuff");
    assert_eq!(kangaroo.as_base::<Walker>().downcast_ref::<Kangaroo>().unwrap().speak(), "chuff");

    let mut counter = LoudCounter::default();
    assert_eq!(
        counter
            .as_base_mut::<Counter>()
            .downcast_mut::<LoudCounter>()
            .unwrap()
            .inc(),
        11
    );

    let slot: Box<dyn AsSlot<String>> = Box::new(Leaf::new("value".to_string()));
    let leaf = match slot.downcast::<dyn AsLeaf<String>>() {
        Ok(leaf) => leaf,
        Err(_) => panic!("Slot should downcast to Leaf"),
    };
    assert_eq!(leaf.as_base::<Leaf<String>>().get(), "value");
}
