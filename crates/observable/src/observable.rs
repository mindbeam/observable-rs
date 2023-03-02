use std::{cell::Ref, cell::RefCell, rc::Rc};

use crate::notifier::{ListenerHandle, Notifier};

pub struct Observable<T> {
    value: Rc<RefCell<T>>,
    notifier: Rc<Notifier<T>>,
}

// Implemented manually because `T` does not need to be Clone
impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Observable {
            value: self.value.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            notifier: Rc::default(),
        }
    }

    // impl<T> Set<T> for Observable<T> {
    pub fn set(&self, value: T) {
        {
            *(self.value.borrow_mut()) = value;
        };
        let r = self.value.borrow();
        self.notifier.notify(&r);
    }
    // }
    // impl<T> Observe<T> for Observable<T> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.notifier.subscribe(cb)
    }
    pub fn once(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.notifier.once(cb)
    }

    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        self.notifier.unsubscribe(handle)
    }
}

impl<T, V> Observable<V>
where
    V: Pushable<Value = T>,
{
    pub fn push(&self, item: T) {
        {
            let mut ref_mut = self.value.borrow_mut();
            let vec = &mut *ref_mut;
            vec.push(item);
        }

        self.notifier.notify(&self.value.borrow());
    }
}

impl<T> Default for Observable<T>
where
    T: Default,
{
    fn default() -> Self {
        Observable::new(T::default())
    }
}

pub trait Pushable {
    type Value;
    fn push(&mut self, value: Self::Value);
}

impl<T> Pushable for Vec<T> {
    type Value = T;
    fn push(&mut self, value: Self::Value) {
        self.push(value)
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, rc::Rc};

    use crate::Pushable;

    use super::Observable;

    #[test]
    fn observable_vec_push() {
        let obs = Observable::new(vec![1, 2, 3]);

        let counter: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        {
            let counter = counter.clone();
            obs.subscribe(Box::new(move |v: &Vec<u32>| {
                *(counter.borrow_mut()) = Some(v.len());
            }));
        }

        assert_eq!(*counter.borrow(), None);
        obs.push(0);
        assert_eq!(*counter.borrow(), Some(4));
    }

    struct Wrapper<T>(Vec<T>);

    impl<T> Pushable for Wrapper<T> {
        type Value = T;

        fn push(&mut self, value: Self::Value) {
            self.0.push(value)
        }
    }

    #[test]
    fn observable_vec_wrapper_push() {
        let obs = Observable::new(Wrapper(vec![1, 2, 3]));

        let counter: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        {
            let counter = counter.clone();
            obs.subscribe(Box::new(move |v: &Wrapper<u32>| {
                *(counter.borrow_mut()) = Some(v.0.len());
            }));
        }

        assert_eq!(*counter.borrow(), None);
        obs.push(0);
        assert_eq!(*counter.borrow(), Some(4));
    }

    #[test]
    fn observable_reactivity() {
        let obs = Observable::new("hello".to_owned());

        let counter_durable: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        {
            let counter_durable = counter_durable.clone();
            obs.subscribe(Box::new(move |_: &String| {
                let mut ptr = counter_durable.borrow_mut();
                *ptr = match *ptr {
                    Some(c) => Some(c + 1),
                    None => Some(1),
                };
            }));
        }

        let counter_once: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        {
            let counter_durable = counter_once.clone();
            obs.once(Box::new(move |_: &String| {
                let mut ptr = counter_durable.borrow_mut();
                *ptr = match *ptr {
                    Some(_) => unreachable!(),
                    None => Some(1),
                };
            }));
        }

        assert_eq!(*counter_durable.borrow(), None);
        assert_eq!(*counter_once.borrow(), None);

        obs.set("world".into());
        assert_eq!(*counter_durable.borrow(), Some(1));
        assert_eq!(*counter_once.borrow(), Some(1));

        obs.set("hallo".into());
        assert_eq!(*counter_durable.borrow(), Some(2));
        assert_eq!(*counter_once.borrow(), Some(1));
    }
}
