#![allow(warnings)]
#![feature(generators)]

extern crate futures_await as futures;

use futures::prelude::*;

fn bar<'a>(a: &'a str) -> Box<Future<Item = i32, Error = u32> + 'a> {
    panic!()
}

#[r#async]
fn foo(a: String) -> Result<i32, u32> {
    r#await!(bar(&a))?;
    drop(a);
    Ok(1)
}

#[async_stream(item = i32)]
fn foos(a: String) -> Result<(), u32> {
    r#await!(bar(&a))?;
    drop(a);
    stream_yield!(5);
    Ok(())
}

fn main() {}
