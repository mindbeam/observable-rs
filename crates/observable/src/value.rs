use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::Pushable;

#[derive(Default)]
pub struct Value<T>(RefCell<T>);

impl<T> Value<T> {
    pub fn new(value: T) -> Self {
        Value(RefCell::new(value))
    }
    pub fn rc(value: T) -> Rc<Self> {
        Rc::new(Value(RefCell::new(value)))
    }
    pub fn set(&self, value: T) {
        self.0.replace(value);
    }
    pub fn get(&self) -> Ref<T> {
        self.0.borrow()
    }
}

impl<T: Pushable> Value<T> {
    pub fn push(&self, value: T::Value) {
        self.0.borrow_mut().push(value)
    }
}

#[cfg(test)]
mod test {
    use crate::Value;

    #[test]
    fn set_and_read() {
        let val = Value::new(0);

        assert_eq!(*val.get(), 0);

        val.set(1);
        assert_eq!(*val.get(), 1);
    }

    #[test]
    fn pushable_value() {
        let list = Value::new(vec![]);

        assert_eq!(list.get().len(), 0);

        list.push(5);
        assert_eq!(list.get().len(), 1);
        assert_eq!(list.get()[0], 5);
    }
}
