use std::cell::Ref;

use dyn_clone::DynClone;
use js_sys::Function;
use observable_rs::{ListenerHandle, Observable};
// use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

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

    fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        Self::unsubscribe(self, handle)
    }

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle;
    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle;
}

impl<T> JsObserve for Observable<T>
where
    T: Into<JsValue> + Clone,
{
    // type Target = T;

    fn subscribe(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Observable::subscribe(
            &self,
            Box::new(move |v: T| -> JsValue { cb(v.clone().into()) }),
        )
    }

    fn once(&self, cb: Box<dyn Fn(JsValue)>) -> ListenerHandle {
        Self::once(self, Box::new(move |v: T| cb(v.clone().into())))
    }

    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        let a: Ref<T> = self.get();
        (&*a).clone().into()
    }
}

impl<T> JsObserve for Observable<Vec<T>>
where
    T: Into<JsValue> + Clone,
{
    type Target = Vec<T>;

    // we need to be able provide a JS value (JS only has one value type)
    fn get_js(&self) -> JsValue {
        // let a: Ref<T> = self.get();
        // (&*a).clone().into()
        JsValue::UNDEFINED
    }
    fn map_js(&self, cb: Function) -> JsValue {
        let ar = js_sys::Array::new();

        for v in self.get().iter() {
            let ret = cb
                .call1(&JsValue::UNDEFINED, &JsValue::from_serde(v).unwrap())
                .unwrap();

            ar.push(&ret);
        }

        ar.into()
    }
}
