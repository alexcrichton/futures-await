#![feature(proc_macro, conservative_impl_trait, generators, generator_trait)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn foo(a: i32) -> Result<i32, i32> {
    Err(a)
}

fn main() {
}
