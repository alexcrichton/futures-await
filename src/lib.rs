//! Runtime support for the async/await syntax for futures.
//!
//! This crate serves as a masquerade over the `futures` crate itself,
//! reexporting all of its contents. It's intended that you'll do:
//!
//! ```
//! extern crate futures_await as futures;
//! ```
//!
//! This crate adds a `prelude` module which contains various traits as well as
//! the `async` and `await` macros you'll likely want to use.
//!
//! See the crates's README for more information about usage.

#![feature(conservative_impl_trait)]
#![feature(generator_trait)]
#![feature(use_extern_macros)]
#![feature(on_unimplemented)]

extern crate futures;
extern crate futures_async_macro;
// the compiler lies that this has no effect
extern crate futures_await_macro;

pub use futures::*;

pub mod prelude {
    pub use {Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
    pub use IntoFuture;
    pub use futures_async_macro::{async, async_block};
    pub use futures_await_macro::await;
}

/// A hidden module that's the "runtime support" for the async/await syntax.
///
/// The `async` attribute and the `await` macro both assume that they can find
/// this module and use its contents. All of their dependencies are defined or
/// reexported here in one way shape or form.
///
/// This module has absolutely not stability at all. All contents may change at
/// any time without notice. Do not use this module in your code if you wish
/// your code to be stable.
#[doc(hidden)]
pub mod __rt {
    pub use std::boxed::Box;
    pub use std::option::Option::{None, Some};
    pub use std::result::Result::{self, Err, Ok};
    pub use std::ops::Generator;

    use futures::Poll;
    use futures::{Async, Future};
    use std::ops::GeneratorState;

    pub trait MyFuture<T: IsResult>: Future<Item = T::Ok, Error = T::Err> {}

    impl<F, T> MyFuture<T> for F
    where
        F: Future<Item = T::Ok, Error = T::Err> + ?Sized,
        T: IsResult,
    {
    }

    #[rustc_on_unimplemented = "async functions must return a `Result` or \
                                a typedef of `Result`"]
    pub trait IsResult {
        type Ok;
        type Err;

        fn into_result(self) -> Result<Self::Ok, Self::Err>;
    }
    impl<T, E> IsResult for Result<T, E> {
        type Ok = T;
        type Err = E;

        fn into_result(self) -> Result<Self::Ok, Self::Err> {
            self
        }
    }

    pub fn diverge<T>() -> T {
        loop {}
    }

    /// Small shim to translate from a generator to a future.
    ///
    /// This is the translation layer from the generator/coroutine protocol to
    /// the futures protocol.
    struct GenFuture<T>(T);

    pub fn gen<T>(gen: T) -> impl MyFuture<T::Return>
    where
        T: Generator<Yield = ()>,
        T::Return: IsResult,
    {
        GenFuture(gen)
    }

    impl<T> Future for GenFuture<T>
    where
        T: Generator<Yield = ()>,
        T::Return: IsResult,
    {
        type Item = <T::Return as IsResult>::Ok;
        type Error = <T::Return as IsResult>::Err;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            match self.0.resume() {
                GeneratorState::Yielded(()) => Ok(Async::NotReady),
                GeneratorState::Complete(e) => e.into_result().map(Async::Ready),
            }
        }
    }
}
