//! Wyoming Trail — browser build entry point.
//!
//! `main.js` calls `init().then(() => start(canvas))`. All game rules live
//! in [`crate::core`]; this file only wires the browser layer in.

mod core;
mod web;

use wasm_bindgen::prelude::*;

/// Runs once when the module instantiates (driven by the generated `init()`).
#[wasm_bindgen(start)]
fn main() {
    console_error_panic_hook::set_once();
}

/// Start the game on the given `<canvas>`.
#[wasm_bindgen]
pub fn start(canvas: web_sys::HtmlCanvasElement) {
    web::run(canvas);
}
