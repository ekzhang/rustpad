//! Core logic for Rustpad, shared with the client through WebAssembly

#![warn(missing_docs)]

use wasm_bindgen::prelude::*;

mod utils;

/// Duplicate an input, returning a list of two copies
#[wasm_bindgen]
pub fn duplicate(input: String) -> JsValue {
    utils::set_panic_hook();
    JsValue::from_serde(&vec![input; 2]).unwrap()
}
