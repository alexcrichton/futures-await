#![feature(proc_macro, catch_expr, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use std::io;
use futures::prelude::*;





enum GenericError<I> {
    /// Non-generic variant works even if it's generic
    Msg(String),
    /// Error from input stream.
    Input(I),
}

impl<I> From<I> for GenericError<I> {
    fn from(i: I) -> Self {
        GenericError::Input(i)
    }
}

#[async]
fn foo() -> io::Result<()> {
    unimplemented!()
}

#[async_stream]
fn generic<F, IE>(input: F) -> impl Stream<Item = (), Error = GenericError<IE>>
where
    F: Future<Item = (), Error = IE>,
{
    // yield do catch { Ok(await!(input)?) };
    yield do catch { Ok(await!(input).map_err(GenericError::Input)?) };
    yield do catch {
        Err(String::from("asd")).map_err(GenericError::Msg)?;
        Ok(())
    };
}

fn main() {
    let _ = generic(foo());
}
