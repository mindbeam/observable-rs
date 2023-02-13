// use js_sys::Function;
// use observable_rs::Observable;
// use serde::{de::DeserializeOwned, Serialize};
// use wasm_bindgen::JsValue;

// use super::traits::{JsObserve, JsObserveBase, JsObserveMap};

// macro_rules! impl_jsobservemap {
//     ($($t:ty),+) => {
//         $(impl JsObserve for Observable<$t> {})*
//         $(impl JsObserveMap for Observable<$t> {
//             fn map_js(&self, cb: Function) -> JsValue {
//                 let ar = js_sys::Array::new();
//                 let ret = cb.call1(&JsValue::UNDEFINED, &self.get_js()).unwrap();
//                 ar.push(&ret);
//                 ar.into()
//             }
//         })*
//     }
// }

// impl_jsobservemap!(
//     bool,
//     u32,
//     u64,
//     usize,
//     i32,
//     i64,
//     isize,
//     String,
//     Option<String>
// );
