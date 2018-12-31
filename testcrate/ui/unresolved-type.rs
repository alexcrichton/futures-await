#![feature(generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[r#async]
fn foo() -> Result<A, u32> {
    Err(3)
}

#[async_stream(item = A)]
fn foos() -> Result<(), u32> {
    Err(3)
}

fn main() {}
