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
            println!("{}", format!("{} speaking", self.identity()));
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

    assert_eq!(<Document as OopClass>::MRO, &["Document", "Named", "Tagged", "Entity"]);
    println!("{}", Document::default().describe()); // Named -> Entity
    assert_eq!(<Document2 as OopClass>::MRO, &["Document2", "Tagged", "Named", "Entity"]);
    println!("{}", Document2::default().describe()); // Tagged -> Entity

}
```