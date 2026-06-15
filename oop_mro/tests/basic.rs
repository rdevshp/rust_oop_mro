use oop_mro::prelude::*;

oop_class! {
    class Animal {
        virtual fn speak(&self) -> String {
            "generic".into()
        }

        fn kingdom(&self) -> String {
            "animalia".into()
        }
    }

    class Walker {
        virtual fn walk(&self) -> String {
            "walking".into()
        }
    }

    class Dog: Animal, Walker {
        #[override]
        virtual fn speak(&self) -> String {
            format!("woof -> {}", super_call!(Animal::speak, self))
        }
    }

    class Cat: Animal {
        #[override]
        virtual fn speak(&self) -> String {
            "meow".into()
        }
    }

    class Object {
        virtual fn root(&self) -> String {
            "object".into()
        }

        virtual fn name(&self) -> String {
            "Object".into()
        }
    }

    class A: Object {
        #[override]
        virtual fn name(&self) -> String {
            "A".into()
        }
    }

    class B: Object {
        virtual fn label(&self) -> String {
            "B".into()
        }
    }

    class C: A, B {}

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

    abstract class Shape {
        abstract virtual fn area(&self) -> usize;

        virtual fn label(&self) -> String {
            "shape".into()
        }
    }

    class Square: Shape {
        #[override]
        virtual fn area(&self) -> usize {
            4
        }
    }

    abstract class AbstractDrawable {
        abstract virtual fn draw(&self) -> String;
    }

    class ConcreteDrawable {
        virtual fn draw(&self) -> String {
            "drawn".into()
        }
    }

    class Icon: AbstractDrawable, ConcreteDrawable {}

    class PlainBase {
        value: usize,

        fn local(&self) -> String {
            "local".into()
        }

        fn set_value(&mut self, value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }
    }

    class PlainChild: PlainBase {}

    class ConstructedAnimal {
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

    class ConstructedDog: ConstructedAnimal {
        constructor(): ConstructedAnimal("base".into()) {
            self.record("derived".into());
        }

        #[override]
        virtual fn record(&mut self, label: String) {
            self.as_constructed_animal_mut()
                .events
                .push(format!("dog:{label}"));
        }
    }
}

#[test]
fn dispatches_direct_and_inherited_methods() {
    let dog = Dog::default();
    let plain = PlainBase::default();

    assert_eq!(dog.speak(), "woof -> generic");
    assert_eq!(dog.walk(), "walking");
    assert_eq!(plain.local(), "local");
}

#[test]
fn casts_to_base_classes_for_non_virtual_methods() {
    let mut dog = Dog::default();
    let mut plain_child = PlainChild::default();

    assert_eq!(dog.as_animal().kingdom(), "animalia");
    assert_eq!(dog.as_animal().speak(), "woof -> generic");
    assert_eq!(dog.as_walker().walk(), "walking");

    plain_child.as_plain_base_mut().set_value(7);
    assert_eq!(plain_child.as_plain_base().local(), "local");
    assert_eq!(plain_child.as_plain_base().value(), 7);

    assert_eq!(dog.as_animal_mut().kingdom(), "animalia");
}

#[test]
fn supports_heterogeneous_base_collections_for_virtual_methods() {
    let dog = Dog::default();
    let cat = Cat::default();
    let animals: Vec<&Animal> = vec![dog.as_animal(), cat.as_animal()];
    let sounds: Vec<_> = animals.iter().map(|animal| animal.speak()).collect();

    assert_eq!(sounds, ["woof -> generic", "meow"]);
}

#[test]
fn base_references_dispatch_inherited_and_mutable_virtual_methods() {
    let dog = Dog::default();
    let walkers: Vec<&Walker> = vec![dog.as_walker()];
    assert_eq!(walkers[0].walk(), "walking");

    let mut counter = Counter::default();
    let mut loud_counter = LoudCounter::default();
    let mut counters: Vec<&mut Counter> = vec![&mut counter, loud_counter.as_counter_mut()];
    let counts: Vec<_> = counters
        .iter_mut()
        .map(|counter| (*counter).inc())
        .collect();

    assert_eq!(counts, [1, 11]);
}

#[test]
fn exposes_c3_metadata_and_uses_c3_for_forwarding() {
    let object = C::default();

    assert_eq!(<C as OopClass>::MRO, &["C", "A", "B", "Object"]);
    assert_eq!(object.name(), "A");
    assert_eq!(object.label(), "B");
    assert_eq!(object.root(), "object");
    assert_eq!(object.as_b().as_object().name(), "A");
}

#[test]
fn exposes_method_table_metadata() {
    let table = <Dog as OopClass>::METHOD_TABLE;

    assert_eq!(table.find("speak").unwrap().owner, "Dog");
    assert_eq!(table.find("walk").unwrap().owner, "Walker");
}

#[test]
fn supports_mutable_super_calls() {
    let mut counter = LoudCounter::default();

    assert_eq!(counter.inc(), 11);
    assert_eq!(counter.inc(), 12);
}

#[test]
fn supports_abstract_superclass_methods_with_concrete_overrides() {
    let square = Square::default();
    let shapes: Vec<&Shape> = vec![square.as_shape()];

    assert!(<Shape as OopClass>::IS_ABSTRACT);
    assert!(!<Square as OopClass>::IS_ABSTRACT);
    assert_eq!(<Shape as OopClass>::ABSTRACT_METHODS[0].name, "area");
    assert_eq!(square.area(), 4);
    assert_eq!(square.label(), "shape");
    assert_eq!(shapes[0].area(), 4);
    assert_eq!(shapes[0].label(), "shape");
}

#[test]
fn inherited_concrete_methods_can_satisfy_abstract_requirements() {
    let icon = Icon::default();
    let drawables: Vec<&AbstractDrawable> = vec![icon.as_abstract_drawable()];

    assert!(!<Icon as OopClass>::IS_ABSTRACT);
    assert_eq!(<Icon as OopClass>::ABSTRACT_METHODS.len(), 0);
    assert_eq!(icon.draw(), "drawn");
    assert_eq!(drawables[0].draw(), "drawn");
    assert_eq!(
        <Icon as OopClass>::METHOD_TABLE.find("draw").unwrap().owner,
        "ConcreteDrawable"
    );
}

#[test]
fn constructors_dispatch_virtual_methods_through_complete_object() {
    let dog = ConstructedDog::new();

    assert_eq!(
        dog.as_constructed_animal().events(),
        vec!["dog:base".to_string(), "dog:derived".to_string()]
    );
}
