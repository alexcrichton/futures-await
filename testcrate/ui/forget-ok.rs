#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn foo() -> Result<(), ()> {}

#[async_stream]
fn foos() -> impl Stream<Item = u32, Error = ()> {
    yield 7;
}

fn main() {}
