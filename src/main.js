// Minimal JS entry point: instantiate the wasm module and hand it the canvas.
// (Hand-written JavaScript is kept to the bare minimum, per the design prompt.)
import init, { start } from "../pkg/wyoming_trail.js";

const canvas = document.getElementById("game");
init().then(() => start(canvas));
