This is an experimental project doing multiple inheritance OOP using C3 MRO in Rust that is co-developed with Codex.

This project supports virtual/non-virtual inheritance (mixed virtual and non-virtual inheritance is also supported), abstract methods/classes, generic classes, virtual/non-virtual methods, static/const class items, class functions, unsafe/async methods, const functions (const functions cannot be virtual), up/down casts (unambiguous upcasts specify the target base type, and ambiguous upcasts specify the path to the base class).

Mandatory `#[override]` attributes are required when a method overrides a superclass method.

```rust
use oop_mro::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

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

        constructor(): Animal(), Serializable() {
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

        constructor(name: String): Mammal() {
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

        constructor(name: String): Mammal() {
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

fn animal_examples() {
    let dog = Dog::new(String::from("Dog1"));
    let kangaroo = Kangaroo::new(String::from("Kangaroo1"));
    let v: Vec<&Mammal> = vec![dog.as_base::<Mammal>(), kangaroo.as_base::<Mammal>()];
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
        let animal: &Animal = object.as_base::<Animal>();
        println!("{}", animal.name());

        // checked downcast
        match animal.downcast_ref::<Dog>() {
            Some(dog) => {
                println!("{} is a dog!", dog.name());
            }
            None => {
                println!("{} is not a dog!", animal.name());
            }
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

fn document_examples() {
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
}

oop_class! {
    class PathExampleRoot {
        label: &'static str,

        constructor(label: &'static str) {
            self.label = label;
        }

        virtual fn label(&self) -> &'static str {
            self.label
        }
    }

    class PathExampleLeft: PathExampleRoot {
        constructor(): PathExampleRoot("left-root") {}
    }

    class PathExampleRight: PathExampleRoot {
        constructor(): PathExampleRoot("right-root") {}
    }

    class PathExampleDiamond: PathExampleLeft, PathExampleRight {
        constructor(): PathExampleLeft(), PathExampleRight() {}
    }

    class PathExampleBranch: PathExampleLeft, PathExampleRight {
        constructor(): PathExampleLeft(), PathExampleRight() {}
    }

    class PathExampleNested: PathExampleBranch {
        constructor(): PathExampleBranch() {}
    }

    class PathExampleGenericBranch<T>: PathExampleLeft, PathExampleRight {
        _marker: core::marker::PhantomData<T> = core::marker::PhantomData,

        constructor(): PathExampleLeft(), PathExampleRight() {}
    }

    class PathExampleGenericNested<T>: PathExampleGenericBranch<T> {
        constructor(): PathExampleGenericBranch() {}
    }
}

fn path_upcast_and_downcast_examples() {
    let diamond = PathExampleDiamond::new();
    let left_root = diamond.as_base_via::<PathExampleLeft, PathExampleRoot>();
    let right_root = diamond.as_base_via::<PathExampleRight, PathExampleRoot>();

    assert_eq!(left_root.label(), "left-root");
    assert_eq!(right_root.label(), "right-root");
    assert!(left_root.downcast_ref::<PathExampleLeft>().is_some());
    assert!(left_root.downcast_ref::<PathExampleRight>().is_none());
    assert!(right_root.downcast_ref::<PathExampleRight>().is_some());
    assert!(right_root.downcast_ref::<PathExampleLeft>().is_none());

    let root: Box<dyn AsPathExampleRoot> = Box::new(PathExampleDiamond::new())
        .into_base_via::<PathExampleRight, dyn AsPathExampleRoot>();
    assert_eq!(root.as_base::<PathExampleRoot>().label(), "right-root");

    let root = match root.downcast::<dyn AsPathExampleLeft>() {
        Ok(_) => panic!("right root should not downcast to left path"),
        Err(root) => root,
    };
    let right = match root.downcast::<dyn AsPathExampleRight>() {
        Ok(right) => right,
        Err(_) => panic!("right root should downcast to right path"),
    };
    let diamond = match right.downcast::<dyn AsPathExampleDiamond>() {
        Ok(diamond) => diamond,
        Err(_) => panic!("right path should downcast to complete diamond"),
    };

    assert_eq!(
        diamond
            .as_base::<PathExampleDiamond>()
            .as_base_via::<PathExampleLeft, PathExampleRoot>()
            .label(),
        "left-root",
    );

    long_path_upcast_and_downcast_example();
    generic_long_path_upcast_and_downcast_example::<u8>();
    concrete_generic_owned_long_path_upcast_and_downcast_example();
    println!("path upcast/downcast examples passed");
}

fn long_path_upcast_and_downcast_example() {
    let nested = PathExampleNested::new();
    let nested_left_root =
        nested.as_base_via::<(PathExampleBranch, PathExampleLeft), PathExampleRoot>();
    let nested_right_root =
        nested.as_base_via::<(PathExampleBranch, PathExampleRight), PathExampleRoot>();

    assert_eq!(nested_left_root.label(), "left-root");
    assert_eq!(nested_right_root.label(), "right-root");
    assert!(nested_left_root.downcast_ref::<PathExampleLeft>().is_some());
    assert!(nested_left_root
        .downcast_ref::<PathExampleRight>()
        .is_none());
    assert!(nested_left_root
        .downcast_ref::<PathExampleBranch>()
        .is_some());
    assert!(nested_left_root
        .downcast_ref::<PathExampleNested>()
        .is_some());
    assert!(nested_right_root
        .downcast_ref::<PathExampleRight>()
        .is_some());
    assert!(nested_right_root
        .downcast_ref::<PathExampleLeft>()
        .is_none());
    assert!(nested_right_root
        .downcast_ref::<PathExampleBranch>()
        .is_some());
    assert!(nested_right_root
        .downcast_ref::<PathExampleNested>()
        .is_some());

    let root: Box<dyn AsPathExampleRoot> =
        Box::new(PathExampleNested::new())
            .into_base_via::<(PathExampleBranch, PathExampleRight), dyn AsPathExampleRoot>();
    assert_eq!(root.as_base::<PathExampleRoot>().label(), "right-root");

    let root = match root.downcast::<dyn AsPathExampleLeft>() {
        Ok(_) => panic!("right nested root should not downcast to left path"),
        Err(root) => root,
    };
    let branch = match root.downcast::<dyn AsPathExampleBranch>() {
        Ok(branch) => branch,
        Err(_) => panic!("right nested root should downcast to branch"),
    };
    let nested = match branch.downcast::<dyn AsPathExampleNested>() {
        Ok(nested) => nested,
        Err(_) => panic!("branch path should downcast to complete nested object"),
    };

    assert_eq!(
        nested
            .as_base::<PathExampleNested>()
            .as_base_via::<(PathExampleBranch, PathExampleRight), PathExampleRoot>()
            .label(),
        "right-root",
    );
}

fn generic_long_path_upcast_and_downcast_example<T>() {
    let nested = PathExampleGenericNested::<T>::new();
    let left_root =
        nested.as_base_via::<(PathExampleGenericBranch<T>, PathExampleLeft), PathExampleRoot>();
    let right_root =
        nested.as_base_via::<(PathExampleGenericBranch<T>, PathExampleRight), PathExampleRoot>();

    assert_eq!(left_root.label(), "left-root");
    assert_eq!(right_root.label(), "right-root");
    assert!(left_root.downcast_ref::<PathExampleLeft>().is_some());
    assert!(left_root.downcast_ref::<PathExampleRight>().is_none());
    assert!(left_root
        .downcast_ref::<PathExampleGenericBranch<T>>()
        .is_some());
    assert!(left_root
        .downcast_ref::<PathExampleGenericNested<T>>()
        .is_some());
    assert!(right_root.downcast_ref::<PathExampleRight>().is_some());
    assert!(right_root.downcast_ref::<PathExampleLeft>().is_none());
    assert!(right_root
        .downcast_ref::<PathExampleGenericBranch<T>>()
        .is_some());
    assert!(right_root
        .downcast_ref::<PathExampleGenericNested<T>>()
        .is_some());
}

fn concrete_generic_owned_long_path_upcast_and_downcast_example() {
    let root: Box<dyn AsPathExampleRoot> = Box::new(PathExampleGenericNested::<u8>::new())
        .into_base_via::<(PathExampleGenericBranch<u8>, PathExampleRight), dyn AsPathExampleRoot>(
    );
    assert_eq!(root.as_base::<PathExampleRoot>().label(), "right-root");

    let root = match root.downcast::<dyn AsPathExampleLeft>() {
        Ok(_) => panic!("generic right nested root should not downcast to left path"),
        Err(root) => root,
    };
    let generic_branch = match root.downcast::<dyn AsPathExampleGenericBranch<u8>>() {
        Ok(generic_branch) => generic_branch,
        Err(_) => panic!("generic right nested root should downcast to generic branch"),
    };
    let generic_nested = match generic_branch.downcast::<dyn AsPathExampleGenericNested<u8>>() {
        Ok(generic_nested) => generic_nested,
        Err(_) => panic!("generic branch should downcast to complete nested object"),
    };

    assert_eq!(
        generic_nested
            .as_base::<PathExampleGenericNested<u8>>()
            .as_base_via::<(PathExampleGenericBranch<u8>, PathExampleRight), PathExampleRoot>()
            .label(),
        "right-root",
    );
}

oop_class! {
    class VirtualExampleRoot {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }

        fn set_value(&mut self, value: usize) {
            self.value = value;
        }

        virtual fn dispatched_value(&self) -> usize {
            self.value
        }
    }

    class VirtualExampleLeft: virtual VirtualExampleRoot {
        constructor(): VirtualExampleRoot(1) {}
    }

    class VirtualExampleRight: virtual VirtualExampleRoot {
        constructor(): VirtualExampleRoot(2) {}
    }

    class VirtualExampleDiamond: VirtualExampleLeft, VirtualExampleRight {
        constructor(): VirtualExampleRoot(10), VirtualExampleLeft(), VirtualExampleRight() {}

        #[override]
        virtual fn dispatched_value(&self) -> usize {
            self.as_base::<VirtualExampleRoot>().value() + 100
        }
    }
}

fn virtual_inheritance_examples() {
    let mut diamond = VirtualExampleDiamond::new();

    assert!(core::ptr::eq(
        diamond.as_base::<VirtualExampleLeft>().as_base::<VirtualExampleRoot>(),
        diamond.as_base::<VirtualExampleRight>().as_base::<VirtualExampleRoot>(),
    ));
    assert!(core::ptr::eq(
        diamond.as_base::<VirtualExampleRoot>(),
        diamond.as_base::<VirtualExampleLeft>().as_base::<VirtualExampleRoot>(),
    ));
    assert_eq!(diamond.as_base::<VirtualExampleRoot>().value(), 10);
    assert_eq!(diamond.as_base::<VirtualExampleRoot>().dispatched_value(), 110);

    diamond
        .as_base_mut::<VirtualExampleRight>()
        .as_base_mut::<VirtualExampleRoot>()
        .set_value(33);
    assert_eq!(
        diamond
            .as_base::<VirtualExampleLeft>()
            .as_base::<VirtualExampleRoot>()
            .value(),
        33
    );
    assert_eq!(diamond.as_base::<VirtualExampleRoot>().dispatched_value(), 133);

    let root = diamond.as_base::<VirtualExampleRoot>();
    assert!(root.downcast_ref::<VirtualExampleLeft>().is_some());
    assert!(root.downcast_ref::<VirtualExampleRight>().is_some());
    assert!(root.downcast_ref::<VirtualExampleDiamond>().is_some());

    diamond
        .as_base_mut::<VirtualExampleRoot>()
        .downcast_mut::<VirtualExampleDiamond>()
        .expect("virtual root should downcast mutably to complete diamond")
        .as_base_mut::<VirtualExampleRoot>()
        .set_value(41);
    assert_eq!(diamond.as_base::<VirtualExampleRoot>().value(), 41);

    let root: Box<dyn AsVirtualExampleRoot> = Box::new(VirtualExampleDiamond::new());
    assert_eq!(root.as_base::<VirtualExampleRoot>().dispatched_value(), 110);

    let left = match root.downcast::<dyn AsVirtualExampleLeft>() {
        Ok(left) => left,
        Err(_) => panic!("virtual root should downcast to left branch"),
    };
    assert_eq!(
        left.as_base::<VirtualExampleLeft>()
            .as_base::<VirtualExampleRoot>()
            .value(),
        10
    );

    let diamond = match left.downcast::<dyn AsVirtualExampleDiamond>() {
        Ok(diamond) => diamond,
        Err(_) => panic!("left branch should downcast to complete diamond"),
    };
    assert_eq!(
        diamond
            .as_base::<VirtualExampleDiamond>()
            .as_base::<VirtualExampleRoot>()
            .dispatched_value(),
        110
    );
    println!("virtual inheritance examples passed");
}

oop_class! {
    class MixedExampleRoot {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        virtual fn value(&self) -> usize {
            self.value
        }

        fn set_value(&mut self, value: usize) {
            self.value = value;
        }
    }

    class MixedExampleVirtualBranch: virtual MixedExampleRoot {
        constructor(): MixedExampleRoot(1) {}
    }

    class MixedExampleConcreteBranch: MixedExampleRoot {
        constructor(): MixedExampleRoot(2) {}
    }

    class MixedExampleDiamond: MixedExampleVirtualBranch, MixedExampleConcreteBranch {
        constructor(): MixedExampleRoot(10), MixedExampleVirtualBranch(), MixedExampleConcreteBranch() {}
    }
}

fn mixed_inheritance_path_cast_examples() {
    let mut diamond = MixedExampleDiamond::new();
    let virtual_root = diamond.as_base_via::<MixedExampleVirtualBranch, MixedExampleRoot>();
    let concrete_root = diamond.as_base_via::<MixedExampleConcreteBranch, MixedExampleRoot>();

    assert_ne!(
        virtual_root as *const MixedExampleRoot,
        concrete_root as *const MixedExampleRoot,
    );
    assert_eq!(virtual_root.value(), 10);
    assert_eq!(concrete_root.value(), 2);

    assert!(virtual_root
        .downcast_ref::<MixedExampleVirtualBranch>()
        .is_some());
    assert!(virtual_root
        .downcast_ref::<MixedExampleConcreteBranch>()
        .is_none());
    assert!(concrete_root
        .downcast_ref::<MixedExampleConcreteBranch>()
        .is_some());
    assert!(concrete_root
        .downcast_ref::<MixedExampleVirtualBranch>()
        .is_none());
    assert!(virtual_root.downcast_ref::<MixedExampleDiamond>().is_some());
    assert!(concrete_root
        .downcast_ref::<MixedExampleDiamond>()
        .is_some());

    diamond
        .as_base_via_mut::<MixedExampleConcreteBranch, MixedExampleRoot>()
        .set_value(22);
    diamond
        .as_base_via_mut::<MixedExampleVirtualBranch, MixedExampleRoot>()
        .set_value(33);
    assert_eq!(
        diamond
            .as_base_via::<MixedExampleVirtualBranch, MixedExampleRoot>()
            .value(),
        33
    );
    assert_eq!(
        diamond
            .as_base_via::<MixedExampleConcreteBranch, MixedExampleRoot>()
            .value(),
        22
    );

    let diamond_trait: &dyn AsMixedExampleDiamond = &diamond;
    assert_eq!(
        diamond_trait
            .as_base_via::<MixedExampleVirtualBranch, MixedExampleRoot>()
            .value(),
        33
    );
    assert_eq!(
        diamond_trait
            .as_base_via::<MixedExampleConcreteBranch, MixedExampleRoot>()
            .value(),
        22
    );

    let root: Box<dyn AsMixedExampleRoot> = Box::new(MixedExampleDiamond::new())
        .into_base_via::<MixedExampleConcreteBranch, dyn AsMixedExampleRoot>(
    );
    assert_eq!(root.as_base::<MixedExampleRoot>().value(), 2);

    let root = match root.downcast::<dyn AsMixedExampleVirtualBranch>() {
        Ok(_) => panic!("concrete root path should not downcast to virtual branch"),
        Err(root) => root,
    };
    let concrete = match root.downcast::<dyn AsMixedExampleConcreteBranch>() {
        Ok(concrete) => concrete,
        Err(_) => panic!("concrete root path should downcast to concrete branch"),
    };
    let diamond = match concrete.downcast::<dyn AsMixedExampleDiamond>() {
        Ok(diamond) => diamond,
        Err(_) => panic!("concrete branch should downcast to complete diamond"),
    };
    assert_eq!(
        diamond
            .as_base::<MixedExampleDiamond>()
            .as_base_via::<MixedExampleConcreteBranch, MixedExampleRoot>()
            .value(),
        2
    );

    let root: Box<dyn AsMixedExampleRoot> = Box::new(MixedExampleDiamond::new())
        .into_base_via::<MixedExampleVirtualBranch, dyn AsMixedExampleRoot>(
    );
    assert_eq!(root.as_base::<MixedExampleRoot>().value(), 10);

    let root = match root.downcast::<dyn AsMixedExampleConcreteBranch>() {
        Ok(_) => panic!("virtual root path should not downcast to concrete branch"),
        Err(root) => root,
    };
    let virtual_branch = match root.downcast::<dyn AsMixedExampleVirtualBranch>() {
        Ok(virtual_branch) => virtual_branch,
        Err(_) => panic!("virtual root path should downcast to virtual branch"),
    };
    assert_eq!(
        virtual_branch
            .as_base::<MixedExampleVirtualBranch>()
            .as_base::<MixedExampleRoot>()
            .value(),
        10
    );
    println!("mixed inheritance path cast examples passed");
}

oop_class! {
    class Test {
        virtual async unsafe fn f(&self) {}
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct Job {
    id: u32,
}
oop_class! {
    abstract class Factory<T> {
        abstract virtual fn create(&mut self) -> T;
    }
    class JobFactory: Factory<Job> {
        id: u32 = 50,
        constructor(): Factory() {}
        #[override]
        virtual fn create(&mut self) -> Job {
            let r = self.id;
            self.id += 1;
            Job { id: r }
        }
    }
}

fn job_factory_examples() -> JobFactory {
    let mut job_factory = JobFactory::new();
    println!("job id: {:?} ", job_factory.create());
    println!("job id: {:?} ", job_factory.create());
    job_factory
}

fn job_factory_downcast_example(job_factory: JobFactory) {
    // checked downcast
    let factory: Box<dyn AsFactory<Job>> = Box::new(job_factory);
    let factory_downcast_result = factory.downcast::<dyn AsJobFactory>();
    match factory_downcast_result {
        Ok(mut job_factory_downcast) => {
            println!("downcasted to job_factory");
            println!(
                "job_factory_downcast job id: {:?}:",
                job_factory_downcast.as_base_mut::<JobFactory>().create()
            );
        }
        Err(_) => {
            println!("failed to downcast factory");
        }
    }
}

oop_class! {
    class TicketRegistry {
        pub const PREFIX: &'static str = "ticket";
        const FIRST_ID: usize = 1000;
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1000);

        fn reset_for_example() {
            Self::NEXT_ID.store(Self::FIRST_ID, Ordering::Relaxed);
        }

        fn next_id() -> usize {
            Self::NEXT_ID.fetch_add(1, Ordering::Relaxed)
        }

        fn next_label() -> String {
            format!("{}-{}", Self::PREFIX, Self::next_id())
        }
    }
}

fn ticket_registry_examples() {
    TicketRegistry::reset_for_example();
    assert_eq!(TicketRegistry::PREFIX, "ticket");
    assert_eq!(TicketRegistry::next_label(), "ticket-1000");
    assert_eq!(TicketRegistry::next_id(), 1001);
    assert_eq!(TicketRegistry::NEXT_ID.load(Ordering::Relaxed), 1002);
    println!("next ticket: {}", TicketRegistry::next_label());
}

fn main() {
    animal_examples();
    document_examples();
    path_upcast_and_downcast_examples();
    virtual_inheritance_examples();
    mixed_inheritance_path_cast_examples();
    let job_factory = job_factory_examples();
    ticket_registry_examples();
    job_factory_downcast_example(job_factory);
}
```
