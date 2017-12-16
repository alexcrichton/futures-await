#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn foo() -> Result<A, u32> {
    Err(3)
}

#[async(boxed)]
fn foo_boxed() -> Result<A, u32> {
    Err(3)
}

#[async_stream]
fn foo_stream() -> impl Stream<Item = A, Error = u32> {
    yield Err(3.into())
}

#[async_stream(boxed)]
fn foo_stream_boxed() -> impl Stream<Item = A, Error = u32> {
    yield Err(3.into())
}

fn main() {}
