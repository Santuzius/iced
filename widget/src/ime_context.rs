//! What soft keyboards and touch text editing need from text fields beyond iced's input method request (Android support of this fork, not part of upstream iced).
//!
//! - Text around the cursor: keyboards predict, autocorrect and capitalise from it. iced's input method request carries only the cursor rectangle, so `text_input` and `text_editor` record their text here while focused, and the Android glue hands it to the keyboard.
//! - Text menu: a long press in a text field selects a word and asks the platform for its Cut/Copy/Paste menu through [`set_menu_handler`].
#![cfg_attr(
    not(target_os = "android"),
    allow(dead_code, reason = "only Android records the text context")
)]

use crate::core::Point;

use std::sync::{Mutex, OnceLock};

/// At most this many characters on each side; keyboards only look at the last few words, and large logs stay cheap.
const LIMIT: usize = 500;

/// Larger selections are not passed on: their text would be copied for every frame. Keyboards then see no context.
const SELECTION_LIMIT: usize = 10_000;

static CONTEXT: Mutex<Option<Context>> = Mutex::new(None);

/// Text around the cursor or selection of the focused field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    /// Text before the cursor or selection.
    pub before: String,
    /// The selected text; empty without a selection.
    pub selected: String,
    /// Text after the cursor or selection.
    pub after: String,
    /// Whether Backspace and Delete remove a whole grapheme, e.g. 👍🏽, as in `text_input`, or a single character (Unicode scalar value), as in `text_editor`. Arrow keys always move by graphemes.
    pub deletes_graphemes: bool,
}

/// The last context recorded by a focused text field, if any. Password fields record their masked text.
pub fn get() -> Option<Context> {
    CONTEXT.lock().ok()?.clone()
}

pub(crate) fn set(
    before: &str,
    selected: &str,
    after: &str,
    deletes_graphemes: bool,
) {
    if selected.len() > SELECTION_LIMIT {
        clear();
        return;
    }

    let skip = before.chars().count().saturating_sub(LIMIT);
    let context = Context {
        before: before.chars().skip(skip).collect(),
        selected: selected.to_owned(),
        after: after.chars().take(LIMIT).collect(),
        deletes_graphemes,
    };

    if let Ok(mut current) = CONTEXT.lock() {
        *current = Some(context);
    }
}

pub(crate) fn clear() {
    if let Ok(mut current) = CONTEXT.lock() {
        *current = None;
    }
}

/// Shows the platform's text menu (Cut, Copy, Paste, Select all) at a position in the window, in logical pixels. The second argument tells whether text is selected.
pub type MenuHandler = fn(Point, bool);

static MENU: OnceLock<MenuHandler> = OnceLock::new();

/// Registers the platform's text menu; without one, a long press only selects.
pub fn set_menu_handler(handler: MenuHandler) {
    let _ = MENU.set(handler);
}

pub(crate) fn show_menu(position: Point, has_selection: bool) {
    if let Some(handler) = MENU.get() {
        handler(position, has_selection);
    }
}

/// How long a finger has to rest on a text field before it selects a word.
pub(crate) const LONG_PRESS: std::time::Duration =
    std::time::Duration::from_millis(500);

/// How far, in logical pixels, a finger may move and still count as resting.
pub(crate) const TOUCH_SLOP: f32 = 8.0;
