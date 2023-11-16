use std::cell::{Cell, Ref, RefCell};
use std::rc::{Rc, Weak};

use crate::listener_set::Subscription;
use crate::unique_ref::{UniqueRef, WeakRef};
use crate::{ListenerSet, Pushable, Value};

pub struct Observable<T> {
    value: Rc<Value<T>>,
    listener_set: UniqueRef<ListenerSet>,
}

/// A reader that stores the present value - regardless of whether the writer is alive or not.
/// Reader is a handle to read the present value of an Observable
/// It does NOT keep that observable alive, but whenever that observable drops,
/// we will keep a copy of its last value
pub struct Reader<T> {
    value: Rc<Value<T>>,

    // Why is this here? Used only for cloning the Reader?
    listener_set: WeakRef<ListenerSet>,
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Value::rc(value),
            listener_set: UniqueRef::default(),
        }
    }
    pub fn reader(&self) -> Reader<T> {
        Reader {
            value: self.value.clone(),
            listener_set: self.listener_set.downgrade(),
        }
    }
}

impl<T> Observable<T> {
    pub fn set(&self, value: T) {
        self.value.set(value);
        self.listener_set.notify();
    }

    pub fn value(&self) -> Ref<T> {
        self.value.get()
    }
    pub fn value_cloned(&self) -> T
    where
        T: Clone,
    {
        self.value.get().clone()
    }
}

impl<T> Observable<T> {
    pub fn on_updated(&self, cb: impl Fn() + 'static) -> Subscription {
        self.listener_set.subscribe(cb)
    }
    pub fn force_notify(&self) {
        self.listener_set.notify()
    }
}

impl<T: 'static> Observable<T> {
    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Subscription {
        self.reader().subscribe(cb).unwrap()
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Subscription {
        self.reader().once(cb).unwrap()
    }
}

impl<T: 'static> Observable<T> {
    pub fn map_value<R: 'static>(&self, f: impl Fn(&T) -> R + 'static) -> MapReader<R> {
        self.reader().map_value(f)
    }

    pub fn map_reader<R: Clone + 'static>(
        &self,
        f: impl Fn(&T) -> Reader<R> + 'static,
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

impl<T: 'static> Reader<T> {
    pub fn map_value<R: 'static>(self, f: impl Fn(&T) -> R + 'static) -> MapReader<R> {
        use crate::map_obs;
        let a = self;
        map_obs!(f, a)
    }

    pub fn map_reader<R: Clone + 'static>(
        self,
        f: impl Fn(&T) -> Reader<R> + 'static,
    ) -> MapReader<R> {
        MapReader::new_dyn(move |ctx| {
            let t = ctx.track(&self);
            let r_reader = f(&t);
            let r_val = ctx.track(&r_reader);
            r_val.clone()
        })
    }

    pub fn reader(self) -> Self {
        self
    }
}

impl<T> Reader<T> {
    pub fn value(&self) -> Ref<T> {
        self.value.get()
    }
    pub fn value_cloned(&self) -> T
    where
        T: Clone,
    {
        self.value.get().clone()
    }
    pub fn split(self) -> (Rc<Value<T>>, WeakRef<ListenerSet>) {
        (self.value, self.listener_set)
    }
}
impl<T: 'static> Reader<T> {
    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Option<Subscription> {
        let value = Rc::downgrade(&self.value);
        let sub = self.listener_set.upgrade()?.subscribe(move || {
            if let Some(value) = value.upgrade() {
                cb(&value.get())
            }
        });
        Some(sub)
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Option<Subscription> {
        let value = Rc::downgrade(&self.value);
        let sub = self.listener_set.upgrade()?.once(move || {
            if let Some(value) = value.upgrade() {
                cb(&value.get())
            }
        });
        Some(sub)
    }
}
impl<T> Reader<T> {
    pub fn on_updated(&self, cb: impl Fn() + 'static) -> Option<Subscription> {
        let sub = self.listener_set.upgrade()?.subscribe(cb);
        Some(sub)
    }
    pub fn force_notify(&self) {
        if let Some(ls) = self.listener_set.upgrade() {
            ls.notify()
        }
    }
}
impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            listener_set: self.listener_set.clone(),
        }
    }
}

/// MapReader is a handle to the latest output value of a map function.
/// It is essential the the MapReader (or its clones) be the only struct to have a strong reference
/// to said closure. This is because we want the closure to be able to close over owned/strong references
/// to its upstream Readers WITHOUT creating a cycle.
///
/// The weak ref must be between the Observable and the map function. NOT inside the map function
pub struct MapReader<T> {
    value: Rc<Value<T>>,
    listener_set: UniqueRef<ListenerSet>,
    #[allow(dead_code, clippy::type_complexity)]
    closure: Rc<dyn Fn()>,
}

impl<T> From<(Rc<Value<T>>, UniqueRef<ListenerSet>, Rc<dyn Fn()>)> for MapReader<T> {
    fn from(
        (value, listener_set, closure): (Rc<Value<T>>, UniqueRef<ListenerSet>, Rc<dyn Fn()>),
    ) -> Self {
        MapReader {
            value,
            listener_set,
            closure,
        }
    }
}
impl<T> MapReader<T> {
    pub fn value(&self) -> Ref<T> {
        self.value.get()
    }
    pub fn value_cloned(&self) -> T
    where
        T: Clone,
    {
        self.value.get().clone()
    }

    pub fn reader(&self) -> Reader<T> {
        Reader {
            value: self.value.clone(),
            listener_set: self.listener_set.downgrade(),
        }
    }
    pub fn listener_set(&self) -> WeakRef<ListenerSet> {
        self.listener_set.downgrade()
    }
}
impl<T: 'static> MapReader<T> {
    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Subscription {
        self.reader().subscribe(cb).unwrap()
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Subscription {
        self.reader().once(cb).unwrap()
    }
}
impl<T> MapReader<T> {
    pub fn on_updated(&self, cb: impl Fn() + 'static) -> Subscription {
        self.listener_set.subscribe(cb)
    }
    pub fn force_notify(&self) {
        self.listener_set.notify()
    }
}
impl<T: 'static> MapReader<T> {
    pub fn new_dyn(f: impl Fn(&DynMapReaderContext<T>) -> T + 'static) -> Self {
        let listener_set: UniqueRef<ListenerSet> = UniqueRef::default();
        let closure: Rc<DynMapReaderClosure<T>> = Rc::new(f);
        let downstreams = Rc::default();

        let value = Rc::new_cyclic(|weak| {
            let ctx = DynMapReaderContext {
                index: Cell::new(0),
                value: weak.clone(),
                my_ls: listener_set.downgrade(),
                downstreams: Rc::downgrade(&downstreams),
                closure: Rc::downgrade(&closure),
            };

            let first_value = closure(&ctx);
            Value::new(first_value)
        });

        MapReader {
            value,
            listener_set,
            closure: Rc::new(move || {
                #[allow(clippy::unnecessary_operation)]
                (&downstreams, &closure);
            }),
        }
    }
}

/// Maps one or many observers into a new one
/// ```
/// use observable_rs::{Observable, map_obs};
///
/// let obs1: Observable<u32> = Observable::new(1);
/// let obs2: Observable<u32> = Observable::new(2);
///
/// let obs = map_obs!(|a: &u32, b: &u32| {*a + *b}, obs1, obs2);
/// assert_eq!(*obs.value(), 3);
///
/// obs1.set(3);
/// assert_eq!(*obs.value(), 5);
///
/// obs2.set(4);
/// assert_eq!(*obs.value(), 7);
/// ```
#[macro_export]
macro_rules! map_obs {
    ($cb:expr, $($obs:ident),+) => {{
        use std::rc::Rc;
        use $crate::unique_ref::{UniqueRef, WeakRef};
        use $crate::{ListenerSet, Value, Reader, MapReader};

        let mut listener_set_list: Vec<WeakRef<ListenerSet>> = Vec::new();

        $(let $obs = {
            let reader: Reader<_> = $obs.reader();
            let (value, listener_set) = reader.split();
            listener_set_list.push(listener_set);
            value
        };)+
        let listener_set: UniqueRef<ListenerSet> = UniqueRef::default();
        #[allow(clippy::redundant_closure_call)]
        let value = $cb($(&*$obs.get(),)*);
        let value = Value::rc(value);

        let closure: Rc<dyn Fn()> = {
            let listener_set = listener_set.downgrade();
            let value = Rc::downgrade(&value);
            Rc::new(move || {
                let Some(reader_value) = value.upgrade() else {
                    return;
                };
                let Some(listener_set) = listener_set.upgrade() else {
                    return;
                };

                #[allow(clippy::redundant_closure_call)]
                let value = $cb($(&*$obs.get(),)*);
                reader_value.set(value);
                listener_set.notify();
            })
        };
        let weak_closure = Rc::downgrade(&closure);
        for ls in listener_set_list.into_iter() {
            if let Some(listener_set) = ls.upgrade() {
                let closure = weak_closure.clone();
                listener_set.subscribe_weak(closure);
            }
        }

        MapReader::from((value, listener_set, closure,))
    }};
}

type Downstreams = RefCell<Vec<(WeakRef<ListenerSet>, Option<Subscription>)>>;

type DynMapReaderClosure<T> = dyn Fn(&DynMapReaderContext<T>) -> T;
pub struct DynMapReaderContext<T> {
    index: Cell<usize>,
    value: Weak<Value<T>>,
    my_ls: WeakRef<ListenerSet>,
    downstreams: Weak<Downstreams>,
    closure: Weak<DynMapReaderClosure<T>>,
}

impl<T: 'static> DynMapReaderContext<T> {
    pub fn track_reader(&self, listener_set: &WeakRef<ListenerSet>) {
        let index = self.index.get();
        let Some(downstreams) = self.downstreams.upgrade() else {
            return;
        };
        let mut list = downstreams.borrow_mut();
        if index < list.len() {
            if list[index].0 != *listener_set {
                let sub = listener_set
                    .upgrade()
                    .map(|ls| ls.subscribe(self.subscription_closure()));
                list[index] = (listener_set.clone(), sub);
            }
        } else {
            let sub = listener_set
                .upgrade()
                .map(|ls| ls.subscribe(self.subscription_closure()));
            list.push((listener_set.clone(), sub))
        }
        self.index.set(index + 1);
    }

    #[inline]
    pub fn track<'a, V>(&self, reader: &'a Reader<V>) -> Ref<'a, V> {
        self.track_reader(&reader.listener_set);
        reader.value.get()
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
impl<T: 'static> DynMapReaderContext<T> {
    fn subscription_closure(&self) -> impl Fn() + 'static {
        let ctx = DynMapReaderContext {
            index: Cell::new(0),
            value: self.value.clone(),
            my_ls: self.my_ls.clone(),
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

            if let Some(listener_set) = ctx.my_ls.upgrade() {
                let new_val = f(&ctx);
                ctx.clear_unused_readers();
                value.set(new_val);
                listener_set.notify();
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

    use crate::{Pushable, Reader, Subscription};

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
            assert_eq!(*obs1.value(), 0);
            assert_eq!(*map_reader.value(), 1);
        }

        {
            obs1.set(1);
            assert_eq!(*obs1.value(), 1);
            assert_eq!(*map_reader.value(), 3);
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
            let new_weight_kg = { *self.weight_kg.value() + 0.1 };
            self.weight_kg.set(new_weight_kg);
        }
        pub fn weight_kg(&self) -> Reader<f32> {
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
        assert_eq!(*app.rex.weight_kg.value(), 4.5);
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
        assert_eq!(*dog_mapped_reader.value(), 4.5);

        {
            person_obs.current_dog.value().weight_kg.set(6.7);
        };
        assert_eq!(*dog_mapped_reader.value(), 6.7);

        {
            let new_dog = Dog::new(10.0);
            person_obs.current_dog.set(new_dog);
        };
        assert_eq!(*dog_mapped_reader.value(), 10.0);

        {
            person_obs.current_dog.value().weight_kg.set(11.0);
        };
        assert_eq!(*dog_mapped_reader.value(), 11.0);

        {
            person_obs.current_dog.value().feed();
        };
        assert_eq!(*dog_mapped_reader.value(), 11.1);
    }
}

#[cfg(test)]
mod view_mode_example {
    use std::rc::{Rc, Weak};

    use crate::{MapReader, Observable, Reader};

    trait ViewModel {
        type Parent: ViewModel;
        fn parent(&self) -> Weak<Self::Parent>;
    }
    impl ViewModel for () {
        type Parent = ();
        fn parent(&self) -> Weak<Self::Parent> {
            let rc = Rc::new(());
            Rc::downgrade(&rc)
        }
    }

    struct TopicSpace {
        member: Rc<Member>,
        clip_box: Observable<BoundingBox>,
    }
    impl ViewModel for TopicSpace {
        type Parent = ();
        fn parent(&self) -> Weak<Self::Parent> {
            let rc = Rc::new(());
            Rc::downgrade(&rc)
        }
    }
    impl TopicSpace {
        pub fn new() -> Rc<TopicSpace> {
            let clip_box = Observable::new(BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 200.0,
            });
            let clip_box_reader = clip_box.reader();
            Rc::new_cyclic(move |weak| TopicSpace {
                clip_box,
                member: Member::new(weak.clone(), clip_box_reader),
            })
        }
    }

    struct Member {
        parent: Weak<TopicSpace>,
        override_clip_box: Observable<Option<OverrideBoundingBox>>,
        clip_box: MapReader<BoundingBox>,
    }
    impl ViewModel for Member {
        type Parent = TopicSpace;

        fn parent(&self) -> Weak<Self::Parent> {
            self.parent.clone()
        }
    }
    impl Member {
        pub fn new(
            parent: Weak<TopicSpace>,
            ts_clip_box_reader: Reader<BoundingBox>,
        ) -> Rc<Member> {
            let override_clip_box = Observable::new(None);

            let clip_box = map_obs!(
                |ts_clip_box: &BoundingBox, override_boundig_box: &Option<OverrideBoundingBox>| {
                    match override_boundig_box.as_ref() {
                        Some(override_bounding_box) => {
                            ts_clip_box.override_with(override_bounding_box)
                        }
                        None => ts_clip_box.clone(),
                    }
                },
                ts_clip_box_reader,
                override_clip_box
            );

            Rc::new(Member {
                parent,
                override_clip_box,
                clip_box,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct BoundingBox {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    }
    #[derive(Debug, Clone, PartialEq)]
    struct OverrideBoundingBox {
        x: Option<f32>,
        y: Option<f32>,
        height: Option<f32>,
        width: Option<f32>,
    }
    impl BoundingBox {
        pub fn override_with(&self, override_bounding_box: &OverrideBoundingBox) -> BoundingBox {
            BoundingBox {
                x: override_bounding_box.x.unwrap_or(self.x),
                y: override_bounding_box.y.unwrap_or(self.y),
                width: override_bounding_box.width.unwrap_or(self.width),
                height: override_bounding_box.height.unwrap_or(self.height),
            }
        }
    }

    #[test]
    fn viewmodel_based_on_observables() {
        let ts = TopicSpace::new();

        let bb1 = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 200.0,
        };

        {
            let ts_clip_box = ts.clip_box.value();
            let member_overrive_clip_box = ts.member.override_clip_box.value();
            let member_clip_box = ts.member.clip_box.value();
            assert_eq!(*ts_clip_box, bb1);
            assert_eq!(*member_overrive_clip_box, None);
            assert_eq!(*member_clip_box, bb1);
        }

        let bb2 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
        };
        ts.clip_box.set(bb2.clone());
        {
            let ts_clip_box = ts.clip_box.value();
            let member_overrive_clip_box = ts.member.override_clip_box.value();
            let member_clip_box = ts.member.clip_box.value();
            assert_eq!(*ts_clip_box, bb2);
            assert_eq!(*member_overrive_clip_box, None);
            assert_eq!(*member_clip_box, bb2);
        }

        let obb1 = OverrideBoundingBox {
            x: None,
            y: None,
            width: None,
            height: None,
        };
        ts.member.override_clip_box.set(Some(obb1.clone()));
        {
            let ts_clip_box = ts.clip_box.value();
            let member_overrive_clip_box = ts.member.override_clip_box.value();
            let member_clip_box = ts.member.clip_box.value();
            assert_eq!(*ts_clip_box, bb2);
            assert_eq!(*member_overrive_clip_box, Some(obb1.clone()));
            assert_eq!(*member_clip_box, bb2);
        }

        let obb2 = OverrideBoundingBox {
            x: None,
            y: Some(30.0),
            width: Some(50.0),
            height: None,
        };
        let bb3 = BoundingBox {
            x: 10.0,
            y: 30.0,
            width: 50.0,
            height: 200.0,
        };
        ts.member.override_clip_box.set(Some(obb2.clone()));
        {
            let ts_clip_box = ts.clip_box.value();
            let member_overrive_clip_box = ts.member.override_clip_box.value();
            let member_clip_box = ts.member.clip_box.value();
            assert_eq!(*ts_clip_box, bb2);
            assert_eq!(*member_overrive_clip_box, Some(obb2.clone()));
            assert_eq!(*member_clip_box, bb3);
        }
    }
}
