use std::cell::Ref;

use dyn_clone::DynClone;
use js_sys::Function;
use observable_rs::{ListenerHandle, Observable};
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

    fn unsubscribe(&self, handle: ListenerHandle) -> bool;

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle;
    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle;
}

impl<T> JsObserve for Observable<T>
where
    T: Into<JsValue> + Clone,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<T> = self.get();
        (*a).clone().into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Observable::subscribe(self, Box::new(move |v: &T| cb(v.clone().into())))
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Observable::once(self, Box::new(move |v: &T| cb(v.clone().into())))
    }

    fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        Observable::unsubscribe(self, handle)
    }
}

impl<T> JsObserve for Observable<List<T>>
where
    T: Into<JsValue> + Clone,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<List<T>> = self.get();
        (&*a).into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Observable::subscribe(self, Box::new(move |v: &List<T>| cb(v.into())))
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Observable::once(self, Box::new(move |v: &List<T>| cb(v.into())))
    }

    fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        Observable::unsubscribe(self, handle)
    }
}
