use std::cell::Ref;

use dyn_clone::DynClone;
use js_sys::Function;
use observable_rs::{CleanUp, Reader, ValueReader};
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

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp>;
    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp>;
}

impl<T> JsObserve for ValueReader<T>
where
    T: Into<JsValue> + Clone + 'static,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<T> = self.get();
        (*a).clone().into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp> {
        Reader::subscribe(self, Box::new(move |v: &T| cb(v.clone().into()))).map(|sub| sub.into())
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp> {
        Reader::once(self, Box::new(move |v: &T| cb(v.clone().into()))).map(|sub| sub.into())
    }
}

impl<T: 'static> JsObserve for ValueReader<List<T>>
where
    T: Into<JsValue> + Clone,
{
    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<List<T>> = self.get();
        (&*a).into()
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp> {
        Reader::subscribe(self, Box::new(move |v: &List<T>| cb(v.into()))).map(|sub| sub.into())
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> Option<CleanUp> {
        Reader::once(self, Box::new(move |v: &List<T>| cb(v.into()))).map(|sub| sub.into())
    }
}
