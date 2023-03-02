use std::{cell::RefCell, rc::Rc};

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
    pub fn once(&self, cb: Box<dyn Fn(&T)>) -> ListenerHandle {
        self.0.borrow_mut().subscribe(Listener::Once(cb))
    }
    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        self.0.borrow_mut().unsubscribe(handle)
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

struct ListenerItem<T> {
    /// Monotonic id for use in the binary search
    id: usize,
    listener: Listener<T>,
}

impl<T> ListenerSet<T> {
    fn working_set(&mut self) -> WorkingSet<T> {
        // It's possible to add listeners while we are firing a listener
        // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions
        let mut working_set: Vec<Listener<T>> = Vec::with_capacity(self.items.len());

        let first_once_pos = 'first_once_pos: {
            for (index, item) in self.items.iter().enumerate() {
                match &item.listener {
                    Listener::Once(_) => break 'first_once_pos Some(index),
                    Listener::Durable(f) => {
                        working_set.push(Listener::Durable(f.clone()));
                    }
                }
            }
            None
        };

        // only moves durables if necessary while fills the working_set
        if let Some(first_once_pos) = first_once_pos {
            let items = unsafe {
                let items = &mut self.items as *mut Vec<ListenerItem<T>>;
                (*items).drain(first_once_pos..)
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
        }

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
                self.items.remove(offset);
                true
            }
            Err(_) => false,
        }
    }
}

pub enum Listener<T> {
    Once(Box<dyn Fn(&T)>),
    Durable(Rc<dyn Fn(&T)>),
}

pub struct ListenerHandle(usize);

pub(crate) struct WorkingSet<T>(Vec<Listener<T>>);

impl<T> WorkingSet<T> {
    pub(crate) fn notify(self, value: &T) {
        for listener in self.0 {
            match listener {
                Listener::Once(f) => f(value),
                Listener::Durable(f) => f(value),
            }
        }
    }
}
