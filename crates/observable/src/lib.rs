//! Observables in rust
//! ## Example
//! ```
//!
//! ```

/// Public API.
mod clean_up;
mod listener_set;
mod observable;

// Reexport of the public API.
#[doc(inline)]
pub use crate::clean_up::*;
#[doc(inline)]
pub use crate::listener_set::*;
#[doc(inline)]
pub use crate::observable::*;

use std::cell::Ref;
pub trait Observe<T>: Sized {
    fn get(&self) -> Ref<T>;
    fn subscribe(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn once(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn unsubscribe(&self, handle: ListenerHandle) -> bool;
}
