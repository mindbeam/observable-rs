mod utils;

use wasm_bindgen::prelude::*;
// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use observable_react::{collections::List, JsObservable};
use observable_rs::Observable;

#[wasm_bindgen(start)]
pub fn start() {
    log::set_logger(&wasm_bindgen_console_logger::DEFAULT_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    crate::utils::set_panic_hook();
}

#[allow(unused)]
#[wasm_bindgen]
pub fn create_rust_thing() -> RustThing {
    RustThing::default()
}

#[wasm_bindgen]
#[derive(Default)]
pub struct RustThing {
    things: usize,
    list: Observable<List<String>>,
}

#[wasm_bindgen]
impl RustThing {
    pub fn do_something(&mut self) {
        self.things += 1;
        self.list.push(format!("Thing {}", self.things));
    }
    pub fn get_the_list(&self) -> JsObservable {
        let list = self.list.reader();
        list.into()
    }
}
