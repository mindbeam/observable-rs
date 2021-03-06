use dyn_clone::DynClone;
use js_sys::Function;
use observable_rs::{ListenerHandle, Observable};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

// Traits for javascript-specific functionality around Observable<T>

/// This trait is necessary to support generic observables
/// which cannot themselves be exportable via wasm_bindgen
pub trait JsObserveBase {
    fn get_js(&self) -> JsValue;
    fn set_js(&self, value: JsValue);
    fn subscribe(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn once(&self, cb: Box<dyn Fn()>) -> ListenerHandle;
    fn unsubscribe(&self, handle: ListenerHandle) -> bool;
}
pub trait JsObserve: JsObserveBase + JsObserveMap + DynClone {}

pub trait JsObserveMap {
    fn map_js(&self, cb: Function) -> JsValue;
}

// TODO - Figure out why rust thinks this is unbound when we impl JsObserveBase for O where O: Observe<T>
impl<T> JsObserveBase for Observable<T>
where
    // O: Observe<T> + Clone,
    T: Serialize + DeserializeOwned,
{
    fn get_js(&self) -> JsValue {
        JsValue::from_serde(&*self.get()).unwrap()
    }

    fn set_js(&self, value: JsValue) {
        let value: T = JsValue::into_serde(&value).unwrap();
        self.set(value)
    }

    fn subscribe(&self, cb: Box<dyn Fn()>) -> ListenerHandle {
        Self::subscribe(&self, cb)
    }

    fn once(&self, cb: Box<dyn Fn()>) -> ListenerHandle {
        Self::once(&self, cb)
    }

    fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        Self::unsubscribe(&self, handle)
    }
}
