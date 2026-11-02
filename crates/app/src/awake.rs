//! Keeping the display awake while the game is in front: 1.1's game settings have "Disable screensavers and
//! power saving actions" on, and GM8's runner does it by refusing the screen saver and the monitor's power-off
//! (`WM_SYSCOMMAND` `SC_SCREENSAVE` and `SC_MONITORPOWER`, Spelunky.exe 0x54604c), which Windows sends to the
//! window in front only. So the display stays on while the game's window has focus, not while it's behind
//! another.
//!
//! - macOS: an IOKit assertion that prevents the display's idle sleep, which also holds off the screen saver.
//! - Windows: `SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED)`.
//! - The web: a screen wake lock (`navigator.wakeLock`), which the browser drops when the tab is hidden.
//! - Linux: nothing yet (the freedesktop `ScreenSaver.Inhibit` call needs a D-Bus client: 851-3873).
//!
//! Frontend only, and only for a session of play: never a replay, a page's clip or a headless run.

/// The display kept awake, while [`hold`](Self::hold) says so.
#[derive(Default)]
pub struct Awake {
    held: Option<platform::Held>,
    /// Times it asked since it was last let go: a platform that refuses isn't asked again and again.
    asked: u32,
}

/// How many times [`Awake::hold`] asks before giving up until it's let go and held again: on the web, a
/// browser may refuse before the page has been used, and grant it after.
const MAX_ASKS: u32 = 3;

impl Awake {
    pub fn new() -> Awake {
        Awake::default()
    }

    /// Keeps the display awake from now on, or no longer. Holding it again does nothing more, except on the
    /// web, where it asks again if the browser dropped or refused it, a few times.
    pub fn hold(&mut self, on: bool) {
        if !on {
            self.held = None;
            self.asked = 0;
            return;
        }
        if self.held.as_ref().is_none_or(platform::Held::dropped) && self.asked < MAX_ASKS {
            self.asked += 1;
            self.held = platform::Held::new();
        }
    }

    /// Whether it holds the display awake (or, on the web, has asked to).
    pub fn holding(&self) -> bool {
        self.held.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_the_display_awake_until_told_not_to() {
        let mut awake = Awake::new();
        assert!(!awake.holding());
        awake.hold(true);
        // Where the platform has a way (Linux doesn't yet).
        assert_eq!(awake.holding(), cfg!(any(target_os = "macos", windows)));
        awake.hold(true);
        assert_eq!(awake.holding(), cfg!(any(target_os = "macos", windows)));
        awake.hold(false);
        assert!(!awake.holding());
    }
}

// IOKit's power assertions have only raw bindings, and nothing that wraps two calls is worth a dependency.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod platform {
    use objc2_foundation::NSString;
    use std::ffi::c_void;

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOPMAssertionCreateWithName(
            assertion_type: *const c_void,
            level: u32,
            name: *const c_void,
            id: *mut u32,
        ) -> i32;
        fn IOPMAssertionRelease(id: u32) -> i32;
    }

    /// `kIOPMAssertionLevelOn`.
    const LEVEL_ON: u32 = 255;

    /// An assertion, released when dropped.
    pub struct Held(u32);

    impl Held {
        pub fn new() -> Option<Held> {
            // `kIOPMAssertionTypePreventUserIdleDisplaySleep`. An NSString is a CFString (toll-free bridged).
            let kind = NSString::from_str("PreventUserIdleDisplaySleep");
            let name = NSString::from_str("Spelunky Classic is running");
            let mut id = 0;
            // SAFETY: both strings live across the call, which copies them, and `id` is a valid out pointer.
            let status = unsafe {
                IOPMAssertionCreateWithName(
                    (&*kind as *const NSString).cast(),
                    LEVEL_ON,
                    (&*name as *const NSString).cast(),
                    &mut id,
                )
            };
            (status == 0).then_some(Held(id))
        }

        pub fn dropped(&self) -> bool {
            false
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            // SAFETY: an assertion this created, released once.
            unsafe {
                IOPMAssertionRelease(self.0);
            }
        }
    }
}

// A raw kernel32 call: windows-sys has it, but behind a feature of its own (`Win32_System_Power`), for one
// function.
#[cfg(windows)]
#[allow(unsafe_code)]
mod platform {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }

    const ES_CONTINUOUS: u32 = 0x8000_0000;
    const ES_DISPLAY_REQUIRED: u32 = 0x0000_0002;

    /// The thread's execution state, set back when dropped. The window's thread, the only one that sets it.
    pub struct Held;

    impl Held {
        pub fn new() -> Option<Held> {
            // SAFETY: no pointers; it returns 0 on failure.
            let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED) };
            (previous != 0).then_some(Held)
        }

        pub fn dropped(&self) -> bool {
            false
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            // SAFETY: as above.
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen::closure::Closure;

    /// A wake lock asked for: its sentinel once the browser grants it. Released when dropped.
    pub struct Held {
        sentinel: Rc<RefCell<Option<JsValue>>>,
        /// The request failed (no wake lock API, a page that isn't visible), or was released.
        failed: Rc<RefCell<bool>>,
    }

    fn get(target: &JsValue, name: &str) -> Option<JsValue> {
        js_sys::Reflect::get(target, &name.into())
            .ok()
            .filter(|v| !v.is_undefined() && !v.is_null())
    }

    impl Held {
        pub fn new() -> Option<Held> {
            // Through Reflect: web-sys has the Screen Wake Lock API only behind its unstable flag.
            let navigator = get(&web_sys::window()?.into(), "navigator")?;
            let lock = get(&navigator, "wakeLock")?;
            let request: js_sys::Function = get(&lock, "request")?.dyn_into().ok()?;
            let promise: js_sys::Promise = request
                .call1(&lock, &"screen".into())
                .ok()?
                .dyn_into()
                .ok()?;
            let held = Held {
                sentinel: Rc::default(),
                failed: Rc::default(),
            };
            let (sentinel, failed) = (held.sentinel.clone(), held.failed.clone());
            let gone = held.failed.clone();
            let granted = Closure::once(move |s: JsValue| {
                // Granted after it was let go: released as it comes.
                if *gone.borrow() {
                    release(&s);
                } else {
                    *sentinel.borrow_mut() = Some(s);
                }
            });
            let refused = Closure::once(move |_: JsValue| {
                *failed.borrow_mut() = true;
            });
            let _ = promise.then2(&granted, &refused);
            granted.forget();
            refused.forget();
            Some(held)
        }

        /// The browser refused it, or released it (the tab was hidden): ask again.
        pub fn dropped(&self) -> bool {
            *self.failed.borrow()
                || self
                    .sentinel
                    .borrow()
                    .as_ref()
                    .and_then(|s| get(s, "released"))
                    .is_some_and(|r| r.as_bool() == Some(true))
        }
    }

    fn release(sentinel: &JsValue) {
        if let Some(release) = get(sentinel, "release")
            && let Ok(release) = release.dyn_into::<js_sys::Function>()
        {
            let _ = release.call0(sentinel);
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            if let Some(sentinel) = self.sentinel.borrow().as_ref() {
                release(sentinel);
            }
            // One not granted yet goes as it comes.
            *self.failed.borrow_mut() = true;
        }
    }
}

#[cfg(not(any(target_os = "macos", windows, target_arch = "wasm32")))]
mod platform {
    /// Nothing to hold yet.
    pub struct Held;

    impl Held {
        pub fn new() -> Option<Held> {
            None
        }

        pub fn dropped(&self) -> bool {
            false
        }
    }
}
