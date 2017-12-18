#![allow(warnings)]
#![feature(proc_macro, catch_expr, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use futures::prelude::*;

enum Error<A> {
    A(A),
    Msg(String),
}

impl<A> From<A> for Error<A> {
    fn from(a: A) -> Self {
        Error::A(a)
    }
}

#[async_stream]
fn bar<A>(a: A) -> impl Stream<Item = (), Error = Error<A>> {
    yield do catch {
        Err(a)?;
        Ok(())
    };
}

fn main() {}
