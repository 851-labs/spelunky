//! What the browser build needs from the page: fetched files, the URL's query and somewhere to show
//! errors. The page is `crates/app/web/index.html`; `scripts/web` builds and serves it.

use crate::window::{Control, Playback, UserEvent};
use std::future::Future;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use winit::event_loop::EventLoopProxy;

fn window() -> Result<web_sys::Window, String> {
    web_sys::window().ok_or_else(|| "no browser window".to_owned())
}

fn js_error(what: &str, e: &wasm_bindgen::JsValue) -> String {
    format!(
        "{what}: {}",
        e.as_string().unwrap_or_else(|| format!("{e:?}"))
    )
}

/// Fetches `url` (relative to the page). The request starts now, so several can be in flight before
/// any is awaited.
pub fn fetch(url: &str) -> impl Future<Output = Result<Vec<u8>, String>> + use<> {
    let url = url.to_owned();
    let request = window().map(|w| JsFuture::from(w.fetch_with_str(&url)));
    async move {
        let response: web_sys::Response = request?
            .await
            .map_err(|e| js_error(&url, &e))?
            .dyn_into()
            .map_err(|e| js_error(&url, &e))?;
        if !response.ok() {
            return Err(format!("{url}: HTTP {}", response.status()));
        }
        let buffer = response.array_buffer().map_err(|e| js_error(&url, &e))?;
        let buffer = JsFuture::from(buffer)
            .await
            .map_err(|e| js_error(&url, &e))?;
        Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    }
}

/// The page URL's query string, which stands in for the native app's command line.
pub struct Query(web_sys::UrlSearchParams);

impl Query {
    pub fn from_page() -> Result<Query, String> {
        let search = window()?
            .location()
            .search()
            .map_err(|e| js_error("location", &e))?;
        web_sys::UrlSearchParams::new_with_str(&search)
            .map(Query)
            .map_err(|e| js_error("query", &e))
    }

    /// `name`'s value; `None` if it's absent.
    pub fn get(&self, name: &str) -> Option<String> {
        self.0.get(name)
    }

    /// Whether `name` is present, with or without a value.
    pub fn has(&self, name: &str) -> bool {
        self.0.has(name)
    }
}

/// Hands the app what the page embedding it asks (`?embed`, [`crate::window::Embed`]): a `postMessage` of
/// `"spelunky:pause"`, `"spelunky:play"`, `"spelunky:take-control"` or `{ spelunky: "seek", frame }` from its
/// own origin, the wiki's.
pub fn listen_to_embedder(proxy: EventLoopProxy<UserEvent>) -> Result<(), String> {
    let window = window()?;
    let origin = window
        .location()
        .origin()
        .map_err(|e| js_error("location", &e))?;
    let listener =
        Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
            if e.origin() != origin {
                return;
            }
            let data = e.data();
            let control = match data.as_string().as_deref() {
                Some("spelunky:pause") => Control::Pause,
                Some("spelunky:play") => Control::Play,
                Some("spelunky:take-control") => Control::TakeControl,
                Some(_) => return,
                None => match seek_frame(&data) {
                    Some(frame) => Control::Seek(frame),
                    None => return,
                },
            };
            let _ = proxy.send_event(UserEvent::Control(control));
        });
    window
        .add_event_listener_with_callback("message", listener.as_ref().unchecked_ref())
        .map_err(|e| js_error("message listener", &e))?;
    // It listens for as long as the page runs.
    listener.forget();
    Ok(())
}

/// A `{ spelunky: "seek", frame }` message's frame: a whole number, not negative.
fn seek_frame(data: &wasm_bindgen::JsValue) -> Option<u64> {
    let field = |name: &str| js_sys::Reflect::get(data, &name.into()).ok();
    if !data.is_object() || field("spelunky")?.as_string().as_deref() != Some("seek") {
        return None;
    }
    let frame = field("frame")?.as_f64()?;
    // Past 2^53 a double has no odd numbers; the clamp to the replay's end handles the rest.
    (frame >= 0.0 && frame.fract() == 0.0).then_some(frame as u64)
}

/// Tells the page embedding the app where its replay is: a `postMessage` to its own origin of
/// `{ spelunky: "playback", ready, paused, live, ended, frame, frames, lead, state }` ([`Playback`]'s fields).
pub fn tell_embedder(playback: Playback) {
    // As clip.json's `state_hash` has it: 16 hex digits, as a double can't hold every u64. Null until the
    // clip's first frame has run.
    let state = playback
        .state
        .map_or(wasm_bindgen::JsValue::NULL, |h| format!("{h:016x}").into());
    post_to_embedder(&[
        ("spelunky", "playback".into()),
        ("ready", playback.ready.into()),
        ("paused", playback.paused.into()),
        ("live", playback.live.into()),
        ("ended", playback.ended.into()),
        // As doubles: whole up to 2^53, millennia of frames.
        ("frame", (playback.frame as f64).into()),
        ("frames", (playback.frames as f64).into()),
        ("lead", (playback.lead as f64).into()),
        ("state", state),
    ]);
}

/// `fields` as an object, to the page embedding this one, if there is one, and if it's of this origin.
fn post_to_embedder(fields: &[(&str, wasm_bindgen::JsValue)]) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let (Ok(Some(parent)), Ok(origin)) = (window.parent(), window.location().origin()) else {
        return;
    };
    // A page that isn't in a frame is its own parent.
    if parent == window {
        return;
    }
    let message = js_sys::Object::new();
    for (key, value) in fields {
        let _ = js_sys::Reflect::set(&message, &(*key).into(), value);
    }
    let _ = parent.post_message(&message, &origin);
}

/// Shows an error on the page (in `#status`, which the page styles) and in the console. The event loop
/// can't return its error the way native `run_app` does, so this is where web failures end up.
pub fn show_error(error: &str) {
    web_sys::console::error_1(&format!("app: {error}").into());
    // An embedding page shows the page, and so the error, instead of whatever it was showing over it.
    post_to_embedder(&[("spelunky", "error".into()), ("error", error.into())]);
    if let Some(status) = status() {
        status.set_text_content(Some(error));
        let _ = status.set_attribute("class", "error");
    }
}

/// A warning, in the console, where the native app would print one.
pub fn warn(message: &str) {
    web_sys::console::warn_1(&format!("app: {message}").into());
}

/// Clears the page's loading message once the app is running.
pub fn clear_status() {
    if let Some(status) = status() {
        status.set_text_content(None);
    }
}

fn status() -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id("status")
}

/// Panics show on the page too: wasm aborts on a panic, leaving a frozen canvas otherwise.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| show_error(&info.to_string())));
}
