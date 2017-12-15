#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;


use futures::prelude::*;
use std::num::ParseIntError;


#[async]
fn parse_int(s: &str) -> Result<u64, ParseIntError> {
    await!(futures::lazy(move || s.parse()))
}

#[test]
fn simple() {
    assert_eq!(parse_int("5").wait(), Ok(5));
    assert_eq!(parse_int("15").wait(), Ok(15));
    assert!(parse_int("uv").wait().is_err());
}


#[async(boxed)]
fn boxed_parse_int(s: &str) -> Result<u64, ParseIntError> {
    await!(futures::lazy(move || s.parse()))
}

#[test]
fn boxed() {
    assert_eq!(boxed_parse_int("5").wait(), Ok(5));
    assert_eq!(boxed_parse_int("15").wait(), Ok(15));
    assert!(boxed_parse_int("uv").wait().is_err());
}

#[async]
fn generic_parse_int<T: AsRef<str>>(t: T) -> Result<u64, ParseIntError> {
    await!(futures::lazy(move || {
        let s = t.as_ref();
        s.parse()
    }))
}

#[test]
fn generic() {
    assert_eq!(generic_parse_int("5").wait(), Ok(5));
    assert_eq!(generic_parse_int("15").wait(), Ok(15));
    assert!(generic_parse_int("uv").wait().is_err());
}
