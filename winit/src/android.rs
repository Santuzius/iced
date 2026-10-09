//! Android glue for the regular iced runtime (added by android-iced-example, not part of upstream iced).
//!
//! winit runs on Android, but leaves out what an iced app needs beyond drawing and touch:
//! - the event loop must be built with the [`AndroidApp`] that `android_main` receives,
//! - the soft keyboard and clipboard are Java APIs,
//! - the system theme, IME text and the back gesture arrive on the Java UI thread.
//!
//! The app hands over the [`AndroidApp`] and a [`Platform`] with [`init`] before calling `run()`. Java callbacks feed events back from any thread with [`send_event`] and [`set_system_theme`].
use crate::core::{self, input_method, theme};

use std::sync::{Mutex, OnceLock};

pub use winit::platform::android::activity::AndroidApp;

/// The Java side of the soft keyboard and the clipboard, implemented by the app.
pub trait Platform: Send + Sync + 'static {
    /// A text field got focus, or its purpose changed. Show the soft keyboard.
    fn show_keyboard(&self, purpose: input_method::Purpose);
    /// No text field has focus any more. Hide the soft keyboard.
    fn hide_keyboard(&self);
    /// Reads text from the system clipboard.
    fn read_clipboard(&self) -> Option<String>;
    /// Writes text to the system clipboard.
    fn write_clipboard(&self, contents: String);
    /// A finger touched the window. Close popups the app opened over it, such as a text menu.
    fn touch_started(&self) {}
}

static APP: OnceLock<AndroidApp> = OnceLock::new();
static PLATFORM: OnceLock<Box<dyn Platform>> = OnceLock::new();
static QUEUE: Mutex<Vec<Injected>> = Mutex::new(Vec::new());
static WAKER: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

pub(crate) enum Injected {
    Event(core::Event),
    Theme(theme::Mode),
}

/// Stores the [`AndroidApp`] and the [`Platform`] for the next `run()`. Call once from `android_main`.
pub fn init(app: AndroidApp, platform: impl Platform) {
    let _ = APP.set(app);
    let _ = PLATFORM.set(Box::new(platform));
}

/// Delivers an event to every open window, as if winit had produced it. Safe to call from any thread.
pub fn send_event(event: core::Event) {
    push(Injected::Event(event));
}

/// Reports the system's light or dark mode, which iced then exposes through `system::theme_changes`.
pub fn set_system_theme(mode: theme::Mode) {
    push(Injected::Theme(mode));
}

fn push(injected: Injected) {
    QUEUE
        .lock()
        .expect("Lock Android event queue")
        .push(injected);

    if let Some(wake) = WAKER.lock().expect("Lock Android waker").as_ref() {
        wake();
    }
}

pub(crate) fn app() -> AndroidApp {
    APP.get().cloned().expect(
        "Call iced_winit::android::init before running the iced program",
    )
}

pub(crate) fn platform() -> Option<&'static dyn Platform> {
    PLATFORM.get().map(Box::as_ref)
}

pub(crate) fn set_waker(wake: impl Fn() + Send + 'static) {
    *WAKER.lock().expect("Lock Android waker") = Some(Box::new(wake));
}

pub(crate) fn drain() -> Vec<Injected> {
    std::mem::take(&mut *QUEUE.lock().expect("Lock Android event queue"))
}
