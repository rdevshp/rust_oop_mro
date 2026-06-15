use oop_mro::prelude::*;

oop_class! {
    abstract class Animal {
        abstract virtual fn speak(&self) -> String;
    }

    class Dog: Animal {
        #[override]
        virtual fn speak(&self) -> String {
            "woof".into()
        }
    }

    class Cat: Animal {
        #[override]
        virtual fn speak(&self) -> String {
            "meow".into()
        }
    }
}

fn main() {
    let dog = Dog::default();
    let cat = Cat::default();
    let animals: Vec<&Animal> = vec![dog.as_animal(), cat.as_animal()];
    let sounds: Vec<_> = animals.iter().map(|animal| animal.speak()).collect();

    assert_eq!(sounds, ["woof", "meow"]);
}
