#![allow(warnings)]
#![feature(generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async_stream]
fn foos(a: String) -> Result<(), u32> {
    Ok(())
}

fn main() {}
