use oop_mro::prelude::*;

oop_class! {
    class Animal {
        virtual fn speak(&self) -> &'static str {
            "animal"
        }
    }

    class Mammal: Animal {}

    class Walker {
        virtual fn walk(&self) -> &'static str {
            "walk"
        }
    }

    class Kangaroo: Mammal, Walker {}
}

fn main() {
    let kangaroo = Kangaroo::default();
    let _ = kangaroo.as_base::<Walker>().downcast_ref::<Mammal>();

    let walker: Box<dyn AsWalker> = Box::new(Kangaroo::default());
    let _ = walker.downcast::<dyn AsMammal>();
}
