use std::cell::{Cell, Ref, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

use crate::listener_set::{ListenerSet, Reader, Subscription};
use crate::{Pushable, Value};

pub struct Observable<T> {
    value: Rc<Value<T>>,
    listener_set: ListenerSet,
}

/// A reader that stores the present value - regardless of whether the writer is alive or not.
/// ValueReader is a handle to read the present value of an Observable
/// It does NOT keep that observable alive, but whenever that observable drops,
/// we will keep a copy of its last value
pub struct ValueReader<T> {
    value: Rc<Value<T>>,

    // Why is this here? Used only for cloning the ValueReader?
    reader: Reader,
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Value::rc(value),
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

impl<T> Observable<T> {
    pub fn set(&self, value: T) {
        self.value.set(value);
        self.listener_set.notify();
    }

    pub fn value_ref(&self) -> Ref<T> {
        self.value.get()
    }
}

impl<T: 'static> Observable<T> {
    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Subscription {
        let value = self.value.clone();
        self.listener_set.subscribe(move || cb(&value.get()))
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Subscription {
        let value = self.value.clone();
        self.listener_set.once(move || cb(&value.get()))
    }
}

impl<T: 'static> Observable<T> {
    pub fn map_value<R: 'static>(&self, f: impl Fn(&T) -> R + 'static) -> MapReader<R> {
        self.reader().map_value(f)
    }

    pub fn map_reader<R: Clone + 'static>(
        &self,
        f: impl Fn(&T) -> ValueReader<R> + 'static,
    ) -> MapReader<R> {
        self.reader().map_reader(f)
    }
}

impl<T, V> Observable<V>
where
    V: Pushable<Value = T>,
{
    pub fn push(&self, item: T)
    where
        T: 'static,
    {
        self.value.push(item);
        self.listener_set.notify();
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

impl<T: 'static> ValueReader<T> {
    pub fn map_value<R: 'static>(self, f: impl Fn(&T) -> R + 'static) -> MapReader<R> {
        MapReader::new(move |ctx| {
            let val = ctx.track(&self);
            f(&val)
        })
    }

    pub fn map_reader<R: Clone + 'static>(
        self,
        f: impl Fn(&T) -> ValueReader<R> + 'static,
    ) -> MapReader<R> {
        MapReader::new(move |ctx| {
            let t = ctx.track(&self);
            let r_reader = f(&t);
            let r_val = ctx.track(&r_reader);
            r_val.clone()
        })
    }
}

impl<T: 'static> ValueReader<T> {
    pub fn value_ref(&self) -> Ref<T> {
        self.value.get()
    }
    pub fn value(&self) -> Rc<Value<T>> {
        self.value.clone()
    }

    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Option<Subscription> {
        let value = self.value.clone();
        self.reader.subscribe(move || cb(&value.get()))
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Option<Subscription> {
        let value = self.value.clone();
        self.reader.once(move || cb(&value.get()))
    }
}
impl<T> Deref for ValueReader<T> {
    type Target = Reader;

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

/// MapReader is a handle to the latest output value of a map function.
/// It is essential the the MapReader (or its clones) be the only struct to have a strong reference
/// to said closure. This is because we want the closure to be able to close over owned/strong references
/// to its upstream ValueReaders WITHOUT creating a cycle.
///
/// The weak ref must be between the Observable and the map function. NOT inside the map function
pub struct MapReader<T> {
    value: Rc<Value<T>>,
    listener_set: ListenerSet,
    #[allow(dead_code)]
    downstreams: Rc<Downstreams>,
    #[allow(dead_code, clippy::type_complexity)]
    closure: Rc<dyn Fn(&MapReaderContext<T>) -> T>,
}

impl<T> MapReader<T> {
    pub fn new(f: impl Fn(&MapReaderContext<T>) -> T + 'static) -> Self {
        let listener_set = ListenerSet::default();
        let closure: Rc<MapReaderClosure<T>> = Rc::new(f);
        let downstreams = Rc::default();

        let value = Rc::new_cyclic(|weak| {
            let ctx = MapReaderContext {
                index: Cell::new(0),
                value: weak.clone(),
                my_reader: listener_set.reader(),
                downstreams: Rc::downgrade(&downstreams),
                closure: Rc::downgrade(&closure),
            };

            let first_value = closure(&ctx);
            Value::new(first_value)
        });

        MapReader {
            value,
            listener_set,
            downstreams,
            closure,
        }
    }

    pub fn value_ref(&self) -> Ref<T> {
        self.value.get()
    }
    pub fn value(&self) -> Rc<Value<T>> {
        self.value.clone()
    }

    pub fn value_reader(&self) -> ValueReader<T> {
        ValueReader {
            value: self.value.clone(),
            reader: self.listener_set.reader(),
        }
    }
    pub fn reader(&self) -> Reader {
        self.listener_set.reader()
    }
}

type Downstreams = RefCell<Vec<(Reader, Option<Subscription>)>>;
type MapReaderClosure<T> = dyn Fn(&MapReaderContext<T>) -> T;
pub struct MapReaderContext<T> {
    index: Cell<usize>,
    value: Weak<Value<T>>,
    my_reader: Reader,
    downstreams: Weak<Downstreams>,
    closure: Weak<MapReaderClosure<T>>,
}

impl<T: 'static> MapReaderContext<T> {
    pub fn track_reader(&self, reader: &Reader) {
        let index = self.index.get();
        let Some(downstreams) = self.downstreams.upgrade() else {
            return;
        };
        let mut list = downstreams.borrow_mut();
        if index < list.len() {
            if list[index].0 != *reader {
                let sub = reader.subscribe(self.subscription_closure());
                list[index] = (reader.clone(), sub);
            }
        } else {
            let sub = reader.subscribe(self.subscription_closure());
            list.push((reader.clone(), sub))
        }
        self.index.set(index + 1);
    }

    #[inline]
    pub fn track<'a, V>(&self, value_reader: &'a ValueReader<V>) -> Ref<'a, V> {
        self.track_reader(&value_reader.reader);
        value_reader.value.get()
    }

    fn clear_unused_readers(&self) {
        let Some(downstreams) = self.downstreams.upgrade() else {
            return;
        };
        let mut list = downstreams.borrow_mut();
        let new_len = self.index.get();
        if new_len < list.len() {
            list.resize_with(new_len, || unreachable!());
        }
    }
}
impl<T: 'static> MapReaderContext<T> {
    fn subscription_closure(&self) -> impl Fn() + 'static {
        let ctx = MapReaderContext {
            index: Cell::new(0),
            value: self.value.clone(),
            my_reader: self.my_reader.clone(),
            downstreams: self.downstreams.clone(),
            closure: self.closure.clone(),
        };
        move || {
            let Some(f) = ctx.closure.upgrade() else {
                return;
            };
            let Some(value) = ctx.value.upgrade() else {
                return;
            };

            if let Some(working_set) = ctx.my_reader.working_set() {
                let new_val = f(&ctx);
                ctx.clear_unused_readers();
                value.set(new_val);
                working_set.notify();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use crate::{Pushable, Subscription, ValueReader};

    use super::Observable;

    #[test]
    fn observable_vec_push() {
        let obs = Observable::new(vec![1, 2, 3]);

        let counter: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

        let _sub = {
            let counter = counter.clone();
            obs.subscribe(move |v: &Vec<u32>| {
                counter.replace(Some(v.len()));
            })
        };

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

        let _sub = {
            let counter = counter.clone();
            obs.subscribe(Box::new(move |v: &Wrapper<u32>| {
                *((*counter).borrow_mut()) = Some(v.0.len());
            }))
        };

        assert_eq!(*counter.borrow(), None);
        obs.push(0);
        assert_eq!(*counter.borrow(), Some(4));
    }

    #[test]
    fn observable_reactivity() {
        let obs = Observable::new("hello".to_owned());

        let counter_durable: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        let _sub_durable = {
            let counter_durable = counter_durable.clone();
            obs.subscribe(move |_: &String| {
                let mut ptr = (*counter_durable).borrow_mut();
                *ptr = match *ptr {
                    Some(c) => Some(c + 1),
                    None => Some(1),
                };
            })
        };

        let counter_once: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
        let _sub_once = {
            let counter_durable = counter_once.clone();
            obs.once(move |_: &String| {
                let mut ptr = (*counter_durable).borrow_mut();
                *ptr = match *ptr {
                    Some(_) => unreachable!(),
                    None => Some(1),
                };
            })
        };

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
    fn observable_map() {
        let obs1 = Observable::new(0);
        let map_reader = obs1.reader().map_value(|n| 2 * n + 1);

        {
            assert_eq!(*obs1.value_ref(), 0);
            assert_eq!(*map_reader.value_ref(), 1);
        }

        {
            obs1.set(1);
            assert_eq!(*obs1.value_ref(), 1);
            assert_eq!(*map_reader.value_ref(), 3);
        }
    }
    struct Dog {
        weight_kg: Observable<f32>,
    }

    impl Dog {
        // TODO impl Default for Observable<T> where D: Default
        // AND Into<Observable<T>> for T
        pub fn new(weight_kg: f32) -> Self {
            Dog {
                weight_kg: Observable::new(weight_kg),
            }
        }
        pub fn feed(&self) {
            let new_weight_kg = { *self.weight_kg.value_ref() + 0.1 };
            self.weight_kg.set(new_weight_kg);
        }
        pub fn weight_kg(&self) -> ValueReader<f32> {
            self.weight_kg.reader()
        }
    }

    struct Person {
        current_dog: Observable<Dog>,
    }

    #[test]
    fn basic_subscription() {
        struct App {
            rex: Dog,
            #[allow(dead_code)]
            sub: Subscription,
            pivot: Rc<Cell<f32>>,
        }

        impl App {
            fn new() -> Self {
                let rex = Dog::new(4.5);

                // NOTE: Subscription drops when App drops
                // QUESTION: Given that Subscription continues to live (even if inactive) after the writer drops
                //           Does it actually make sense to return Option/Result from subscribe?
                //           What's the difference between the writer going away before vs after we subscribe?

                let pivot: Rc<Cell<f32>> = Rc::default();
                let pivot1 = pivot.clone();
                let sub = rex
                    .weight_kg()
                    .subscribe(move |w| {
                        pivot1.set(*w);
                        Self::render(*w);
                    })
                    .unwrap();
                App { rex, sub, pivot }
            }
            fn render(w: f32) {
                println!("Rex weighs {}", w); // or self.sub.value - is the subscription also a reader?
            }
        }

        let app = App::new();
        assert_eq!(*app.rex.weight_kg.value_ref(), 4.5);
        assert_eq!(app.pivot.get(), 0.0);

        app.rex.weight_kg.set(6.5);
        assert_eq!(app.pivot.get(), 6.5);
    }

    #[test]
    fn mapped_subscription() {
        let person_obs = Person {
            current_dog: Observable::new(Dog::new(4.5)),
        };

        let dog_mapped_reader = person_obs
            .current_dog
            .reader()
            .map_reader(|p| p.weight_kg());
        assert_eq!(*dog_mapped_reader.value_ref(), 4.5);

        {
            person_obs.current_dog.value_ref().weight_kg.set(6.7);
        };
        assert_eq!(*dog_mapped_reader.value_ref(), 6.7);

        {
            let new_dog = Dog::new(10.0);
            person_obs.current_dog.set(new_dog);
        };
        assert_eq!(*dog_mapped_reader.value_ref(), 10.0);

        {
            person_obs.current_dog.value_ref().weight_kg.set(11.0);
        };
        assert_eq!(*dog_mapped_reader.value_ref(), 11.0);

        {
            person_obs.current_dog.value_ref().feed();
        };
        assert_eq!(*dog_mapped_reader.value_ref(), 11.1);
    }
}
