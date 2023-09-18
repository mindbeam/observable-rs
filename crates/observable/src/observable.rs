use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::notifier::{CleanUp, ListenerHandle, Notifier};

pub struct Observable<T>(Rc<Inner<T>>);

struct Inner<T> {
    value: RefCell<T>,
    notifier: Notifier<T>,
}

// Implemented manually because `T` does not need to be Clone
impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Observable(self.0.clone())
    }
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(Inner {
            value: RefCell::new(value),
            notifier: Notifier::default(),
        }))
    }

    pub fn set(&self, value: T) {
        self.0.value.replace(value);
        let r = self.0.value.borrow();
        self.0.notifier.notify(&r);
    }
}

impl<T> Observable<T> {
    pub fn get(&self) -> Ref<T> {
        self.0.value.borrow()
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.0.notifier.subscribe(cb)
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> ListenerHandle {
        self.0.notifier.once(cb)
    }

    pub fn on_cleanup(&self, clean_up: impl Into<CleanUp>) {
        self.0.notifier.on_cleanup(clean_up.into())
    }

    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        self.0.notifier.unsubscribe(handle)
    }

    pub fn clean_up(&self) {
        self.0.notifier.clean_up()
    }
}

impl<T, V> Observable<V>
where
    V: Pushable<Value = T>,
{
    pub fn push(&self, item: T) {
        {
            let mut ref_mut = self.0.value.borrow_mut();
            let vec = &mut *ref_mut;
            vec.push(item);
        }

        self.0.notifier.notify(&self.0.value.borrow());
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

impl<T: 'static> From<(&Observable<T>, ListenerHandle)> for CleanUp {
    fn from((obs, handle): (&Observable<T>, ListenerHandle)) -> Self {
        let weak_obs = Rc::downgrade(&obs.0);

        let f: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(obs) = weak_obs.upgrade() {
                Observable::<T>(obs).unsubscribe(handle);
            }
        });

        CleanUp::from(f)
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use crate::Pushable;

    use super::Observable;

    #[test]
    fn observable_vec_push() {
        let obs = Observable::new(vec![1, 2, 3]);

        let counter: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        {
            let counter = counter.clone();
            obs.subscribe(Box::new(move |v: &Vec<u32>| {
                counter.replace(Some(v.len()));
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

    #[test]
    fn observable_on_cleanup() {
        let obs = Observable::new(5);

        let count = Rc::new(Cell::new(0));
        let f = {
            let count = count.clone();
            move || {
                count.set(1);
                drop(count);
            }
        };
        let clean_up: Box<dyn FnOnce()> = Box::new(f);
        obs.on_cleanup(clean_up);

        assert_eq!(count.get(), 0);
        obs.set(1);
        assert_eq!(count.get(), 0);
        obs.clean_up();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn observable_on_cleanup_by_drop() {
        let obs = Observable::new(5);

        let count = Rc::new(Cell::new(0));
        let f = {
            let count = count.clone();
            move || {
                count.set(1);
                drop(count);
            }
        };
        let clean_up: Box<dyn FnOnce()> = Box::new(f);
        obs.on_cleanup(clean_up);

        assert_eq!(count.get(), 0);
        obs.set(1);
        assert_eq!(count.get(), 0);
        drop(obs);
        assert_eq!(count.get(), 1);
    }
}
