use oop_mro::prelude::*;

oop_class! {
    abstract class Service {
        async fn direct(&self, input: &str) -> usize {
            input.len()
        }

        abstract virtual async fn fetch(&self, _input: &str) -> usize;
    }

    class ApiService: Service {
        #[override]
        virtual async fn fetch(&self, input: &str) -> usize {
            input.len() + 10
        }
    }

    class AsyncUnsafeBase {
        virtual async unsafe fn secret(&self) -> usize {
            1
        }
    }

    class AsyncUnsafeChild: AsyncUnsafeBase {
        #[override]
        virtual async unsafe fn secret(&self) -> usize {
            2
        }
    }
}

fn main() {
    let service = ApiService::default();
    let _direct = service.as_base::<Service>().direct("abc");
    let _virtual_direct = service.fetch("abc");
    let _virtual_base = service.as_base::<Service>().fetch("abc");

    let unsafe_service = AsyncUnsafeChild::default();
    let _unsafe_direct = unsafe { unsafe_service.secret() };
    let _unsafe_base = unsafe { unsafe_service.as_base::<AsyncUnsafeBase>().secret() };
}
