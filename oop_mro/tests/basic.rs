use oop_mro::prelude::*;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

static VIRTUAL_OBJECT_DROPS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
struct NoDefault(usize);

struct VirtualDropToken;

impl Drop for VirtualDropToken {
    fn drop(&mut self) {
        VIRTUAL_OBJECT_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

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

    class Mammal: Animal {}

    class Dog: Animal, Walker {
        #[override]
        virtual fn speak(&self) -> String {
            format!("woof -> {}", super_call!(Animal::speak, self))
        }
    }

    class Kangaroo: Mammal, Walker {
        #[override]
        virtual fn speak(&self) -> String {
            "chuff".into()
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
            self.as_base_mut::<ConstructedAnimal>()
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

    class ConstValue<T: Default> {
        value: T,

        const fn get(&self) -> &T {
            &self.value
        }

        const fn passthrough<U>(&self, value: U) -> U {
            value
        }

        const unsafe fn unchecked(&self) -> &T {
            &self.value
        }
    }

    class StaticUtility {
        const DEFAULT: usize = 41;
        pub const PUBLIC: usize = 42;
        static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

        fn default_value() -> usize {
            Self::DEFAULT
        }

        fn public_value() -> usize {
            Self::PUBLIC
        }

        fn pair<T>(left: T, right: T) -> (T, T) {
            (left, right)
        }

        fn allocate_id() -> usize {
            Self::NEXT_ID.fetch_add(1, Ordering::Relaxed)
        }
    }

    class InitializedFields {
        id: NoDefault = NoDefault(7),
        label: String = String::from("ready"),
        numbers: Vec<usize> = vec![1, 2, 3],

        constructor() {
            self.numbers.push(4);
        }

        fn id(&self) -> usize {
            self.id.0
        }

        fn label(&self) -> &str {
            &self.label
        }

        fn numbers(&self) -> &[usize] {
            &self.numbers
        }
    }

    class VirtualObject {
        value: usize,
        drop_token: VirtualDropToken = VirtualDropToken,

        constructor(value: usize) {
            self.value = value;
        }

        fn raw_value(&self) -> usize {
            self.value
        }

        fn set_raw_value(&mut self, value: usize) {
            self.value = value;
        }

        virtual fn virtual_value(&self) -> usize {
            self.value
        }
    }

    class VirtualLeft: virtual VirtualObject {
        constructor(): VirtualObject(1) {}
    }

    class VirtualRight: virtual VirtualObject {
        constructor(): VirtualObject(2) {}
    }

    class VirtualDiamond: VirtualLeft, VirtualRight {
        constructor(): VirtualObject(10), VirtualLeft(), VirtualRight() {}

        #[override]
        virtual fn virtual_value(&self) -> usize {
            self.as_base::<VirtualObject>().raw_value() + 1000
        }
    }

    class DirectIndirectVirtualRoot {
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

        virtual fn dispatched(&self) -> usize {
            self.value
        }
    }

    class IndirectVirtualBranch: virtual DirectIndirectVirtualRoot {
        constructor(): DirectIndirectVirtualRoot(1) {}
    }

    class DirectIndirectVirtualDiamond: virtual DirectIndirectVirtualRoot, IndirectVirtualBranch {
        constructor(): DirectIndirectVirtualRoot(7), IndirectVirtualBranch() {}

        #[override]
        virtual fn dispatched(&self) -> usize {
            self.as_base::<DirectIndirectVirtualRoot>().value() + 100
        }
    }

    class MixedRoot {
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
    }

    class MixedVirtualLeft: virtual MixedRoot {
        constructor(): MixedRoot(1) {}
    }

    class MixedConcreteRight: MixedRoot {
        constructor(): MixedRoot(2) {}
    }

    class MixedOther: MixedRoot {
        constructor(): MixedRoot(99) {}
    }

    class MixedDiamond: MixedVirtualLeft, MixedConcreteRight {
        constructor(): MixedRoot(10), MixedVirtualLeft(), MixedConcreteRight() {}
    }

    class AmbRoot {
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
    }

    class AmbChild: AmbRoot {
        constructor(): AmbRoot(2) {}
    }

    class AmbMixed: virtual AmbRoot, AmbChild {
        constructor(): AmbRoot(5), AmbChild() {}
    }

    class PathRoot {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }
    }

    class PathLeft: PathRoot {
        constructor(): PathRoot(11) {}
    }

    class PathRight: PathRoot {
        constructor(): PathRoot(17) {}
    }

    class PathBranch: PathLeft, PathRight {
        constructor(): PathLeft(), PathRight() {}
    }

    class PathDiamond: PathBranch {
        constructor(): PathBranch() {}
    }

    class DowncastPathRoot {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        virtual fn value(&self) -> usize {
            self.value
        }
    }

    class DowncastPathLeft: DowncastPathRoot {
        constructor(): DowncastPathRoot(31) {}
    }

    class DowncastPathRight: DowncastPathRoot {
        constructor(): DowncastPathRoot(47) {}
    }

    class DowncastPathDiamond: DowncastPathLeft, DowncastPathRight {
        constructor(): DowncastPathLeft(), DowncastPathRight() {}
    }

    class OwnedViaDynRoot {
        value: usize,

        constructor(value: usize) {
            self.value = value;
        }

        fn value(&self) -> usize {
            self.value
        }
    }

    class OwnedViaDynLeft: OwnedViaDynRoot {
        constructor(value: usize): OwnedViaDynRoot(value) {}
    }

    class OwnedViaDynRight: OwnedViaDynRoot {
        constructor(value: usize): OwnedViaDynRoot(value) {}
    }

    class OwnedViaDynBranch: OwnedViaDynLeft, OwnedViaDynRight {
        constructor(left: usize, right: usize): OwnedViaDynLeft(left), OwnedViaDynRight(right) {}
    }

    class OwnedViaDynTopLeft: OwnedViaDynBranch {
        constructor(): OwnedViaDynBranch(10, 11) {}
    }

    class OwnedViaDynTopRight: OwnedViaDynBranch {
        constructor(): OwnedViaDynBranch(20, 21) {}
    }

    class OwnedViaDynDiamond: OwnedViaDynTopLeft, OwnedViaDynTopRight {
        constructor(): OwnedViaDynTopLeft(), OwnedViaDynTopRight() {}
    }

    class GenericViaRoot {
        label: &'static str,

        constructor(label: &'static str) {
            self.label = label;
        }

        fn label(&self) -> &'static str {
            self.label
        }
    }

    class GenericViaBase<T>: GenericViaRoot {
        marker: core::marker::PhantomData<T> = core::marker::PhantomData,

        constructor(label: &'static str): GenericViaRoot(label) {}

        fn type_name(&self) -> &'static str {
            core::any::type_name::<T>()
        }
    }

    class GenericViaLeft: GenericViaBase<i32> {
        constructor(): GenericViaBase<i32>("left") {}
    }

    class GenericViaRight: GenericViaBase<String> {
        constructor(): GenericViaBase<String>("right") {}
    }

    class GenericViaDiamond: GenericViaLeft, GenericViaRight {
        constructor(): GenericViaLeft(), GenericViaRight() {}
    }

    class SpecializedSlot<T> {
        label: String,

        constructor(label: String) {
            self.label = label;
        }

        fn label(&self) -> &str {
            &self.label
        }

        virtual fn type_name(&self) -> &'static str {
            core::any::type_name::<T>()
        }
    }

    class SpecializedLeft: virtual SpecializedSlot<i32> {
        constructor(): SpecializedSlot<i32>("left".into()) {}
    }

    class SpecializedRight: virtual SpecializedSlot<String> {
        constructor(): SpecializedSlot<String>("right".into()) {}
    }

    class SpecializedDiamond: SpecializedLeft, SpecializedRight {
        constructor():
            SpecializedSlot<i32>("int".into()),
            SpecializedSlot<String>("string".into()),
            SpecializedLeft(),
            SpecializedRight()
        {}
    }
}

const CONST_VALUE: ConstValue<usize> = ConstValue { value: 17 };
const CONST_GET: &usize = CONST_VALUE.get();
const CONST_PASSTHROUGH: usize = CONST_VALUE.passthrough(23);
const CONST_UNCHECKED: &usize = unsafe { CONST_VALUE.unchecked() };

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

    assert_eq!(dog.as_base::<Animal>().kingdom(), "animalia");
    assert_eq!(dog.as_base::<Animal>().speak(), "woof -> generic");
    assert_eq!(dog.as_base::<Walker>().walk(), "walking");

    plain_child.as_base_mut::<PlainBase>().set_value(7);
    assert_eq!(plain_child.as_base::<PlainBase>().local(), "local");
    assert_eq!(plain_child.as_base::<PlainBase>().value(), 7);

    assert_eq!(dog.as_base_mut::<Animal>().kingdom(), "animalia");
}

#[test]
fn supports_heterogeneous_base_collections_for_virtual_methods() {
    let dog = Dog::default();
    let cat = Cat::default();
    let animals: Vec<&Animal> = vec![dog.as_base::<Animal>(), cat.as_base::<Animal>()];
    let sounds: Vec<_> = animals.iter().map(|animal| animal.speak()).collect();

    assert_eq!(sounds, ["woof -> generic", "meow"]);
}

#[test]
fn base_references_dispatch_inherited_and_mutable_virtual_methods() {
    let dog = Dog::default();
    let walkers: Vec<&Walker> = vec![dog.as_base::<Walker>()];
    assert_eq!(walkers[0].walk(), "walking");

    let mut counter = Counter::default();
    let mut loud_counter = LoudCounter::default();
    let mut counters: Vec<&mut Counter> = vec![&mut counter, loud_counter.as_base_mut::<Counter>()];
    let counts: Vec<_> = counters
        .iter_mut()
        .map(|counter| (*counter).inc())
        .collect();

    assert_eq!(counts, [1, 11]);
}

#[test]
fn owned_base_trait_objects_can_target_inherited_interfaces() {
    let animal: Box<dyn AsAnimal> = Box::new(Dog::default());
    assert_eq!(animal.as_base::<Animal>().speak(), "woof -> generic");

    let walker: Box<dyn AsWalker> = Box::new(Dog::default());
    assert_eq!(walker.as_base::<Walker>().walk(), "walking");
}

#[test]
fn owned_base_trait_objects_downcast_through_inheritance() {
    let animal: Box<dyn AsAnimal> = Box::new(Kangaroo::default());
    let mammal = match animal.downcast::<dyn AsMammal>() {
        Ok(mammal) => mammal,
        Err(_) => panic!("kangaroo should downcast from Animal to Mammal"),
    };
    assert_eq!(mammal.as_base::<Mammal>().speak(), "chuff");

    let kangaroo = match mammal.downcast::<dyn AsKangaroo>() {
        Ok(kangaroo) => kangaroo,
        Err(_) => panic!("kangaroo should downcast from Mammal to Kangaroo"),
    };
    assert_eq!(kangaroo.as_base::<Kangaroo>().walk(), "walking");

    let walker: Box<dyn AsWalker> = Box::new(Kangaroo::default());
    let kangaroo = match walker.downcast::<dyn AsKangaroo>() {
        Ok(kangaroo) => kangaroo,
        Err(_) => panic!("kangaroo should downcast from Walker to Kangaroo"),
    };
    assert_eq!(kangaroo.as_base::<Kangaroo>().speak(), "chuff");
}

#[test]
fn failed_owned_downcast_preserves_original_box() {
    let animal: Box<dyn AsAnimal> = Box::new(Cat::default());
    let animal = match animal.downcast::<dyn AsDog>() {
        Ok(_) => panic!("cat should not downcast to Dog"),
        Err(animal) => animal,
    };

    assert_eq!(animal.as_base::<Animal>().speak(), "meow");
}

#[test]
fn borrowed_base_references_downcast_through_vtable_metadata() {
    let kangaroo = Kangaroo::default();
    let animal = kangaroo.as_base::<Animal>();
    let mammal = animal
        .downcast_ref::<Mammal>()
        .expect("kangaroo Animal view should downcast to Mammal");
    let kangaroo_ref = animal
        .downcast_ref::<Kangaroo>()
        .expect("kangaroo Animal view should downcast to Kangaroo");

    assert_eq!(mammal.speak(), "chuff");
    assert_eq!(kangaroo_ref.walk(), "walking");

    let walker = kangaroo.as_base::<Walker>();
    let kangaroo_ref = walker
        .downcast_ref::<Kangaroo>()
        .expect("kangaroo Walker view should downcast to Kangaroo");
    assert_eq!(kangaroo_ref.speak(), "chuff");

    let cat = Cat::default();
    assert!(cat.as_base::<Animal>().downcast_ref::<Mammal>().is_none());
}

#[test]
fn mutable_base_references_downcast_through_vtable_metadata() {
    let mut counter = LoudCounter::default();
    let base = counter.as_base_mut::<Counter>();
    let loud = base
        .downcast_mut::<LoudCounter>()
        .expect("loud counter should downcast from Counter");

    assert_eq!(loud.inc(), 11);
}

#[test]
fn borrowed_downcast_follows_the_receiver_inheritance_path() {
    let diamond = DowncastPathDiamond::new();
    let left_root = diamond.as_base_via::<DowncastPathLeft, DowncastPathRoot>();
    let right_root = diamond.as_base_via::<DowncastPathRight, DowncastPathRoot>();

    assert_eq!(
        left_root
            .downcast_ref::<DowncastPathLeft>()
            .expect("left root should downcast to left")
            .as_base::<DowncastPathRoot>()
            .value(),
        31
    );
    assert!(left_root.downcast_ref::<DowncastPathRight>().is_none());
    assert_eq!(
        right_root
            .downcast_ref::<DowncastPathRight>()
            .expect("right root should downcast to right")
            .as_base::<DowncastPathRoot>()
            .value(),
        47
    );
    assert!(right_root.downcast_ref::<DowncastPathLeft>().is_none());

    assert!(core::ptr::eq(
        left_root
            .downcast_ref::<DowncastPathDiamond>()
            .expect("left root should downcast to complete diamond"),
        &diamond,
    ));
    assert!(core::ptr::eq(
        right_root
            .downcast_ref::<DowncastPathDiamond>()
            .expect("right root should downcast to complete diamond"),
        &diamond,
    ));
}

#[test]
fn owned_downcast_follows_the_receiver_inheritance_path() {
    let diamond: Box<DowncastPathDiamond> = Box::new(DowncastPathDiamond::new());
    let root: Box<dyn AsDowncastPathRoot> =
        diamond.into_base_via::<DowncastPathLeft, dyn AsDowncastPathRoot>();
    let left = match root.downcast::<dyn AsDowncastPathLeft>() {
        Ok(left) => left,
        Err(_) => panic!("left root should downcast to left"),
    };
    assert_eq!(
        left.as_base::<DowncastPathLeft>()
            .as_base::<DowncastPathRoot>()
            .value(),
        31
    );

    let diamond: Box<DowncastPathDiamond> = Box::new(DowncastPathDiamond::new());
    let root: Box<dyn AsDowncastPathRoot> =
        diamond.into_base_via::<DowncastPathLeft, dyn AsDowncastPathRoot>();
    let root = match root.downcast::<dyn AsDowncastPathRight>() {
        Ok(_) => panic!("left root should not downcast to right"),
        Err(root) => root,
    };
    let diamond = match root.downcast::<dyn AsDowncastPathDiamond>() {
        Ok(diamond) => diamond,
        Err(_) => panic!("left root should downcast to complete diamond"),
    };
    assert_eq!(
        diamond
            .as_base::<DowncastPathDiamond>()
            .as_base_via::<DowncastPathRight, DowncastPathRoot>()
            .value(),
        47
    );
}

#[test]
fn exposes_c3_metadata_and_uses_c3_for_forwarding() {
    let object = C::default();

    assert_eq!(<C as OopClass>::MRO, &["C", "A", "B", "Object"]);
    assert_eq!(object.name(), "A");
    assert_eq!(object.label(), "B");
    assert_eq!(object.root(), "object");
    assert_ne!(
        object.as_base_via::<A, Object>() as *const Object,
        object.as_base_via::<B, Object>() as *const Object,
    );
    assert_eq!(object.as_base::<B>().as_base::<Object>().name(), "A");
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
    let shapes: Vec<&Shape> = vec![square.as_base::<Shape>()];

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
    let drawables: Vec<&AbstractDrawable> = vec![icon.as_base::<AbstractDrawable>()];

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
        dog.as_base::<ConstructedAnimal>().events(),
        vec!["dog:base".to_string(), "dog:derived".to_string()]
    );
}

#[test]
fn supports_unsafe_direct_and_virtual_methods() {
    let child = UnsafeChild::default();

    unsafe {
        assert_eq!(child.as_base::<UnsafeBase>().direct_secret(), 3);
        assert_eq!(child.code(), 29);
        assert_eq!(child.as_base::<UnsafeBase>().code(), 29);
    }
}

#[test]
fn supports_async_direct_and_virtual_methods() {
    let leaf = AsyncLeaf::default();
    let concrete = AsyncConcrete::default();
    let roots: Vec<&AsyncRoot> = vec![leaf.as_base::<AsyncRoot>()];

    assert_eq!(block_on(leaf.score("abc")), 13);
    assert_eq!(block_on(leaf.as_base::<AsyncRoot>().score("abcd")), 14);
    assert_eq!(block_on(leaf.as_base::<AsyncRoot>().direct_score("xy")), 3);
    assert_eq!(block_on(leaf.as_base::<AsyncRoot>().direct_label()), "");
    assert_eq!(block_on(leaf.as_base::<AsyncRoot>().label_ref()), "");
    assert_eq!(block_on(roots[0].score("hello")), 15);
    assert_eq!(block_on(concrete.as_base::<AsyncAbstract>().load()), 33);
}

#[test]
fn supports_generic_classes_and_base_views() {
    let leaf = GenericLeaf::new("leaf".to_string());
    let slots: Vec<Box<dyn AsGenericSlot<String>>> =
        vec![Box::new(GenericLeaf::new("boxed".to_string()))];
    let slot: Box<dyn AsGenericSlot<String>> = Box::new(GenericLeaf::new("downcast".to_string()));
    let leaf_box = match slot.downcast::<dyn AsGenericLeaf<String>>() {
        Ok(leaf) => leaf,
        Err(_) => panic!("generic slot should downcast to GenericLeaf"),
    };

    assert_eq!(leaf.get(), "leaf");
    assert_eq!(leaf.as_base::<GenericSlot<String>>().get(), "leaf");
    assert_eq!(
        leaf.as_base::<GenericSlot<String>>().passthrough(42usize),
        42
    );
    assert_eq!(
        leaf.as_base::<GenericSlot<String>>()
            .cloned("clone".to_string()),
        "clone"
    );
    assert_eq!(slots[0].as_base::<GenericSlot<String>>().get(), "boxed");
    assert_eq!(leaf_box.as_base::<GenericLeaf<String>>().get(), "downcast");
}

#[test]
fn target_explicit_as_base_casts_to_unambiguous_bases() {
    let mut dog = Dog::default();
    assert_eq!(dog.as_base::<Animal>().kingdom(), "animalia");
    assert_eq!(dog.as_base::<Animal>().speak(), "woof -> generic");
    assert_eq!(dog.as_base::<Walker>().walk(), "walking");
    assert_eq!(dog.as_base_mut::<Animal>().kingdom(), "animalia");

    let animal: Box<dyn AsAnimal> = Box::new(Dog::default());
    assert_eq!(animal.as_base::<Animal>().speak(), "woof -> generic");

    let mut loud_counter = LoudCounter::default();
    assert_eq!(loud_counter.as_base_mut::<Counter>().inc(), 11);

    let leaf = GenericLeaf::new("leaf".to_string());
    assert_eq!(leaf.as_base::<GenericSlot<String>>().get(), "leaf");

    let slot: Box<dyn AsGenericSlot<String>> = Box::new(GenericLeaf::new("boxed".to_string()));
    assert_eq!(slot.as_base::<GenericSlot<String>>().get(), "boxed");
}

#[test]
fn supports_const_direct_methods() {
    assert_eq!(*CONST_GET, 17);
    assert_eq!(CONST_PASSTHROUGH, 23);
    assert_eq!(*CONST_UNCHECKED, 17);
}

#[test]
fn supports_static_methods_and_associated_constants() {
    assert_eq!(StaticUtility::DEFAULT, 41);
    assert_eq!(StaticUtility::PUBLIC, 42);
    assert_eq!(StaticUtility::default_value(), 41);
    assert_eq!(StaticUtility::public_value(), 42);
    assert_eq!(StaticUtility::pair("left", "right"), ("left", "right"));
    assert_eq!(StaticUtility::NEXT_ID.load(Ordering::Relaxed), 1);
    assert_eq!(StaticUtility::allocate_id(), 1);
    assert_eq!(StaticUtility::allocate_id(), 2);
    assert_eq!(StaticUtility::NEXT_ID.load(Ordering::Relaxed), 3);
}

#[test]
fn supports_field_initializers() {
    let default_value = InitializedFields::default();
    let constructed = InitializedFields::new();

    assert_eq!(default_value.id(), 7);
    assert_eq!(default_value.label(), "ready");
    assert_eq!(default_value.numbers(), &[1, 2, 3]);

    assert_eq!(constructed.id(), 7);
    assert_eq!(constructed.label(), "ready");
    assert_eq!(constructed.numbers(), &[1, 2, 3, 4]);
}

#[test]
fn virtual_diamond_shares_base_storage_and_dispatch() {
    VIRTUAL_OBJECT_DROPS.store(0, Ordering::SeqCst);

    {
        let mut diamond = VirtualDiamond::new();

        assert!(core::ptr::eq(
            diamond.as_base::<VirtualLeft>().as_base::<VirtualObject>(),
            diamond.as_base::<VirtualRight>().as_base::<VirtualObject>(),
        ));
        assert_eq!(diamond.as_base::<VirtualObject>().raw_value(), 10);
        assert_eq!(
            diamond
                .as_base::<VirtualLeft>()
                .as_base::<VirtualObject>()
                .virtual_value(),
            1010
        );
        assert!(diamond
            .as_base::<VirtualObject>()
            .downcast_ref::<VirtualLeft>()
            .is_some());
        assert!(diamond
            .as_base::<VirtualObject>()
            .downcast_ref::<VirtualRight>()
            .is_some());
        assert!(diamond
            .as_base::<VirtualObject>()
            .downcast_ref::<VirtualDiamond>()
            .is_some());

        diamond
            .as_base_mut::<VirtualRight>()
            .as_base_mut::<VirtualObject>()
            .set_raw_value(33);
        assert_eq!(
            diamond
                .as_base::<VirtualLeft>()
                .as_base::<VirtualObject>()
                .raw_value(),
            33
        );
        assert_eq!(diamond.as_base::<VirtualObject>().virtual_value(), 1033);
    }

    assert_eq!(VIRTUAL_OBJECT_DROPS.load(Ordering::SeqCst), 1);

    let boxed: Box<dyn AsVirtualObject> = Box::new(VirtualDiamond::new());
    let boxed = match boxed.downcast::<dyn AsVirtualDiamond>() {
        Ok(boxed) => boxed,
        Err(_) => panic!("virtual object box should downcast to complete diamond"),
    };
    assert_eq!(boxed.as_base::<VirtualDiamond>().virtual_value(), 1010);
}

#[test]
fn virtual_base_can_be_reached_by_direct_and_indirect_edges() {
    let mut diamond = DirectIndirectVirtualDiamond::new();

    assert!(core::ptr::eq(
        diamond.as_base::<DirectIndirectVirtualRoot>(),
        diamond
            .as_base::<IndirectVirtualBranch>()
            .as_base::<DirectIndirectVirtualRoot>(),
    ));
    assert_eq!(diamond.as_base::<DirectIndirectVirtualRoot>().value(), 7);
    assert_eq!(
        diamond.as_base::<DirectIndirectVirtualRoot>().dispatched(),
        107
    );

    diamond
        .as_base_mut::<IndirectVirtualBranch>()
        .as_base_mut::<DirectIndirectVirtualRoot>()
        .set_value(19);
    assert_eq!(diamond.as_base::<DirectIndirectVirtualRoot>().value(), 19);
    assert_eq!(
        diamond.as_base::<DirectIndirectVirtualRoot>().dispatched(),
        119
    );
}

#[test]
fn mixed_virtual_and_non_virtual_paths_create_distinct_base_subobjects() {
    let mut diamond = MixedDiamond::new();

    assert!(core::ptr::eq(
        diamond.as_base_via::<MixedVirtualLeft, MixedRoot>(),
        diamond.as_base::<MixedVirtualLeft>().as_base::<MixedRoot>(),
    ));
    assert_ne!(
        diamond.as_base_via::<MixedVirtualLeft, MixedRoot>() as *const MixedRoot,
        diamond.as_base_via::<MixedConcreteRight, MixedRoot>() as *const MixedRoot,
    );
    assert_eq!(
        diamond.as_base_via::<MixedVirtualLeft, MixedRoot>().value(),
        10
    );
    assert_eq!(
        diamond
            .as_base_via::<MixedConcreteRight, MixedRoot>()
            .value(),
        2
    );

    diamond
        .as_base_via_mut::<MixedConcreteRight, MixedRoot>()
        .set_value(22);
    assert_eq!(
        diamond.as_base_via::<MixedVirtualLeft, MixedRoot>().value(),
        10
    );
    assert_eq!(
        diamond
            .as_base_via::<MixedConcreteRight, MixedRoot>()
            .value(),
        22
    );

    diamond
        .as_base_via_mut::<MixedVirtualLeft, MixedRoot>()
        .set_value(33);
    assert_eq!(
        diamond.as_base_via::<MixedVirtualLeft, MixedRoot>().value(),
        33
    );
    assert_eq!(
        diamond
            .as_base_via::<MixedConcreteRight, MixedRoot>()
            .value(),
        22
    );

    let diamond_trait: &dyn AsMixedDiamond = &diamond;
    assert_eq!(
        diamond_trait
            .as_base_via::<MixedVirtualLeft, MixedRoot>()
            .value(),
        33,
    );
}

#[test]
fn direct_virtual_and_indirect_non_virtual_paths_are_explicit() {
    let mut mixed = AmbMixed::new();

    assert_ne!(
        mixed.as_base_via::<AmbRoot, AmbRoot>() as *const AmbRoot,
        mixed.as_base_via::<AmbChild, AmbRoot>() as *const AmbRoot,
    );
    assert_eq!(mixed.as_base_via::<AmbRoot, AmbRoot>().value(), 5);
    assert_eq!(mixed.as_base_via::<AmbChild, AmbRoot>().value(), 2);

    mixed.as_base_via_mut::<AmbRoot, AmbRoot>().set_value(8);
    mixed.as_base_via_mut::<AmbChild, AmbRoot>().set_value(13);

    assert_eq!(mixed.as_base_via::<AmbRoot, AmbRoot>().value(), 8);
    assert_eq!(mixed.as_base_via::<AmbChild, AmbRoot>().value(), 13);
}

#[test]
fn via_type_selects_specific_subobject() {
    let diamond = PathDiamond::new();

    assert_ne!(
        diamond.as_base_via::<PathLeft, PathRoot>() as *const PathRoot,
        diamond.as_base_via::<PathRight, PathRoot>() as *const PathRoot,
    );
    assert_eq!(diamond.as_base_via::<PathLeft, PathRoot>().value(), 11);
    assert_eq!(diamond.as_base_via::<PathRight, PathRoot>().value(), 17);

    let branch_trait: &dyn AsPathBranch = diamond.as_base::<PathBranch>();
    assert_eq!(branch_trait.as_base_via::<PathLeft, PathRoot>().value(), 11,);
}

#[test]
fn via_type_uses_actual_generic_specialization() {
    let diamond = GenericViaDiamond::new();

    let left_root = diamond.as_base_via::<GenericViaBase<i32>, GenericViaRoot>();
    let right_root = diamond.as_base_via::<GenericViaBase<String>, GenericViaRoot>();

    assert_ne!(
        left_root as *const GenericViaRoot,
        right_root as *const GenericViaRoot,
    );
    assert_eq!(left_root.label(), "left");
    assert_eq!(right_root.label(), "right");
    assert_eq!(
        diamond
            .as_base_via::<GenericViaBase<i32>, GenericViaRoot>()
            .label(),
        "left",
    );
}

#[test]
fn owned_via_upcasts_preserve_selected_subobject() {
    let concrete_root =
        Box::new(MixedDiamond::new()).into_base_via::<MixedVirtualLeft, dyn AsMixedRoot>();
    assert_eq!(concrete_root.as_base::<MixedRoot>().value(), 10);

    let boxed: Box<dyn AsMixedDiamond> = Box::new(MixedDiamond::new());
    let root = boxed.into_base_via::<MixedConcreteRight, dyn AsMixedRoot>();

    assert_eq!(root.as_base::<MixedRoot>().value(), 2);

    let diamond = match root.downcast::<dyn AsMixedDiamond>() {
        Ok(diamond) => diamond,
        Err(_) => panic!("path-owned MixedRoot should downcast back to MixedDiamond"),
    };
    assert_eq!(
        diamond
            .as_base::<MixedDiamond>()
            .as_base_via::<MixedConcreteRight, MixedRoot>()
            .value(),
        2,
    );
}

#[test]
fn owned_via_upcasts_support_tuple_paths() {
    let boxed: Box<dyn AsPathDiamond> = Box::new(PathDiamond::new());
    let root = boxed.into_base_via::<(PathBranch, PathRight), dyn AsPathRoot>();

    assert_eq!(root.as_base::<PathRoot>().value(), 17);
}

#[test]
fn owned_via_upcast_from_path_owned_dyn_source_preserves_source_subobject() {
    let diamond: Box<OwnedViaDynDiamond> = Box::new(OwnedViaDynDiamond::new());
    let branch: Box<dyn AsOwnedViaDynBranch> =
        diamond.into_base_via::<OwnedViaDynTopRight, dyn AsOwnedViaDynBranch>();
    let root = branch.into_base_via::<OwnedViaDynLeft, dyn AsOwnedViaDynRoot>();

    assert_eq!(root.as_base::<OwnedViaDynRoot>().value(), 20);

    let diamond: Box<OwnedViaDynDiamond> = Box::new(OwnedViaDynDiamond::new());
    let branch: Box<dyn AsOwnedViaDynBranch> =
        diamond.into_base_via::<OwnedViaDynTopLeft, dyn AsOwnedViaDynBranch>();
    let root = branch.into_base_via::<OwnedViaDynRight, dyn AsOwnedViaDynRoot>();

    assert_eq!(root.as_base::<OwnedViaDynRoot>().value(), 11);
}

#[test]
fn owned_via_upcasts_use_actual_generic_specialization() {
    let boxed: Box<dyn AsGenericViaDiamond> = Box::new(GenericViaDiamond::new());
    let root = boxed.into_base_via::<GenericViaBase<String>, dyn AsGenericViaRoot>();

    assert_eq!(root.as_base::<GenericViaRoot>().label(), "right");
}

#[test]
fn failed_owned_via_downcast_preserves_original_box() {
    let boxed: Box<dyn AsMixedDiamond> = Box::new(MixedDiamond::new());
    let root = boxed.into_base_via::<MixedVirtualLeft, dyn AsMixedRoot>();
    let root = match root.downcast::<dyn AsMixedOther>() {
        Ok(_) => panic!("MixedDiamond should not downcast to MixedOther"),
        Err(root) => root,
    };

    assert_eq!(root.as_base::<MixedRoot>().value(), 10);
}

#[test]
fn virtual_generic_specializations_are_distinct_bases() {
    let diamond = SpecializedDiamond::new();
    let left_slot: &SpecializedSlot<i32> = diamond.as_base::<SpecializedSlot<i32>>();
    let right_slot: &SpecializedSlot<String> = diamond.as_base::<SpecializedSlot<String>>();

    assert_ne!(
        left_slot as *const SpecializedSlot<i32> as *const (),
        right_slot as *const SpecializedSlot<String> as *const (),
    );
    assert_eq!(left_slot.label(), "int");
    assert_eq!(right_slot.label(), "string");
    assert_eq!(left_slot.type_name(), "i32");
    assert_eq!(right_slot.type_name(), "alloc::string::String");
    assert_eq!(diamond.as_base::<SpecializedSlot<i32>>().label(), "int");
    assert_eq!(
        diamond.as_base::<SpecializedSlot<String>>().label(),
        "string"
    );

    let left = left_slot
        .downcast_ref::<SpecializedLeft>()
        .expect("i32 slot should downcast to SpecializedLeft");
    let right = right_slot
        .downcast_ref::<SpecializedRight>()
        .expect("String slot should downcast to SpecializedRight");
    assert_eq!(left.as_base::<SpecializedSlot<i32>>().label(), "int");
    assert_eq!(right.as_base::<SpecializedSlot<String>>().label(), "string");

    let slot: Box<dyn AsSpecializedSlot<String>> = Box::new(SpecializedDiamond::new());
    let right = match slot.downcast::<dyn AsSpecializedRight>() {
        Ok(right) => right,
        Err(_) => panic!("String slot should owned-downcast to SpecializedRight"),
    };
    assert_eq!(
        right
            .as_base::<SpecializedRight>()
            .as_base::<SpecializedSlot<String>>()
            .label(),
        "string"
    );
}
