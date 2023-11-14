//! Observables in rust
//! ## Example
//! ```
//!
//! ```

/// Public API.
mod listener_set;
mod observable;
mod pushable;
pub mod unique_ref;
mod value;

// Reexport of the public API.
#[doc(inline)]
pub use crate::listener_set::*;
#[doc(inline)]
pub use crate::observable::*;
#[doc(inline)]
pub use crate::pushable::*;
#[doc(inline)]
pub use crate::value::*;

use std::cell::Ref;
pub trait Observe<T>: Sized {
    fn value_ref(&self) -> Ref<T>;
    fn subscribe(&self, cb: Box<dyn Fn()>) -> Subscription;
    fn once(&self, cb: Box<dyn Fn()>) -> Subscription;
}
