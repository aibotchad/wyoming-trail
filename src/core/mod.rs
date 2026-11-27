//! Pure game logic. NO web-sys / js-sys / wasm-bindgen in this tree — it must
//! compile and unit-test on a native target as well as wasm32.

pub mod config;
pub mod content;
pub mod data;
pub mod defs;
pub mod game;
pub mod gather;
pub mod map_graph;
pub mod party;
pub mod resources;
pub mod rng;
pub mod scores;
pub mod state;
pub mod truck;
