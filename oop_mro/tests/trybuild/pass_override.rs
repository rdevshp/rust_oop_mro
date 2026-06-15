use oop_mro::prelude::*;

oop_class! {
    abstract class Drawable {
        abstract virtual fn draw(&self) -> String;

        virtual fn label(&self) -> String {
            "drawable".into()
        }
    }

    abstract class AbstractIcon: Drawable {
        #[override]
        abstract virtual fn draw(&self) -> String;
    }

    class Icon: AbstractIcon {
        #[override]
        virtual fn draw(&self) -> String {
            "icon".into()
        }

        #[override]
        #[inline]
        virtual fn label(&self) -> String {
            format!("icon {}", super_call!(Drawable::label, self))
        }
    }
}

fn main() {
    let icon = Icon::default();
    let drawable = icon.as_drawable();

    assert_eq!(drawable.draw(), "icon");
    assert_eq!(drawable.label(), "icon drawable");
}
