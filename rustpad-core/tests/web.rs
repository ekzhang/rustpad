//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use rustpad_core::duplicate;

use js_sys::JSON;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    let s = String::from("foobar");
    let value = duplicate(s);
    let value = JSON::stringify(&value).unwrap();
    assert_eq!(value.to_string(), String::from("[\"foobar\",\"foobar\"]"));
}
