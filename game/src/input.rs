//! Buttons come from the WASM-AD `KEYS` register; the SDK does edge detection.

pub use wasm_ad::{Buttons as Input, key as btn};
