use std::convert::TryInto;

use js_sys::Function;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "react")]
extern "C" {
    /// Binding to React.useState
    #[wasm_bindgen(js_name = useState)]
    fn js_use_state(initial_value: JsValue) -> js_sys::Array;

    /// Binding to React.useEffect
    #[wasm_bindgen(js_name = useEffect)]
    fn js_use_effect(effect: &Closure<dyn FnMut()>, bindings: js_sys::Array) -> js_sys::Array;

    /// Binding to React.useReducer
    #[wasm_bindgen(js_name = useReducer)]
    fn js_use_reducer(reducer: &Closure<dyn FnMut()>, initial_value: JsValue) -> js_sys::Array;
}

// Duck type for React components
#[wasm_bindgen]
extern "C" {
    pub type ReactComponent;

    #[wasm_bindgen(structural, method)]
    pub fn forceUpdate(this: &ReactComponent);
}

/// Oxidized interface to React.useState
pub fn use_state<T>(initial_value: T) -> (T, impl Fn(T))
where
    T: Serialize + DeserializeOwned,
{
    #[allow(unused_unsafe)]
    let jsa = unsafe { js_use_state(JsValue::from_serde(&initial_value).unwrap()) };

    let current = jsa.get(0).into_serde().unwrap();
    let update: Function = jsa.get(1).try_into().unwrap();

    let cb = move |value: T| {
        // unimplemented!()
        update
            .call1(&JsValue::UNDEFINED, &JsValue::from_serde(&value).unwrap())
            .unwrap();
    };

    (current, cb)
}
