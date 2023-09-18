use std::{cell::RefCell, mem, rc::Rc};

pub struct Notifier<T>(RefCell<ListenerSet<T>>);

impl<T> Default for Notifier<T> {
    fn default() -> Self {
        Self(RefCell::default())
    }
}

impl<T> Notifier<T> {
    pub fn notify(&self, value: &T) {
        let working_set = { self.0.borrow_mut().working_set() };

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        working_set.notify(value);
    }

    pub fn subscribe(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.0.borrow_mut().subscribe(Listener::Durable(cb.into()))
    }
    pub fn once(&self, cb: Box<dyn FnOnce(&T)>) -> ListenerHandle {
        self.0.borrow_mut().subscribe(Listener::Once(cb))
    }
    pub fn on_cleanup(&self, clean_up: CleanUp) {
        self.0.borrow_mut().subscribe(Listener::OnCleanUp(clean_up));
    }
    pub(crate) fn on_mapped_obs_unsubscribe(&self, clean_up: CleanUp) {
        self.0
            .borrow_mut()
            .subscribe(Listener::MapObsUnsubscription(clean_up));
    }
    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        self.0.borrow_mut().unsubscribe(handle)
    }
    pub(crate) fn clean_up(&self) {
        self.0.borrow_mut().items.clear();
    }

    pub(crate) fn unsubscribe_mapped_obs(&self) {
        self.0.borrow_mut().unsubscribe_mapped_obs()
    }
}

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

pub struct CleanUp(Option<Box<dyn FnOnce()>>);

impl From<Box<dyn FnOnce()>> for CleanUp {
    fn from(value: Box<dyn FnOnce()>) -> Self {
        CleanUp(Some(value))
    }
}

impl Drop for CleanUp {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

struct ListenerItem<T> {
    /// Monotonic id for use in the binary search
    id: usize,
    listener: Listener<T>,
}

impl<T> ListenerSet<T> {
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
            Listener::MapObsUnsubscription(_) => true,
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

    fn unsubscribe_mapped_obs(&mut self) {
        self.items.retain_mut(|item| match &item.listener {
            Listener::MapObsUnsubscription(_) => {
                if let Listener::MapObsUnsubscription(cleanup) =
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

pub enum Listener<T> {
    Once(Box<dyn FnOnce(&T)>),
    Durable(Rc<dyn Fn(&T)>),
    OnCleanUp(CleanUp),
    MapObsUnsubscription(CleanUp),
    None,
}

pub enum WorkingItem<T> {
    Once(Box<dyn FnOnce(&T)>),
    Durable(Rc<dyn Fn(&T)>),
}

#[derive(Debug, Clone)]
pub struct ListenerHandle(usize);

pub(crate) struct WorkingSet<T>(Vec<WorkingItem<T>>);

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
