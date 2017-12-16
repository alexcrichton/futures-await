#![allow(warnings)]
#![feature(proc_macro, conservative_impl_trait, generators, catch_expr)]

extern crate futures_await as futures;

use futures::prelude::*;

fn bar<'a>(a: &'a str) -> Box<Future<Item = i32, Error = u32> + 'a> {
    panic!()
}

#[async]
fn foo(a: String) -> Result<i32, u32> {
    await!(bar(&a))?;
    drop(a);
    Ok(1)
}

#[async_stream]
fn foos(a: String) -> impl Stream<Item = i32, Error = u32> {
    yield do catch { Ok(await!(bar(&a))?) };
    drop(a);
    yield Ok(5);
}

fn main() {}
