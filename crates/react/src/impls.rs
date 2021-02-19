use js_sys::Function;
use observable_rs::{Observable, Observe};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;

use super::traits::{JsObserve, JsObserveBase, JsObserveMap};

macro_rules! impl_jsobservemap {
    ($($t:ty),+) => {
        $(impl JsObserve for Observable<$t> {})*
        $(impl JsObserveMap for Observable<$t> {
            fn map_js(&self, cb: Function) -> JsValue {
                let ar = js_sys::Array::new();
                let ret = cb.call1(&JsValue::UNDEFINED, &self.get_js()).unwrap();
                ar.push(&ret);
                ar.into()
            }
        })*
    }
}

impl_jsobservemap!(bool, u32);

impl<T> JsObserve for Observable<Vec<T>> where T: Serialize + DeserializeOwned + Clone {}
impl<T> JsObserveMap for Observable<Vec<T>>
where
    T: Serialize + DeserializeOwned + Clone,
{
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
