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

    class Walker {
        fn legs(&self) -> usize {
            2
        }
    }

    class Kangaroo: Mammal, Walker {
        #[override]
        virtual fn speak(&self) -> &'static str {
            "chuff"
        }
    }
}

fn main() {
    let animal: Box<dyn AsAnimal> = Box::new(Dog::default());
    assert_eq!(animal.as_base::<Animal>().speak(), "woof");

    let walker: Box<dyn AsWalker> = Box::new(Kangaroo::default());
    assert_eq!(walker.as_base::<Walker>().legs(), 2);
}
