#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn empty_block() -> Result<(), ()> {
    await!({})
}

#[async]
fn not_expr_on_last() -> Result<(), ()> {
    await!({
        struct S {}
    })
}

fn main() {}
