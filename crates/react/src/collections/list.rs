use std::ops::Deref;

use observable_rs::Pushable;
use wasm_bindgen::JsValue;

pub struct List<T>(Vec<T>);

impl<T> Deref for List<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> From<Vec<T>> for List<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T> Pushable for List<T> {
    type Value = T;

    fn push(&mut self, value: Self::Value) {
        self.0.push(value)
    }
}

impl<T> From<&List<T>> for JsValue
where
    T: Into<JsValue> + Clone,
{
    fn from(value: &List<T>) -> Self {
        let array = js_sys::Array::new();
        for v in value.0.iter() {
            let v = v.clone();
            let v: JsValue = v.into();
            array.push(&v);
        }
        array.into()
    }
}
