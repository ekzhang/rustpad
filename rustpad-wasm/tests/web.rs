//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use rustpad_wasm::OpSeq;

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn compose_operations() {
    let mut a = OpSeq::default();
    a.insert("abc");
    let mut b = OpSeq::default();
    b.retain(3);
    b.insert("def");
    let after_a = a.apply("").unwrap();
    let after_b = b.apply(&after_a).unwrap();
    let c = a.compose(&b).unwrap();
    let after_c = c.apply("").unwrap();
    assert_eq!(after_c, after_b);
}

#[wasm_bindgen_test]
fn transform_operations() {
    let s = "abc";
    let mut a = OpSeq::default();
    a.retain(3);
    a.insert("def");
    let mut b = OpSeq::default();
    b.retain(3);
    b.insert("ghi");
    let pair = a.transform(&b).unwrap();
    let (a_prime, b_prime) = (pair.first(), pair.second());
    let ab_prime = a.compose(&b_prime).unwrap();
    let ba_prime = b.compose(&a_prime).unwrap();
    let after_ab_prime = ab_prime.apply(s).unwrap();
    let after_ba_prime = ba_prime.apply(s).unwrap();
    assert_eq!(ab_prime, ba_prime);
    assert_eq!(after_ab_prime, after_ba_prime);
}

#[wasm_bindgen_test]
fn invert_operations() {
    let s = "abc";
    let mut o = OpSeq::default();
    o.retain(3);
    o.insert("def");
    let p = o.invert(s);
    assert_eq!(p.apply(&o.apply(s).unwrap()).unwrap(), s);
}

#[wasm_bindgen_test]
fn transform_index() {
    let mut o = OpSeq::default();
    o.retain(3);
    o.insert("def");
    o.retain(3);
    o.insert("abc");
    assert_eq!(o.transform_index(2), 2);
    assert_eq!(o.transform_index(3), 6);
    assert_eq!(o.transform_index(5), 8);
    assert_eq!(o.transform_index(7), 13);
}
