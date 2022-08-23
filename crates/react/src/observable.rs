use crate::{react::ReactComponent, traits::JsObserve};
use observable_rs::ListenerHandle;
use wasm_bindgen::{prelude::*, JsValue};

/// # Wrapper around JsObserve to which provides React binding convenience methods
/// The JsObserve trait, and this wrapper are necessary because wasm_bindgen cannot express generics at this time.
#[wasm_bindgen]
pub struct ReactObservable {
    obs: Box<dyn JsObserve>,
    bound_listener: Option<ListenerHandle>,
}

impl ReactObservable {
    pub fn new(obs: Box<dyn JsObserve>) -> Self {
        ReactObservable {
            obs,
            bound_listener: None,
        }
    }
}

#[wasm_bindgen]
impl ReactObservable {
    pub fn get(&self) -> JsValue {
        self.obs.get_js()
    }
    pub fn map(&self, cb: js_sys::Function) -> JsValue {
        self.obs.map_js(cb)
    }
    /// Bind this observable to a React component
    /// for class-based react components
    pub fn bind_component(&mut self, component: ReactComponent) {
        if self.bound_listener.is_some() {
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
    pub fn subscribe(
        &mut self,
        cb: js_sys::Function,
        // TODO: ChangeContext contract from TS?
    ) -> js_sys::Function {
        let handle = self.obs.subscribe(Box::new(move || {
            cb.call0(&JsValue::UNDEFINED).unwrap();
        }));

        // Make a copy that the closure can hold on to
        let obs = dyn_clone::clone_box(&*self.obs);

        let unsub = Closure::once_into_js(Box::new(move || {
            obs.unsubscribe(handle);
        }) as Box<dyn FnOnce()>);

        unsub.into()
    }

    pub fn destroy(&self) {
        // TODO
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        self.obs.get_js()
    }

    pub fn load(&self) -> js_sys::Promise {
        js_sys::Promise::resolve(&JsValue::null())
    }
}

impl<O> From<O> for ReactObservable
where
    O: JsObserve + 'static + Sized,
{
    fn from(obs: O) -> Self {
        ReactObservable::new(Box::new(obs))
    }
}
