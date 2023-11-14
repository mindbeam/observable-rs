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

    pub fn subscribe(&self, cb: impl Fn() + 'static) -> Subscription {
        let cb: Rc<dyn Fn()> = Rc::new(cb);
        self.0
            .borrow_mut()
            .subscribe(Listener::Durable(Rc::downgrade(&cb)));
        Subscription::new(cb)
    }
    pub fn subscribe_rc(&self, cb: Rc<dyn Fn()>) -> Subscription {
        self.0
            .borrow_mut()
            .subscribe(Listener::Durable(Rc::downgrade(&cb)));
        Subscription::new(cb)
    }
    pub fn once(&self, cb: impl FnOnce() + 'static) -> Subscription {
        let cb = RefCell::new(Some(cb));
        let cb: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(f) = cb.take() {
                f();
            }
        });
        self.0
            .borrow_mut()
            .subscribe(Listener::Once(Rc::downgrade(&cb)));
        Subscription::new(cb)
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

        self.items.retain_mut(|item| match &item {
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

    pub fn subscribe(&mut self, listener: Listener) {
        self.items.push(listener);
    }
}

// Reader needs to keep this alive. That's basically it
enum Listener {
    Once(Weak<dyn Fn()>),
    Durable(Weak<dyn Fn()>),
}

pub type WorkingItem = Weak<dyn Fn()>;

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
                f()
            }
        }
    }
}

pub struct Subscription {
    #[allow(dead_code)]
    cb: Rc<dyn Fn()>,
}
impl Subscription {
    pub fn new(cb: Rc<dyn Fn()>) -> Self {
        Self { cb }
    }
}
