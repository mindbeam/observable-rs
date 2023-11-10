use std::{
    cell::RefCell,
    ops::Deref,
    rc::{Rc, Weak},
};

#[derive(Default)]
pub struct ListenerSet(Rc<ListenerSetBase>);

impl Deref for ListenerSet {
    type Target = ListenerSetBase;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl ListenerSet {
    pub fn reader(&self) -> Reader {
        Reader(Rc::downgrade(&self.0))
    }
}

#[derive(Default)]
pub struct ListenerSetBase(RefCell<Inner>);

impl ListenerSetBase {
    pub(crate) fn notify(&self) {
        let working_set = self.working_set();

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        working_set.notify();
    }

    pub(crate) fn working_set(&self) -> WorkingSet {
        self.0.borrow_mut().working_set()
    }

    pub fn subscribe(&self, cb: impl Fn() + 'static) -> Subscription {
        let cb: Rc<dyn FnMut()> = Rc::new(cb);
        self.0
            .borrow_mut()
            .subscribe(Listener::Durable(Rc::downgrade(&cb)));
        Subscription::new(cb)
    }
    pub fn once(&self, cb: impl FnOnce() + 'static) -> Subscription {
        let mut cb = Some(cb);
        let cb: Rc<dyn FnMut()> = Rc::new(move || {
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
    Once(Weak<dyn FnMut()>),
    Durable(Weak<dyn FnMut()>),
}

pub type WorkingItem = Weak<dyn FnMut()>;

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
            if let Some(rc) = item.upgrade() {
                unsafe {
                    let f = Rc::as_ptr(&rc) as *mut dyn FnMut();
                    (*f)();
                }
            }
        }
    }
}

pub struct Writer(Weak<ListenerSetBase>);
impl Clone for Writer {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Reader(Weak<ListenerSetBase>);

impl Reader {
    pub fn subscribe(&self, cb: impl Fn() + 'static) -> Option<Subscription> {
        let sub = self.0.upgrade()?.subscribe(cb);
        Some(sub)
    }
    pub fn once(&self, cb: impl FnOnce() + 'static) -> Option<Subscription> {
        let sub = self.0.upgrade()?.once(cb);
        Some(sub)
    }
    pub fn working_set(&self) -> Option<WorkingSet> {
        self.0.upgrade().map(|ls| ls.working_set())
    }
}
impl PartialEq<Reader> for Reader {
    fn eq(&self, other: &Reader) -> bool {
        let Some(rc1) = self.0.upgrade() else {
            return false;
        };
        let Some(rc2) = other.0.upgrade() else {
            return false;
        };
        Rc::ptr_eq(&rc1, &rc2)
    }
}
impl Eq for Reader {}

pub struct Subscription {
    #[allow(dead_code)]
    cb: Rc<dyn FnMut()>,
}
impl Subscription {
    pub fn new(cb: Rc<dyn FnMut()>) -> Self {
        Self { cb }
    }
}

#[cfg(test)]
mod test {
    use crate::ListenerSet;

    #[test]
    fn reader_equality() {
        let listener_set = ListenerSet::default();
        let reader1 = listener_set.reader();
        let reader2 = listener_set.reader();
        assert_eq!(reader1, reader2);

        drop(listener_set);
        assert_ne!(reader1, reader2);
    }
}
