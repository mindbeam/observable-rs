//! Export and bind Observables to react applications using WASM
//! ## Example
//! ```
//!
//! ```

pub mod collections;
pub mod impls;
pub mod react;
pub mod traits;

use crate::traits::JsObserve;
use wasm_bindgen::{prelude::*, JsValue};

/// # Wrapper around Observable<T> for usage in javascript/typescript
/// ```
///
/// # use std::rc::Rc;
/// # use observable_react::JsObservable;
/// # use observable_rs::Observable;
/// # use serde::Serialize;
/// # use wasm_bindgen::prelude::*;
/// # use std::cell::RefCell;
///
/// #[derive(Clone, Default)]
/// #[wasm_bindgen]
/// pub struct CatState { cats: usize };
/// impl CatState {
///   fn new (cats: usize) -> Self {
///     CatState{cats}
///   }
/// }
/// #[wasm_bindgen]
/// impl CatState {
///   #[wasm_bindgen(getter)]
///   pub fn cats(&self) -> usize {
///     self.cats
///   }
/// }
///
/// #[derive(Default, Clone, Serialize)]
/// pub struct Bar(pub Vec<usize>);
///
/// impl Into<JsValue> for Bar {
///     fn into(self) -> JsValue {
///         JsValue::from_serde(&self).unwrap()
///     }
/// }
///
/// let obs = Observable::new(CatState::new(1));
/// let obsJs: JsObservable = obs.clone().into();
/// let lastRustCats: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
/// let lrc = lastRustCats.clone();
/// obs.subscribe(Box::new(move |v|{
///   *(lrc.borrow_mut()) = Some(v.cats);
///   println!("Rust Cats: {}", v.cats);
/// }));
/// // In JS:
/// // catStateJs.subscribe(() => console.log(`JS Cats ${catstate.cats}`));
///
/// // We are not presently firing the listener for initial state
/// assert_eq!(*(lastRustCats.borrow()), None);
/// obs.set(CatState::new(7));
/// assert_eq!(*(lastRustCats.borrow()), Some(7));
///
/// // Both Rust Cats and JS Cats logs are printed
///
/// let barObsJs: JsObservable = Observable::new(Bar::default()).into();
/// let strObsJs: JsObservable = Observable::new(String::from("Meow")).into();
/// let intObsJs: JsObservable = Observable::new(123).into();
/// let fltObsJs: JsObservable = Observable::new(123.0).into();
/// ```
#[wasm_bindgen]
pub struct JsObservable {
    obs: Box<dyn JsObserve>,
}

impl JsObservable {
    pub fn new(obs: Box<dyn JsObserve>) -> Self {
        JsObservable { obs }
    }
}

#[wasm_bindgen]
impl JsObservable {
    pub fn get(&self) -> JsValue {
        self.obs.get_js()
    }
    pub fn map(&self, cb: js_sys::Function) -> JsValue {
        self.obs.map_js(cb)
    }
    pub fn subscribe(
        &mut self,
        cb: js_sys::Function,
        // TODO: ChangeContext contract from TS?
    ) -> js_sys::Function {
        let handle = self.obs.subscribe(Box::new(move |v: JsValue| {
            cb.call1(&JsValue::UNDEFINED, &v).unwrap();
        }));

        // Make a copy that the closure can hold on to
        let obs = dyn_clone::clone_box(&*self.obs);

        let unsub = Closure::once_into_js(Box::new(move || {
            obs.unsubscribe(handle);
        }) as Box<dyn FnOnce()>);

        unsub.into()
    }

    pub fn destroy(&self) {
        // NOOP. Call the free() method instead
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        self.obs.get_js()
    }

    pub fn load(&self) -> js_sys::Promise {
        // TODO implement loaders in observable_rs
        js_sys::Promise::resolve(&JsValue::null())
    }
}

impl<O> From<O> for JsObservable
where
    O: JsObserve + 'static + Sized,
{
    fn from(obs: O) -> Self {
        JsObservable::new(Box::new(obs))
    }
}
