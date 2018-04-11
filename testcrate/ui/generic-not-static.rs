#![feature(proc_macro, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn foo<T>(t: T) -> Result<T, u32> {
    Ok(t)
}

#[async_stream(item = T)]
fn foos<T>(t: T) -> Result<(), u32> {
    stream_yield!(t);
    Ok(())
}

#[async_stream(item = i32)]
fn foos2<T>(t: T) -> Result<(), u32> {
    Ok(())
}

fn main() {}
