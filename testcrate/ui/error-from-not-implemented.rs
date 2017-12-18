#![allow(warnings)]
#![feature(proc_macro, conservative_impl_trait, generators, catch_expr)]

extern crate futures_await as futures;

use futures::prelude::*;

struct E;
#[async]
fn foo(res: Result<(), ()>) -> Result<(), E> {
    Ok(res?)
}

#[async_stream]
fn foos(res: Result<(), ()>) -> impl Stream<Item = (), Error = E> {
    yield do catch {
        res?;
        Ok(())
    };
}

fn main() {}
