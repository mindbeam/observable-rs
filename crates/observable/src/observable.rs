use std::{cell::Ref, cell::RefCell, rc::Rc};

pub trait Observe<T>: Sized {
    fn get(&self) -> Ref<T>;
    fn subscribe(&self, cb: Box<dyn Fn()>);
}

pub enum Listener {
    Once(Box<dyn FnMut()>),
    Durable(Rc<RefCell<Box<dyn FnMut()>>>),
}

#[derive(Clone)]
pub struct Observable<T> {
    value: Rc<RefCell<T>>,
    listeners: Rc<RefCell<Option<Vec<Listener>>>>,
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            listeners: Default::default(),
        }
    }

    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }

    // TODO fn mutate, returning changed: bool

    pub fn set(&self, value: T) {
        {
            *(self.value.borrow_mut()) = value;
        };

        self.notify();
    }
    fn notify(&self) {
        let mut working_set: Vec<Listener>;
        {
            if let Some(ref mut listeners) = *self.listeners.borrow_mut() {
                // It's possible to add listeners while we are firing a listener
                // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions

                working_set = Vec::with_capacity(listeners.len());

                // Take all Listener::Once entries, and clone the others
                let mut i = 0;
                while i != listeners.len() {
                    match listeners[i] {
                        Listener::Once(_) => {
                            // Just take it
                            working_set.push(listeners.remove(i));
                        }
                        Listener::Durable(ref f) => {
                            working_set.push(Listener::Durable(f.clone()));
                            i += 1;
                        }
                    }
                }
            } else {
                return;
            }
        }

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        for listener in working_set {
            match listener {
                Listener::Once(mut f) => f(),
                Listener::Durable(f) => {
                    (f.borrow_mut())();
                }
            }
        }
    }

    pub fn subscribe(&self, cb: Box<dyn FnMut()>) {
        let mut listeners = self.listeners.borrow_mut();

        let listener = Listener::Durable(Rc::new(RefCell::new(cb)));
        match *listeners {
            Some(ref mut listeners) => {
                listeners.push(listener);
            }
            None => *listeners = Some(vec![listener]),
        }
    }
    pub fn once(&self, cb: Box<dyn FnMut()>) {
        let mut listeners = self.listeners.borrow_mut();

        let listener = Listener::Once(cb);
        match *listeners {
            Some(ref mut listeners) => {
                listeners.push(listener);
            }
            None => *listeners = Some(vec![listener]),
        }
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

impl<T> Observable<Vec<T>> {
    pub fn push(&mut self, item: T) {
        self.value.borrow_mut().push(item);
        self.notify();
    }
}
