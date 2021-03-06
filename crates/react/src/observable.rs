use crate::{react::ReactComponent, traits::JsObserve};
use js_sys::Function;
use observable_rs::ListenerHandle;
use wasm_bindgen::{prelude::*, JsCast};

/// # Wrapper around JsObserve to which provides React binding convenience methods
/// The JsObserve trait, and this wrapper are necessary because wasm_bindgen cannot express generics at this time.
#[wasm_bindgen]
pub struct ReactObservable {
    obs: Box<dyn JsObserve>,
    bound_listener: Option<ListenerHandle>,
}

#[wasm_bindgen]
impl ReactObservable {
    pub fn get(&self) -> JsValue {
        self.obs.get_js()
    }
    pub fn map(&self, cb: Function) -> JsValue {
        self.obs.map_js(cb)
    }
    /// Bind this observable to a React component
    pub fn bind_component(&mut self, component: ReactComponent) {
        if let Some(_) = self.bound_listener {
            panic!("Can only bind to one component at a time")
        }

        let handle = self
            .obs
            .subscribe(Box::new(move || component.forceUpdate()));

        self.bound_listener = Some(handle);
    }
    pub fn unbind(&mut self) {
        if let Some(handle) = self.bound_listener.take() {
            self.obs.unsubscribe(handle);
        }
    }
    pub fn subscribe(&mut self, cb: Function) -> Function {
        log::info!("subscribe");
        let handle = self.obs.subscribe(Box::new(move || {
            cb.call0(&JsValue::UNDEFINED).unwrap();
        }));

        // Make a copy that the closure can hold on to
        let obs = dyn_clone::clone_box(&*self.obs);

        let unsub = Closure::once_into_js(Box::new(move || {
            log::info!("unsubscribe");

            obs.unsubscribe(handle);
        }) as Box<dyn FnOnce()>);

        unsub.unchecked_into()
    }
}

impl<O> From<O> for ReactObservable
where
    O: JsObserve + 'static + Sized,
{
    fn from(obs: O) -> Self {
        ReactObservable {
            obs: Box::new(obs),
            bound_listener: None,
        }
    }
}
