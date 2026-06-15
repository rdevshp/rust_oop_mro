use oop_mro::prelude::*;

oop_class! {
    class Animal {
        events: Vec<String>,

        constructor(label: String) {
            self.record(label);
        }

        virtual fn record(&mut self, label: String) {
            self.events.push(format!("animal:{label}"));
        }

        fn events(&self) -> Vec<String> {
            self.events.clone()
        }
    }

    class Dog: Animal {
        constructor(): Animal("base".into()) {
            self.record("derived".into());
        }

        #[override]
        virtual fn record(&mut self, label: String) {
            self.as_animal_mut().events.push(format!("dog:{label}"));
        }
    }
}

fn main() {
    let dog = Dog::new();
    assert_eq!(
        dog.as_animal().events(),
        vec!["dog:base".to_string(), "dog:derived".to_string()]
    );
}
