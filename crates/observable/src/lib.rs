//! Observables in rust
//! ## Example
//! ```
//!
//! ```

/// Public API.
mod notifier;
mod observable;

// Reexport of the public API.
#[doc(inline)]
pub use crate::notifier::*;
#[doc(inline)]
pub use crate::observable::*;

use std::cell::Ref;
pub trait Set<T>: Sized {
    fn set(&self, value: T);
}
pub trait Observe<T>: Sized {
    fn get(&self) -> Ref<T>;
    fn subscribe(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn once(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn unsubscribe(&self, handle: ListenerHandle) -> bool;
}
