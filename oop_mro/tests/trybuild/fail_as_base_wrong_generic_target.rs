use oop_mro::prelude::*;

oop_class! {
    abstract class Repository<T> {
        abstract virtual fn current(&self) -> &T;
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
}

fn main() {
    let repository = MemoryRepository::new(String::from("stored"));
    let _ = repository.as_base::<Repository<u8>>();
}
