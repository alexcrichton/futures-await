#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

#[async]
fn foobar() -> Result<Option<i32>, ()> {
    let val = Some(42);
    if val.is_none() {
        return Ok(None);
    }
    let val = val.unwrap();
    Ok(val)
}

#[async_stream]
fn foobars() -> impl Stream<Item = Option<i32>, Error = ()> {
    let val = Some(42);
    if val.is_none() {
        yield Ok(None);
        return;
    }
    let val = val.unwrap();
    yield Ok(val);
}

#[async]
fn tuple() -> Result<(i32, i32), ()> {
    if false {
        return Ok(3);
    }
    Ok((1, 2))
}

fn main() {}
