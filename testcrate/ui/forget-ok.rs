#![feature(proc_macro, generators)]

extern crate futures_await as futures;

use futures::prelude::{*, r#await};

#[async]
fn foo() -> Result<(), ()> {
}

#[async_stream(item = i32)]
fn foos() -> Result<(), ()> {
}

fn main() {}
