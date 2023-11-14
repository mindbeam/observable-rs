use std::cell::Ref;

use dyn_clone::DynClone;
use js_sys::Function;
use observable_rs::{Reader, Subscription};
// use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

use crate::collections::List;

// Traits for javascript-specific functionality around Observable<T>

/// This trait is necessary to support generic observables
/// which cannot themselves be exportable via wasm_bindgen
pub trait JsObserve: DynClone {
    // type Target: Clone + Into<JsValue>;

    fn get_js(&self) -> JsValue;

    /// The default implementation of map is to call the closure once
    /// with the output of .get - other types may call the closure multiple
    /// times for different sub-values
    fn map_js(&self, cb: Function) -> JsValue {
        let ar = js_sys::Array::new();
        let ret = cb.call1(&JsValue::UNDEFINED, &self.get_js()).unwrap();
        ar.push(&ret);
        ar.into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription>;
    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription>;
}

impl<T> JsObserve for Reader<T>
where
    T: Into<JsValue> + Clone + 'static,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<T> = self.value();
        (*a).clone().into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription> {
        self.subscribe(move |v: &T| cb(v.clone().into()))
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription> {
        self.once(move |v: &T| cb(v.clone().into()))
    }
}

impl<T: 'static> JsObserve for Reader<List<T>>
where
    T: Into<JsValue> + Clone,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<List<T>> = self.value();
        (&*a).into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription> {
        self.subscribe(move |v: &List<T>| cb(v.into()))
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<Subscription> {
        self.once(move |v: &List<T>| cb(v.into()))
    }
}
