This is an experimental project doing C3 MRO OOP in Rust that is co-developed with Codex.

```rust
use oop_mro::prelude::*;

oop_class! {
    abstract class Animal {
        abstract virtual fn typ(&self) -> &String;
        abstract virtual fn name(&self) -> &String;
        abstract virtual fn speak(&self);
        abstract virtual fn identity(&self) -> String;
    }
    abstract class Serializable {
        abstract virtual fn serialize(&self) -> String;
    }
    abstract class Mammal: Animal, Serializable {
        typ: String,

        constructor() {
            self.typ = String::from("mammal");
        }
        #[override]
        virtual fn typ(&self) -> &String {
            &self.typ
        }
        #[override]
        virtual fn speak(&self) {
            println!("{} speaking", self.identity());
        }
    }
    class Kangaroo: Mammal {
        name: String,

        constructor(name: String) {
            self.name = name;
        }
        #[override]
        virtual fn name(&self) -> &String {
            &self.name
        }
        #[override]
        virtual fn serialize(&self) -> String {
            String::new()
        }
        #[override]
        virtual fn identity(&self) -> String {
            String::from("Kangaroo")
        }
    }
    class Dog: Mammal {
        name: String,

        constructor(name: String) {
            self.name = name;
        }
        #[override]
        virtual fn name(&self) -> &String {
            &self.name
        }
        #[override]
        virtual fn serialize(&self) -> String {
            String::new()
        }
        #[override]
        virtual fn identity(&self) -> String{
            String::from("Dog")
        }
    }
}

oop_class! {
    class Entity {
        virtual fn describe(&self) -> String {
            "Entity".into()
        }
    }

    class Named: Entity {
        #[override]
        virtual fn describe(&self) -> String {
            format!("Named -> {}", super_call!(Entity::describe, self))
        }
    }

    class Tagged: Entity {
        #[override]
        virtual fn describe(&self) -> String {
            format!("Tagged -> {}", super_call!(Entity::describe, self))
        }
    }

    class Document: Named, Tagged {
    }

    class Document2: Tagged, Named {
    }
}

oop_class! {
    class Test {
        virtual async unsafe fn f(&self) {}
    }
}
#[derive(Debug)]
struct Job {
    id: u32
}
oop_class! {
    abstract class Factory<T> {
        abstract virtual fn create(&mut self) -> T;
    }
    class JobFactory: Factory<Job> {
        id: u32,
        constructor() {
            self.id = 0;
        }
        #[override]
        virtual fn create(&mut self) -> Job {
            let r = self.id;
            self.id += 1;
            Job { id: r }
        }
    }
}
fn main() {
    let dog = Dog::new(String::from("Dog1"));
    let kangaroo = Kangaroo::new(String::from("Kangaroo1"));
    let v: Vec<&Mammal> = vec![dog.as_mammal(), kangaroo.as_mammal()];
    for i in v {
        println!("type: {}, name: {}", i.typ(), i.name());
        i.speak();
        /*
        type: mammal, name: Dog1
        Dog speaking
        type: mammal, name: Kangaroo1
        Kangaroo speaking
        */
    }

    let animals: Vec<Box<dyn AsAnimal>> = vec![
        Box::new(Dog::new(String::from("Dog2"))),
        Box::new(Kangaroo::new(String::from("Kangaroo2"))),
    ];

    for object in &animals {
        let animal: &Animal = object.as_animal();
        println!("{}", animal.name());
    }

    assert_eq!(
        <Document as OopClass>::MRO,
        &["Document", "Named", "Tagged", "Entity"]
    );
    println!("{}", Document::default().describe()); // Named -> Entity
    assert_eq!(
        <Document2 as OopClass>::MRO,
        &["Document2", "Tagged", "Named", "Entity"]
    );
    println!("{}", Document2::default().describe()); // Tagged -> Entity
    let mut job_factory = JobFactory::new();
    println!("job id: {:?} ", job_factory.create());
    println!("job id: {:?} ", job_factory.create());

}
```