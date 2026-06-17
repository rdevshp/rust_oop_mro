use oop_mro::prelude::*;

oop_class! {
    abstract class Device {
        abstract virtual unsafe fn read(&self) -> usize;
    }

    class Sensor: Device {
        unsafe fn raw(&self) -> usize {
            7
        }

        #[override]
        virtual unsafe fn read(&self) -> usize {
            42
        }
    }
}

fn main() {
    let sensor = Sensor::default();

    unsafe {
        assert_eq!(sensor.raw(), 7);
        assert_eq!(sensor.read(), 42);
        assert_eq!(sensor.as_base::<Device>().read(), 42);
    }
}
