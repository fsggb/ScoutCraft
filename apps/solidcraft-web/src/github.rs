use std::cell::RefCell;

use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast as _, JsValue, closure::Closure};
use web_sys::{CustomEvent, CustomEventInit, Event};

thread_local! {
    static OPENED: RefCell<Vec<(String, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

const OPEN_EVENT: &str = "scout-github-open";
const SAVE_EVENT: &str = "scout-github-save";

pub fn install() {
    let Some(window) = web_sys::window() else { return };
    let callback = Closure::<dyn FnMut(Event)>::new(|event: Event| {
        let detail = event.dyn_ref::<CustomEvent>().map(CustomEvent::detail).unwrap_or(JsValue::NULL);
        let Some(name) = Reflect::get(&detail, &"name".into()).ok().and_then(|v| v.as_string()) else { return };
        let Ok(bytes) = Reflect::get(&detail, &"bytes".into()) else { return };
        OPENED.with(|queue| queue.borrow_mut().push((name, Uint8Array::new(&bytes).to_vec())));
    });
    let _ = window.add_event_listener_with_callback(OPEN_EVENT, callback.as_ref().unchecked_ref());
    callback.forget();
}

pub fn drain() -> Vec<(String, Vec<u8>)> {
    OPENED.with(|queue| std::mem::take(&mut *queue.borrow_mut()))
}

pub fn offer(name: &str, mime: &str, bytes: &[u8]) {
    let Some(window) = web_sys::window() else { return };
    let detail = Object::new();
    let _ = Reflect::set(&detail, &"name".into(), &name.into());
    let _ = Reflect::set(&detail, &"mime".into(), &mime.into());
    let _ = Reflect::set(&detail, &"bytes".into(), &Uint8Array::from(bytes).into());
    let init = CustomEventInit::new();
    init.set_detail(&detail.into());
    if let Ok(event) = CustomEvent::new_with_event_init_dict(SAVE_EVENT, &init) {
        let _ = window.dispatch_event(&event);
    }
}
