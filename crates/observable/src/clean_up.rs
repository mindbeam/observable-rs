pub struct CleanUp(Option<Box<dyn FnOnce()>>);

impl Drop for CleanUp {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
impl From<Box<dyn FnOnce()>> for CleanUp {
    fn from(value: Box<dyn FnOnce()>) -> Self {
        CleanUp(Some(value))
    }
}

#[cfg(test)]
mod test {
    use std::{cell::Cell, rc::Rc};

    use crate::CleanUp;

    #[test]
    fn clean_up() {
        let counter = Rc::new(Cell::new(0));
        let f = {
            let counter = counter.clone();
            move || {
                let val = counter.get() + 1;
                counter.set(val);
                drop(counter);
            }
        };
        let f: Box<dyn FnOnce()> = Box::new(f);
        let clean_up = CleanUp::from(f);

        assert_eq!(counter.get(), 0);
        drop(clean_up);
        assert_eq!(counter.get(), 1);
    }
}
