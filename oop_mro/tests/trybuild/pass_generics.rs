use oop_mro::prelude::*;

oop_class! {
    abstract class Repository<T> {
        abstract virtual fn current(&self) -> &T;

        fn map<U>(&self, value: U) -> Option<U> {
            Some(value)
        }
    }

    class MemoryRepository<Item>: Repository<Item> where Item: Default {
        value: Item,

        constructor(value: Item): Repository() {
            self.value = value;
        }

        #[override]
        virtual fn current(&self) -> &Item {
            &self.value
        }
    }

    class ConstBase<const N: usize> {
        virtual fn len(&self) -> usize {
            N
        }
    }

    class ConstLeaf<const M: usize>: ConstBase<M> {
        #[override]
        virtual fn len(&self) -> usize {
            M + 1
        }
    }

    class ArrayHolder<T: Copy, const N: usize> where [T; N]: Default {
        items: [T; N],

        fn len(&self) -> usize {
            N
        }
    }
}

fn main() {
    let repository = MemoryRepository::new(String::from("stored"));
    let base: &Repository<String> = repository.as_repository();
    let owned: Vec<Box<dyn AsRepository<String>>> =
        vec![Box::new(MemoryRepository::new(String::from("boxed")))];
    let array = ArrayHolder::<u8, 4>::default();
    let const_leaf = ConstLeaf::<4>::default();

    assert_eq!(repository.current(), "stored");
    assert_eq!(base.current(), "stored");
    assert_eq!(base.map(7), Some(7));
    assert_eq!(owned[0].as_repository().current(), "boxed");
    assert_eq!(array.len(), 4);
    assert_eq!(const_leaf.as_const_base().len(), 5);
}
