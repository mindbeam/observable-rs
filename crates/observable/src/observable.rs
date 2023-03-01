use std::{cell::Ref, cell::RefCell, rc::Rc};

struct ListenerSet<T> {
    nextid: usize,
    items: Vec<ListenerItem<T>>,
}

impl<T> Default for ListenerSet<T> {
    fn default() -> Self {
        ListenerSet {
            nextid: 0,
            items: Vec::new(),
        }
    }
}

struct ListenerItem<T> {
    /// Monotonic id for use in the binary search
    id: usize,
    listener: Listener<T>,
}

impl<T> ListenerSet<T> {
    fn working_set(&mut self) -> Vec<Listener<T>> {
        // It's possible to add listeners while we are firing a listener
        // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions
        let mut working_set: Vec<Listener<T>> = Vec::with_capacity(self.items.len());

        let items = unsafe {
            let items = &mut self.items as *mut Vec<ListenerItem<T>>;
            (*items).drain(..)
        };

        for item in items {
            match &item.listener {
                Listener::Once(_) => working_set.push(item.listener),
                Listener::Durable(f) => {
                    working_set.push(Listener::Durable(f.clone()));
                    self.items.push(item);
                }
            }
        }

        working_set
    }
}

pub enum Listener<T> {
    Once(Box<dyn Fn(&T)>),
    Durable(Rc<dyn Fn(&T)>),
}

pub struct ListenerHandle(usize);

pub struct Observable<T> {
    value: Rc<RefCell<T>>,
    listener_set: Rc<RefCell<ListenerSet<T>>>,
}

// Implemented manually because `T` does not need to be Clone
impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Observable {
            value: self.value.clone(),
            listener_set: self.listener_set.clone(),
        }
    }
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            listener_set: Rc::new(RefCell::new(ListenerSet::default())),
        }
    }

    fn notify(&self) {
        let working_set = { self.listener_set.borrow_mut().working_set() };

        let r = self.get();

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        for listener in working_set {
            match listener {
                Listener::Once(f) => f(&r),
                Listener::Durable(f) => f(&r),
            }
        }
    }

    fn _subscribe(&self, listener: Listener<T>) -> ListenerHandle {
        let mut listener_set = self.listener_set.borrow_mut();

        let id = listener_set.nextid;
        listener_set.nextid += 1;
        listener_set.items.push(ListenerItem { id, listener });
        ListenerHandle(id)
    }

    // impl<T> Set<T> for Observable<T> {
    pub fn set(&self, value: T) {
        {
            *(self.value.borrow_mut()) = value;
        };

        self.notify();
    }
    // }
    // impl<T> Observe<T> for Observable<T> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }
    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        let listener = Listener::Durable(cb.into());
        self._subscribe(listener)
    }
    pub fn once(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        let listener = Listener::Once(cb);
        self._subscribe(listener)
    }

    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        let mut listener_set = self.listener_set.borrow_mut();

        // Find the current listener offset
        match listener_set
            .items
            .binary_search_by(|probe| probe.id.cmp(&handle.0))
        {
            Ok(offset) => {
                listener_set.items.remove(offset);
                true
            }
            Err(_) => false,
        }
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
        self.notify();
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

    use crate::{observable::ListenerSet, Pushable};

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
        let obs: ListenerSet<Wrapper<Vec<u32>>> = ListenerSet::default();

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
