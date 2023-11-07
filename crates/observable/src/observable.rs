use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::listener_set::{ListenerSet, Reader, Subscription};
use crate::CleanUp;

pub struct Observable<T, const E: bool = true> {
    value: Rc<RefCell<T>>,
    listener_set: ListenerSet<T>,
}

pub struct ValueReader<T> {
    value: Rc<RefCell<T>>,
    reader: Reader<T>,
}

impl<T> ValueReader<T> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }
    pub fn reader(&self) -> Reader<T> {
        self.reader.clone()
    }
}
impl<T> Deref for ValueReader<T> {
    type Target = Reader<T>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
impl<T> Clone for ValueReader<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            reader: self.reader.clone(),
        }
    }
}

impl<T> Observable<T, true> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            listener_set: ListenerSet::default(),
        }
    }

    // pub fn value(initial_value: T) -> (Writer<T>, Value<T>) {

    // }

    pub fn reader(&self) -> ValueReader<T> {
        ValueReader {
            value: self.value.clone(),
            reader: self.listener_set.reader(),
        }
    }
}

// Observable<T> = Observable<T, true>
// Observable<T, false> : mapped observable
impl<T> Observable<T, true> {
    pub fn set(&self, value: T) {
        self.value.replace(value);
        let r = self.value.borrow();
        self.listener_set.notify(&r);
    }
}

impl<T, const E: bool> Observable<T, E> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> Subscription<T> {
        self.listener_set.subscribe(cb)
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> Subscription<T> {
        self.listener_set.once(cb)
    }

    pub fn on_cleanup(&self, clean_up: impl Into<CleanUp>) {
        self.listener_set.on_cleanup(clean_up.into())
    }

    pub fn clean_up(&self) {
        self.listener_set.clean_up()
    }
}

type MapFn<T, R> = Box<dyn Fn(&T) -> R>;
type MapReaderFn<T, R> = MapFn<T, ValueReader<R>>;
type MapObsFn<T, R> = MapFn<T, Observable<R, false>>;

impl<T: 'static> Observable<T, false> {
    pub fn map_value<R: 'static>(self, f: MapFn<T, R>) -> Observable<R, false> {
        let initial_value = { f(&self.get()) };
        let mapped_obs = Observable::new(initial_value);

        let mapped_writer = mapped_obs.listener_set.writer();
        let self_notifier = self.listener_set;
        self_notifier.subscribe(Box::new(move |value| {
            let mapped = f(value);
            mapped_writer.write(&mapped);
        }));
        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self_notifier);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }

    pub fn map_reader<R: Clone + 'static>(self, f: MapReaderFn<T, R>) -> Observable<R, false> {
        let middle_obs = { f(&self.get()) };
        let initial_value = { middle_obs.get().clone() };
        let mapped_obs = Observable::new(initial_value);

        if let Some(sub) = mapped_obs.reader().subscribe_reader(middle_obs.reader) {
            mapped_obs.listener_set.on_mapped_obs_unsubscribe(sub)
        }

        let mapped_reader = mapped_obs.reader();

        self.listener_set.subscribe(Box::new(move |value: &T| {
            if let Some(mapped_notifier) = mapped_reader.parent() {
                mapped_notifier.cleanup_downstreams();
                let middle_obs = { f(value) };
                mapped_reader.value.replace(middle_obs.get().clone());
                if let Some(clean_up) = mapped_reader.clone().subscribe_reader(middle_obs.reader) {
                    mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                }
            }
        }));

        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self.listener_set);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }

    pub fn map_obs<R: Clone + 'static>(self, f: MapObsFn<T, R>) -> Observable<R, false> {
        let initial_mapped_obs = { f(&self.get()) };
        let mapped_obs: Observable<R> = Observable {
            value: initial_mapped_obs.value,
            listener_set: ListenerSet::default(),
        };

        let mapped_obs_writer = mapped_obs.listener_set.writer();
        let sub = initial_mapped_obs
            .listener_set
            .subscribe(Box::new(move |value| {
                mapped_obs_writer.write(value);
            }));
        mapped_obs
            .listener_set
            .on_mapped_obs_unsubscribe(sub.into());

        let mapped_reader = mapped_obs.reader();

        self.listener_set.subscribe(Box::new(move |value: &T| {
            if let Some(mapped_notifier) = mapped_reader.parent() {
                mapped_notifier.cleanup_downstreams();
                let middle_obs = { f(value) };
                mapped_reader.value.replace(middle_obs.get().clone());
                if let Some(clean_up) = mapped_reader
                    .clone()
                    .subscribe_reader(middle_obs.listener_set.reader())
                {
                    mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                }
            }
        }));

        let clean_up: Box<dyn FnOnce()> = Box::new(move || {
            drop(self.listener_set);
        });
        mapped_obs.on_cleanup(clean_up);

        mapped_obs.into()
    }
}

impl<T, V> Observable<V, true>
where
    V: Pushable<Value = T>,
{
    pub fn push(&self, item: T)
    where
        T: 'static,
    {
        {
            let mut ref_mut = self.value.borrow_mut();
            let vec = &mut *ref_mut;
            vec.push(item);
        }

        self.listener_set.notify(&self.value.borrow());
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

impl<T: 'static> ValueReader<T> {
    pub fn map_value<R: 'static>(self, f: MapFn<T, R>) -> Observable<R, false> {
        let initial_value = { f(&self.get()) };
        let mapped_obs = Observable::new(initial_value);

        let mapped_weak = mapped_obs.reader();
        if let Some(clean_up) = mapped_weak.subscribe_mapped_reader(self, f) {
            mapped_obs.on_cleanup(clean_up);
        }

        mapped_obs.into()
    }

    pub fn map_reader<R: Clone + 'static>(self, f: MapReaderFn<T, R>) -> Observable<R, false> {
        let middle_obs = { f(&self.get()) };
        let initial_value = { middle_obs.get().clone() };
        let mapped_obs = Observable::new(initial_value);

        if let Some(clean_up) = mapped_obs.reader().subscribe_reader(middle_obs.reader) {
            mapped_obs.listener_set.on_mapped_obs_unsubscribe(clean_up)
        }

        let mapped_reader = mapped_obs.reader();

        if let Some(self_listener_set) = self.reader.parent() {
            let sub = self_listener_set.subscribe(Box::new(move |value: &T| {
                if let Some(mapped_notifier) = mapped_reader.parent() {
                    mapped_notifier.cleanup_downstreams();
                    let middle_obs = { f(value) };
                    mapped_reader.value.replace(middle_obs.get().clone());
                    if let Some(clean_up) =
                        mapped_reader.clone().subscribe_reader(middle_obs.reader)
                    {
                        mapped_notifier.on_mapped_obs_unsubscribe(clean_up);
                    }
                }
            }));

            mapped_obs.on_cleanup(sub);
        }

        mapped_obs.into()
    }

    fn subscribe_reader(self, reader: Reader<T>) -> Option<CleanUp>
    where
        T: Clone,
    {
        let sub = reader.subscribe(Box::new(move |value: &T| {
            self.value.replace(value.clone());
            self.reader.writer().write(&self.value.borrow());
        }))?;

        Some(sub.into())
    }

    fn subscribe_mapped_reader<In: 'static>(
        self,
        value_reader: ValueReader<In>,
        f: MapFn<In, T>,
    ) -> Option<CleanUp> {
        let writer = self.writer();
        let sub = value_reader.subscribe(Box::new(move |value: &In| {
            if let Some(working_set) = writer.working_set() {
                let mapped = f(value);
                self.value.replace(mapped);
                working_set.notify(&self.value.borrow());
            }
        }))?;

        Some(sub.into())
    }
}

impl<T> From<Observable<T, true>> for Observable<T, false> {
    fn from(obs: Observable<T>) -> Self {
        Observable {
            value: obs.value,
            listener_set: obs.listener_set,
        }
    }
}

pub enum MapObsResult<T> {
    Value(T),
    Weak(ValueReader<T>),
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
