use std::{
    cell::RefCell,
    mem,
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

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> Subscription<T> {
        let handle = self.0.subscribe(cb);
        Subscription::new(self.writer(), handle)
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> Subscription<T> {
        let handle = self.0.once(cb);
        Subscription::new(self.writer(), handle)
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
    fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.0.borrow_mut().subscribe(Listener::Durable(cb.into()))
    }
    fn once(&self, cb: Box<dyn FnOnce(&T)>) -> ListenerHandle {
        self.0.borrow_mut().subscribe(Listener::Once(cb))
    }
}

struct Inner<T> {
    nextid: usize,
    items: Vec<ListenerItem<T>>,
}

impl<T> Default for Inner<T> {
    fn default() -> Self {
        Inner {
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

impl<T> Inner<T> {
    fn working_set(&mut self) -> WorkingSet<T> {
        // It's possible to add listeners while we are firing a listener
        // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions
        let mut working_set: Vec<WorkingItem<T>> = Vec::with_capacity(self.items.len());

        self.items.retain_mut(|item| match &item.listener {
            Listener::Once(_) => {
                if let Listener::Once(f) = mem::replace(&mut item.listener, Listener::None) {
                    working_set.push(WorkingItem::Once(f));
                }
                false
            }
            Listener::Durable(f) => {
                working_set.push(WorkingItem::Durable(f.clone()));
                true
            }
            Listener::OnCleanUp(_) => true,
            Listener::Downstream(_) => true,
            Listener::None => false,
        });

        WorkingSet(working_set)
    }

    pub fn subscribe(&mut self, listener: Listener<T>) -> ListenerHandle {
        let id = self.nextid;
        self.nextid += 1;
        self.items.push(ListenerItem { id, listener });
        ListenerHandle(id)
    }

    pub fn unsubscribe(&mut self, handle: ListenerHandle) -> bool {
        // Find the current listener offset
        match self.items.binary_search_by(|probe| probe.id.cmp(&handle.0)) {
            Ok(offset) => {
                self.items[offset].listener = Listener::None;
                true
            }
            Err(_) => false,
        }
    }

    fn cleanup_downstreams(&mut self) {
        self.items.retain_mut(|item| match &item.listener {
            Listener::Downstream(_) => {
                if let Listener::Downstream(cleanup) =
                    mem::replace(&mut item.listener, Listener::None)
                {
                    drop(cleanup);
                }
                false
            }
            Listener::None => false,
            _ => true,
        })
    }
}

// Reader needs to keep this alive. That's basically it
enum Listener<T> {
    Once(Box<dyn FnOnce(&T)>),
    Durable(Rc<dyn Fn(&T)>),
    OnCleanUp(CleanUp),
    Downstream(CleanUp),
    None, // HACK for cleaning memory
}

#[derive(Debug, Clone, Copy)]
pub struct ListenerHandle(usize);

pub enum WorkingItem<T> {
    Once(Box<dyn FnOnce(&T)>),
    Durable(Rc<dyn Fn(&T)>),
}

pub struct WorkingSet<T>(Vec<WorkingItem<T>>);

impl<T> WorkingSet<T> {
    pub(crate) fn notify(self, value: &T) {
        for listener in self.0 {
            match listener {
                WorkingItem::Once(f) => f(value),
                WorkingItem::Durable(f) => f(value),
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

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> Option<Subscription<T>> {
        let listener_set = ListenerSet(self.0.upgrade()?);
        let sub = listener_set.subscribe(cb);
        Some(sub)
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> Option<Subscription<T>> {
        let listener_set = ListenerSet(self.0.upgrade()?);
        let sub = listener_set.once(cb);
        Some(sub)
    }
    pub fn parent(&self) -> Option<ListenerSet<T>> {
        self.0.upgrade().map(|rc| ListenerSet(rc))
    }
}

pub struct Subscription<T> {
    listener_set: Weak<ListenerSetBase<T>>,
    handle: ListenerHandle,
}
impl<T> Subscription<T> {
    pub fn new(writer: Writer<T>, handle: ListenerHandle) -> Self {
        Self {
            listener_set: writer.0,
            handle,
        }
    }
    pub fn cancel(&self) {
        if let Some(inner) = self.listener_set.upgrade() {
            inner.0.borrow_mut().unsubscribe(self.handle);
        }
    }
}
impl<T: 'static> From<Subscription<T>> for CleanUp {
    fn from(subscription: Subscription<T>) -> Self {
        let cb = move || {
            subscription.cancel();
            drop(subscription);
        };
        let f: Box<dyn FnOnce()> = Box::new(cb);
        CleanUp::from(f)
    }
}
