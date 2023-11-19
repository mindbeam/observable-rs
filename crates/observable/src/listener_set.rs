use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

#[derive(Default)]
pub struct ListenerSet(RefCell<Inner>);

impl ListenerSet {
    pub fn notify(&self) {
        let working_set = self.working_set();

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        working_set.notify();
    }

    pub(crate) fn working_set(&self) -> WorkingSet {
        self.0.borrow_mut().working_set()
    }

    pub fn subscribe(&self, cb: impl Dispatch + 'static) -> Subscription {
        let cb: Rc<dyn Dispatch> = Rc::new(cb);
        self.subscribe_weak(Rc::downgrade(&cb));
        Subscription::new(cb)
    }
    pub fn once(&self, cb: impl FnOnce() + 'static) -> Subscription {
        let cb = RefCell::new(Some(cb));
        let cb: Rc<dyn Dispatch> = Rc::new(move || {
            if let Some(f) = cb.take() {
                f();
            }
        });
        self.once_weak(Rc::downgrade(&cb));
        Subscription::new(cb)
    }
    pub fn subscribe_weak(&self, cb: Weak<dyn Dispatch>) {
        self.0.borrow_mut().subscribe(Listener::Durable(cb));
    }
    pub fn once_weak(&self, cb: Weak<dyn Dispatch>) {
        self.0.borrow_mut().subscribe(Listener::Once(cb));
    }
    pub fn unsubscribe(&self, cb: Weak<dyn Dispatch>) {
        self.0.borrow_mut().unsubscribe(cb);
    }
}

#[derive(Default)]
struct Inner {
    items: Vec<Listener>,
}

impl Inner {
    fn working_set(&mut self) -> WorkingSet {
        // It's possible to add listeners while we are firing a listener
        // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions
        let mut working_set: Vec<WorkingItem> = Vec::new();

        self.items.retain(|item| match item {
            Listener::Once(f) => {
                working_set.push(f.clone());
                false
            }
            Listener::Durable(f) => match f.upgrade() {
                Some(_) => {
                    working_set.push(f.clone());
                    true
                }
                None => false,
            },
        });

        WorkingSet::new(working_set)
    }

    fn subscribe(&mut self, listener: Listener) {
        self.items.push(listener);
    }
    fn unsubscribe(&mut self, cb: Weak<dyn Dispatch>) {
        let Some(cb) = cb.upgrade() else { return };
        self.items.retain_mut(|item| {
            let f = match &item {
                Listener::Once(f) => f,
                Listener::Durable(f) => f,
            };
            let Some(f) = f.upgrade() else {
                return false;
            };

            Rc::ptr_eq(&f, &cb)
        });
    }
}

// Reader needs to keep this alive. That's basically it
enum Listener {
    Once(Weak<dyn Dispatch>),
    Durable(Weak<dyn Dispatch>),
}

pub type WorkingItem = Weak<dyn Dispatch>;

pub struct WorkingSet {
    items: Vec<WorkingItem>,
}
impl WorkingSet {
    pub(crate) fn new(items: Vec<WorkingItem>) -> Self {
        WorkingSet { items }
    }
}

impl WorkingSet {
    pub(crate) fn notify(self) {
        for item in self.items {
            if let Some(f) = item.upgrade() {
                f.dispatch()
            }
        }
    }
}

pub struct Subscription {
    #[allow(dead_code)]
    cb: Rc<dyn Dispatch>,
}
impl Subscription {
    pub fn new(cb: Rc<dyn Dispatch>) -> Self {
        Self { cb }
    }
}

pub trait Dispatch {
    fn dispatch(&self);
}
impl<Out, F: Fn() -> Out> Dispatch for F {
    fn dispatch(&self) {
        self();
    }
}
