use std::cell::{Ref, RefCell};
use std::rc::{Rc, Weak};

use crate::notifier::{CleanUp, ListenerHandle, Notifier};

pub struct Observable<T, const E: bool = true> {
    value: Rc<RefCell<T>>,
    notifier: Rc<Notifier<T>>,
}

pub struct Reader<T> {
    value: Rc<RefCell<T>>,
    notifier: Weak<Notifier<T>>,
}

impl<T> Reader<T> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }
}

impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

// Implemented manually because `T` does not need to be Clone
impl<T, const E: bool> Clone for Observable<T, E> {
    fn clone(&self) -> Self {
        Observable {
            value: self.value.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T> Observable<T, true> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            notifier: Rc::default(),
        }
    }

    pub fn reader(&self) -> Reader<T> {
        Reader {
            value: self.value.clone(),
            notifier: Rc::downgrade(&self.notifier),
        }
    }
}

// Observable<T> = Observable<T, true>
// Observable<T, false> : mapped observable
impl<T> Observable<T, true> {
    pub fn set(&self, value: T) {
        self.value.replace(value);
        let r = self.value.borrow();
        self.notifier.notify(&r);
    }
}

impl<T, const E: bool> Observable<T, E> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.notifier.subscribe(cb)
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> ListenerHandle {
        self.notifier.once(cb)
    }

    pub fn on_cleanup(&self, clean_up: impl Into<CleanUp>) {
        self.notifier.on_cleanup(clean_up.into())
    }

    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        self.notifier.unsubscribe(handle)
    }

    pub fn clean_up(&self) {
        self.notifier.clean_up()
    }
}

type MapFn<T, R> = Box<dyn Fn(&T) -> R>;
type MapWeakObsFn<T, R> = MapFn<T, Reader<R>>;
type MapObsFn<T, R> = MapFn<T, Observable<R, false>>;

impl<T: 'static> Observable<T, false> {
    pub fn map_value<R: 'static>(self, f: MapFn<T, R>) -> Observable<R, false> {
        let initial_value = { f(&self.get()) };
        let mapped_obs = Observable::new(initial_value);

        let mapped_weak = mapped_obs.reader();
        let self_notifier = self.notifier;
        self_notifier.subscribe(Box::new(move |value| {
            let mapped = f(value);
            if let Some(notifier) = mapped_weak.notifier.upgrade() {
                notifier.notify(&mapped);
            }
        }));
        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self_notifier);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }

    pub fn map_weak_obs<R: Clone + 'static>(self, f: MapWeakObsFn<T, R>) -> Observable<R, false> {
        let middle_obs = { f(&self.get()) };
        let initial_value = { middle_obs.get().clone() };
        let mapped_obs = Observable::new(initial_value);

        if let Some(clean_up) = mapped_obs.reader().subscribe_reader(middle_obs.notifier) {
            mapped_obs.notifier.on_mapped_obs_unsubscribe(clean_up)
        }

        let mapped_weak = mapped_obs.reader();

        self.notifier.subscribe(Box::new(move |value: &T| {
            if let Some(mapped_notifier) = mapped_weak.notifier.upgrade() {
                mapped_notifier.unsubscribe_mapped_obs();
                let middle_obs = { f(value) };
                mapped_weak.value.replace(middle_obs.get().clone());
                if let Some(clean_up) = mapped_weak.clone().subscribe_reader(middle_obs.notifier) {
                    mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                }
            }
        }));

        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self.notifier);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }

    pub fn map_obs<R: Clone + 'static>(self, f: MapObsFn<T, R>) -> Observable<R, false> {
        let initial_mapped_obs = { f(&self.get()) };
        let mapped_obs: Observable<R> = Observable {
            value: initial_mapped_obs.value,
            notifier: Rc::default(),
        };

        let mapped_obs_notifier = Rc::downgrade(&mapped_obs.notifier);
        let handle = initial_mapped_obs
            .notifier
            .subscribe(Box::new(move |value| {
                if let Some(notifier) = mapped_obs_notifier.upgrade() {
                    notifier.notify(value);
                }
            }));
        mapped_obs
            .notifier
            .on_mapped_obs_unsubscribe((&mapped_obs.notifier, handle).into());

        let mapped_reader = mapped_obs.reader();

        self.notifier.subscribe(Box::new(move |value: &T| {
            if let Some(mapped_notifier) = mapped_reader.notifier.upgrade() {
                mapped_notifier.unsubscribe_mapped_obs();
                let middle_obs = { f(value) };
                mapped_reader.value.replace(middle_obs.get().clone());
                if let Some(clean_up) = mapped_reader
                    .clone()
                    .subscribe_reader(Rc::downgrade(&middle_obs.notifier))
                {
                    mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                }
            }
        }));

        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self.notifier);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }
}

impl<T, V> Observable<V, true>
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

impl<T: 'static> Reader<T> {
    pub fn map_value<R: 'static>(self, f: MapFn<T, R>) -> Observable<R, false> {
        let initial_value = { f(&self.get()) };
        let mapped_obs = Observable::new(initial_value);

        let mapped_weak = mapped_obs.reader();
        if let Some(clean_up) = mapped_weak.subscribe_mapped_reader(self, f) {
            mapped_obs.on_cleanup(clean_up);
        }

        mapped_obs.into()
    }

    pub fn map_reader<R: Clone + 'static>(self, f: MapWeakObsFn<T, R>) -> Observable<R, false> {
        let middle_obs = { f(&self.get()) };
        let initial_value = { middle_obs.get().clone() };
        let mapped_obs = Observable::new(initial_value);

        if let Some(clean_up) = mapped_obs.reader().subscribe_reader(middle_obs.notifier) {
            mapped_obs.notifier.on_mapped_obs_unsubscribe(clean_up)
        }

        let mapped_weak = mapped_obs.reader();

        if let Some(self_notifier) = self.notifier.upgrade() {
            let handle = self_notifier.subscribe(Box::new(move |value: &T| {
                if let Some(mapped_notifier) = mapped_weak.notifier.upgrade() {
                    mapped_notifier.unsubscribe_mapped_obs();
                    let middle_obs = { f(value) };
                    mapped_weak.value.replace(middle_obs.get().clone());
                    if let Some(clean_up) =
                        mapped_weak.clone().subscribe_reader(middle_obs.notifier)
                    {
                        mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                    }
                }
            }));

            mapped_obs.on_cleanup((&self_notifier, handle));
        }

        mapped_obs.into()
    }

    fn subscribe_reader(self, reader_notifier: Weak<Notifier<T>>) -> Option<CleanUp>
    where
        T: Clone,
    {
        let notifier = reader_notifier.upgrade()?;
        let handle = notifier.subscribe(Box::new(move |value: &T| {
            if let Some(notifier) = self.notifier.upgrade() {
                self.value.replace(value.clone());
                notifier.notify(&self.value.borrow());
            }
        }));

        Some((&notifier, handle).into())
    }

    fn subscribe_mapped_reader<In: 'static>(
        self,
        reader: Reader<In>,
        f: MapFn<In, T>,
    ) -> Option<CleanUp> {
        let notifier = reader.notifier.upgrade()?;
        let handle = notifier.subscribe(Box::new(move |value: &In| {
            if let Some(notifier) = self.notifier.upgrade() {
                let mapped = f(value);
                self.value.replace(mapped);
                notifier.notify(&self.value.borrow());
            }
        }));

        Some((&notifier, handle).into())
    }
}

impl<T: 'static> From<(&Rc<Notifier<T>>, ListenerHandle)> for CleanUp {
    fn from((notifier, handle): (&Rc<Notifier<T>>, ListenerHandle)) -> Self {
        let weak_notifier = Rc::downgrade(notifier);

        let f: Box<dyn FnOnce()> = Box::new(move || {
            if let Some(notifier) = weak_notifier.upgrade() {
                notifier.unsubscribe(handle);
            }
        });

        CleanUp::from(f)
    }
}

impl<T> From<Observable<T, true>> for Observable<T, false> {
    fn from(obs: Observable<T>) -> Self {
        Observable {
            value: obs.value,
            notifier: obs.notifier,
        }
    }
}

pub enum MapObsResult<T> {
    Value(T),
    Weak(Reader<T>),
    Obs(Observable<T, false>),
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

    #[test]
    fn observable_map() {
        let obs1 = Observable::new(0);
        let obs2 = obs1.reader().map_value(Box::new(|n| 2 * n + 1));

        {
            assert_eq!(*obs1.get(), 0);
            assert_eq!(*obs2.get(), 1);
        }

        {
            obs1.set(1);
            assert_eq!(*obs1.get(), 1);
            assert_eq!(*obs2.get(), 3);
        }
    }
}
