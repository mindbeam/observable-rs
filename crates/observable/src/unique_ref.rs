use std::{
    marker::PhantomData,
    ops::Deref,
    rc::{Rc, Weak},
};

#[derive(Default)]
pub struct UniqueRef<T: ?Sized>(Rc<T>);

impl<T> Deref for UniqueRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl<T> UniqueRef<T> {
    #[allow(dead_code)]
    pub fn new(value: T) -> UniqueRef<T> {
        UniqueRef(Rc::new(value))
    }
    #[allow(dead_code)]
    pub fn new_cyclic<F: FnOnce(&Weak<T>) -> T>(f: F) -> UniqueRef<T> {
        UniqueRef(Rc::new_cyclic(f))
    }
    pub fn downgrade(&self) -> WeakRef<T> {
        WeakRef(Rc::downgrade(&self.0))
    }
}

pub struct WeakRef<T>(Weak<T>);
impl<T> Clone for WeakRef<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> WeakRef<T> {
    pub fn upgrade(&self) -> Option<DataRef<'_, T>> {
        let data = self.0.upgrade()?;
        Some(DataRef {
            inner: data,
            phantom: PhantomData,
        })
    }
}
impl<T> std::fmt::Debug for WeakRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct DataRef<'a, T> {
    inner: Rc<T>,
    phantom: PhantomData<&'a T>,
}
impl<'a, T> Deref for DataRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T> PartialEq<WeakRef<T>> for WeakRef<T> {
    fn eq(&self, other: &WeakRef<T>) -> bool {
        let Some(rc1) = self.0.upgrade() else {
            return false;
        };
        let Some(rc2) = other.0.upgrade() else {
            return false;
        };
        Rc::ptr_eq(&rc1, &rc2)
    }
}
#[cfg(test)]
mod test {
    use crate::{unique_ref::UniqueRef, ListenerSet};

    #[test]
    fn reader_equality() {
        let strong: UniqueRef<ListenerSet> = UniqueRef::default();
        let weak1 = strong.downgrade();
        let weak2 = strong.downgrade();
        assert_eq!(weak1, weak2);

        drop(strong);
        assert_ne!(weak1, weak2);
    }
}
