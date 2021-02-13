use js_sys::Function;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

use observable_rs::Observable;

// Traits for javascript-specific functionality around Observable<T>

pub trait JsObserveBase {
    fn get_js(&self) -> JsValue;
    fn set_js(&self, value: JsValue);
    fn subscribe_js(&self, cb: Box<dyn FnMut()>);
    fn once_js(&self, cb: Box<dyn FnMut()>);
}

pub trait JsObserve: JsObserveBase + JsObserveMap {}

pub trait JsObserveMap {
    fn map_js(&self, cb: Function) -> JsValue;
}

impl<T> JsObserveBase for Observable<T>
where
    T: Serialize + DeserializeOwned,
{
    fn get_js(&self) -> JsValue {
        JsValue::from_serde(&*self.get()).unwrap()
    }

    fn set_js(&self, value: JsValue) {
        let value: T = JsValue::into_serde(&value).unwrap();
        self.set(value)
    }

    fn subscribe_js(&self, cb: Box<dyn FnMut()>) {
        self.subscribe(cb)
    }
    fn once_js(&self, cb: Box<dyn FnMut()>) {
        self.once(cb)
    }
}
