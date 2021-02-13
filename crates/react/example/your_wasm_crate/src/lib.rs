use observable_react::ReactObservable;
use observable_rs::Observable;
use wasm_bindgen::prelude::*;

#[allow(unused)]
#[wasm_bindgen]
pub fn create_rust_thing() -> RustThing {
    RustThing::default()
}

#[wasm_bindgen]
#[derive(Clone, Default)]
pub struct RustThing {
    things: usize,
    list: Observable<Vec<String>>,
}

#[wasm_bindgen]
impl RustThing {
    pub fn do_something(&mut self) {
        self.things += 1;
        self.list.push(format!("Thing {}", self.things));
    }
    pub fn get_the_list(&self) -> ReactObservable {
        self.list.clone().into()
    }
}
