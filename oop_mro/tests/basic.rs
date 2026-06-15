use oop_mro::prelude::*;
use std::future::Future;
use std::task::{Context, Poll, Waker};

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

    class UnsafeBase {
        unsafe fn direct_secret(&self) -> usize {
            3
        }

        virtual unsafe fn code(&self) -> usize {
            11
        }
    }

    class UnsafeChild: UnsafeBase {
        #[override]
        virtual unsafe fn code(&self) -> usize {
            29
        }
    }

    class AsyncRoot {
        label: String,

        async fn direct_score(&self, label: &str) -> usize {
            label.len() + 1
        }

        async fn direct_label(&self) -> &str {
            &self.label
        }

        virtual async fn score(&self, label: &str) -> usize {
            label.len()
        }

        virtual async fn label_ref(&self) -> &str {
            &self.label
        }
    }

    class AsyncLeaf: AsyncRoot {
        #[override]
        virtual async fn score(&self, label: &str) -> usize {
            super_call!(AsyncRoot::score, self, label).await + 10
        }
    }

    abstract class AsyncAbstract {
        abstract virtual async fn load(&self) -> usize;
    }

    class AsyncConcrete: AsyncAbstract {
        #[override]
        virtual async fn load(&self) -> usize {
            33
        }
    }

    abstract class GenericSlot<T> {
        abstract virtual fn get(&self) -> &T;

        fn passthrough<U>(&self, value: U) -> U {
            value
        }

        fn cloned(&self, value: T) -> T
        where
            T: Clone,
        {
            value.clone()
        }
    }

    class GenericLeaf<U>: GenericSlot<U> where U: Default {
        value: U,

        constructor(value: U): GenericSlot() {
            self.value = value;
        }

        #[override]
        virtual fn get(&self) -> &U {
            &self.value
        }
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);

    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
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
#[allow(clippy::assertions_on_constants)]
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
#[allow(clippy::assertions_on_constants)]
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

#[test]
fn supports_unsafe_direct_and_virtual_methods() {
    let child = UnsafeChild::default();

    unsafe {
        assert_eq!(child.as_unsafe_base().direct_secret(), 3);
        assert_eq!(child.code(), 29);
        assert_eq!(child.as_unsafe_base().code(), 29);
    }
}

#[test]
fn supports_async_direct_and_virtual_methods() {
    let leaf = AsyncLeaf::default();
    let concrete = AsyncConcrete::default();
    let roots: Vec<&AsyncRoot> = vec![leaf.as_async_root()];

    assert_eq!(block_on(leaf.score("abc")), 13);
    assert_eq!(block_on(leaf.as_async_root().score("abcd")), 14);
    assert_eq!(block_on(leaf.as_async_root().direct_score("xy")), 3);
    assert_eq!(block_on(leaf.as_async_root().direct_label()), "");
    assert_eq!(block_on(leaf.as_async_root().label_ref()), "");
    assert_eq!(block_on(roots[0].score("hello")), 15);
    assert_eq!(block_on(concrete.as_async_abstract().load()), 33);
}

#[test]
fn supports_generic_classes_and_base_views() {
    let leaf = GenericLeaf::new("leaf".to_string());
    let slots: Vec<Box<dyn AsGenericSlot<String>>> =
        vec![Box::new(GenericLeaf::new("boxed".to_string()))];

    assert_eq!(leaf.get(), "leaf");
    assert_eq!(leaf.as_generic_slot().get(), "leaf");
    assert_eq!(leaf.as_generic_slot().passthrough(42usize), 42);
    assert_eq!(leaf.as_generic_slot().cloned("clone".to_string()), "clone");
    assert_eq!(slots[0].as_generic_slot().get(), "boxed");
}
