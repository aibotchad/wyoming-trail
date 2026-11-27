//! Wyoming Trail — browser build entry point.
//!
//! Trunk instantiates the module and `index.html` then calls `start(canvas)`.
//! All game rules live in [`crate::core`]; this file only wires the browser
//! layer in.

mod core;
mod web;

use wasm_bindgen::prelude::*;

/// Runs once when the module instantiates (driven by the generated `init()`).
#[wasm_bindgen(start)]
fn main() {
    // A crash stops the game for good, so say so instead of just freezing.
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        show_crash_screen();
    }));
}

/// Call the page's `gameCrashed`, declared in `index.html`.
fn show_crash_screen() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let show = js_sys::Reflect::get(&window, &JsValue::from_str("gameCrashed"));
    if let Ok(show) = show.and_then(|f| f.dyn_into::<js_sys::Function>()) {
        let _ = show.call0(&JsValue::NULL);
    }
}

/// Start the game on the given `<canvas>`.
#[wasm_bindgen]
pub fn start(canvas: web_sys::HtmlCanvasElement) {
    web::run(canvas);
}
