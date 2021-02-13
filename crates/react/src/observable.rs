use crate::{react::ReactComponent, traits::JsObserve};
use js_sys::Function;
use wasm_bindgen::prelude::*;

/// Wrapper around JsObserve to which provides React binding convenience methods
#[wasm_bindgen]
pub struct ReactObservable(Box<dyn JsObserve>);

#[wasm_bindgen]
impl ReactObservable {
    pub fn get(&self) -> JsValue {
        self.0.get_js()
    }
    pub fn map(&self, cb: Function) -> JsValue {
        self.0.map_js(cb)
    }
    pub fn bind_component(&self, component: ReactComponent) {
        self.0
            .subscribe_js(Box::new(move || component.forceUpdate()))
    }
    pub fn observe(self) -> Self {
        // let (ct, update) = crate::react::use_state(0u32);

        // // TODO 4 - using once has partly solved our duplicate binding problem,
        // // insofar as duplicates will be cleared when the observable updates
        // // Unfortunately there might be other triggers causing the functional
        // // component to re-render, and thus we will still get duplicates unless
        // // we implement some sort of explicit deduplication
        // self.0.once_js(Box::new(move || {
        //     update(ct + 1);
        // }));
        // self

        // This is dumb. Increasinly not a fan of React hooks
        let [_, forceUpdate] = crate::react::use_reducer(|x: u32| x + 1, 0);
        crate::react::use_effect(
            || {
                let obs = self.clone();
                self.subscribe(|| forceUpdate());
                //     ||{
                //  self.unsubscribe
                //     }
            },
            [],
        );
        return self;
    }
}

impl<O> From<O> for ReactObservable
where
    O: JsObserve + 'static + Sized,
{
    fn from(obs: O) -> Self {
        ReactObservable(Box::new(obs))
    }
}
