//! What the browser build needs from the page: fetched files, the URL's query and somewhere to show
//! errors. The page is `crates/app/web/index.html`; `scripts/web` builds and serves it.

use std::future::Future;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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

/// Shows an error on the page (in `#status`, which the page styles) and in the console. The event loop
/// can't return its error the way native `run_app` does, so this is where web failures end up.
pub fn show_error(error: &str) {
    web_sys::console::error_1(&format!("app: {error}").into());
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
