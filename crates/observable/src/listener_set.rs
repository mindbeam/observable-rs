use std::{
    cell::RefCell,
    ops::Deref,
    rc::{Rc, Weak},
};

use crate::CleanUp;

pub struct ListenerSet<T>(Rc<ListenerSetBase<T>>);
impl<T> Default for ListenerSet<T> {
    fn default() -> Self {
        Self(Rc::new(ListenerSetBase(Default::default())))
    }
}
impl<T> Deref for ListenerSet<T> {
    type Target = ListenerSetBase<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> ListenerSet<T> {
    pub fn writer(&self) -> Writer<T> {
        Writer(Rc::downgrade(&self.0))
    }
    pub fn reader(&self) -> Reader<T> {
        Reader(Rc::downgrade(&self.0))
    }
}

pub struct ListenerSetBase<T>(RefCell<Inner<T>>);

impl<T> ListenerSetBase<T> {
    pub(crate) fn notify(&self, value: &T) {
        let working_set = self.working_set();

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        working_set.notify(value);
    }

    pub(crate) fn working_set(&self) -> WorkingSet<T> {
        self.0.borrow_mut().working_set()
    }

    pub fn on_cleanup(&self, clean_up: CleanUp) {
        self.0.borrow_mut().subscribe(Listener::OnCleanUp(clean_up));
    }
    pub(crate) fn on_mapped_obs_unsubscribe(&self, clean_up: CleanUp) {
        self.0
            .borrow_mut()
            .subscribe(Listener::Downstream(clean_up));
    }
    pub(crate) fn clean_up(&self) {
        self.0.borrow_mut().items.clear();
    }

    pub(crate) fn cleanup_downstreams(&self) {
        self.0.borrow_mut().cleanup_downstreams()
    }
    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Subscription<T> {
        let cb: Rc<dyn FnMut(&T)> = Rc::new(cb);
        self.0
            .borrow_mut()
            .subscribe(Listener::Durable(Rc::downgrade(&cb)));
        Subscription::new(cb)
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Subscription<T> {
        let mut cb = Some(cb);
        let cb = move |val: &T| {
            if let Some(f) = cb.take() {
                f(val);
            }
        };
        let cb: Rc<dyn FnMut(&T)> = Rc::new(cb);
        self.0
            .borrow_mut()
            .subscribe(Listener::Once(Rc::downgrade(&cb)));
        Subscription::new(cb)
    }
}

struct Inner<T> {
    items: Vec<Listener<T>>,
}

impl<T> Default for Inner<T> {
    fn default() -> Self {
        Inner { items: Vec::new() }
    }
}

impl<T> Inner<T> {
    fn working_set(&mut self) -> WorkingSet<T> {
        // It's possible to add listeners while we are firing a listener
        // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions
        let mut working_set: Vec<WorkingItem<T>> = Vec::with_capacity(self.items.len());

        self.items.retain_mut(|item| match &item {
            Listener::Once(f) => {
                if f.upgrade().is_some() {
                    working_set.push(f.clone());
                }
                false
            }
            Listener::Durable(f) => match f.upgrade() {
                Some(_) => {
                    working_set.push(f.clone());
                    true
                }
                None => false,
            },
            Listener::OnCleanUp(_) => true,
            Listener::Downstream(_) => true,
        });

        WorkingSet::new(working_set)
    }

    pub fn subscribe(&mut self, listener: Listener<T>) {
        self.items.push(listener);
    }

    fn cleanup_downstreams(&mut self) {
        self.items
            .retain_mut(|item| !matches!(item, Listener::Downstream(_)))
    }
}

// Reader needs to keep this alive. That's basically it
enum Listener<T> {
    Once(Weak<dyn FnMut(&T)>),
    Durable(Weak<dyn FnMut(&T)>),
    OnCleanUp(CleanUp),
    Downstream(CleanUp),
}

#[derive(Debug, Clone, Copy)]
pub struct ListenerHandle(usize);

pub type WorkingItem<T> = Weak<dyn FnMut(&T)>;

pub struct WorkingSet<T> {
    items: Vec<WorkingItem<T>>,
}
impl<T> WorkingSet<T> {
    pub(crate) fn new(items: Vec<WorkingItem<T>>) -> Self {
        WorkingSet { items }
    }
}

impl<T> WorkingSet<T> {
    pub(crate) fn notify(self, value: &T) {
        for item in self.items {
            if let Some(rc) = item.upgrade() {
                unsafe {
                    let f = Rc::as_ptr(&rc) as *mut dyn FnMut(&T);
                    (*f)(value);
                }
            }
        }
    }
}

pub struct Writer<T>(Weak<ListenerSetBase<T>>);
impl<T> Clone for Writer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Writer<T> {
    pub fn write(&self, value: &T) -> bool {
        match self.working_set() {
            Some(working_set) => {
                working_set.notify(value);
                true
            }
            None => false,
        }
    }

    pub fn working_set(&self) -> Option<WorkingSet<T>> {
        self.0.upgrade().map(|rc| ListenerSet(rc).working_set())
    }
}

pub struct Reader<T>(Weak<ListenerSetBase<T>>);
impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Reader<T> {
    pub fn writer(&self) -> Writer<T> {
        Writer(self.0.clone())
    }

    pub fn subscribe(&self, cb: impl Fn(&T) + 'static) -> Option<Subscription<T>> {
        let listener_set = ListenerSet(self.0.upgrade()?);
        let sub = listener_set.subscribe(cb);
        Some(sub)
    }
    pub fn once(&self, cb: impl FnOnce(&T) + 'static) -> Option<Subscription<T>> {
        let listener_set = ListenerSet(self.0.upgrade()?);
        let sub = listener_set.once(cb);
        Some(sub)
    }
    pub fn parent(&self) -> Option<ListenerSet<T>> {
        self.0.upgrade().map(|rc| ListenerSet(rc))
    }
}

pub struct Subscription<T> {
    #[allow(dead_code)]
    cb: Rc<dyn FnMut(&T)>,
}
impl<T> Subscription<T> {
    pub fn new(cb: Rc<dyn FnMut(&T)>) -> Self {
        Self { cb }
    }
}
impl<T: 'static> From<Subscription<T>> for CleanUp {
    fn from(subscription: Subscription<T>) -> Self {
        let cb = move || {
            drop(subscription);
        };
        let f: Box<dyn FnOnce()> = Box::new(cb);
        CleanUp::from(f)
    }
}
