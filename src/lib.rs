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
#![feature(attr_literals)]
#![feature(on_unimplemented)]
#![feature(optin_builtin_traits)]


extern crate futures;
extern crate futures_async_macro;
// the compiler lies that this has no effect

// extern crate futures_await_macro;

pub use futures::*;

pub mod prelude {
    pub use {Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
    pub use IntoFuture;
    pub use futures_async_macro::{async, async_block, async_stream, async_stream_block, await};

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
    pub use std::ops::Generator;
    pub use std::option::Option::{None, Some};
    pub use std::result::Result::{self, Err, Ok};

    use futures::{Async, Future, Poll, Stream};
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
    struct GenFut<G>(G);

    pub fn async_future<'a, G, T, E>(gen: G) -> impl 'a + MyFuture<Result<T, E>>
    where
        G: 'a + Generator<Yield = (), Return = Result<T, E>>,
    {
        GenFut(gen)
    }

    impl<G, T, E> Future for GenFut<G>
    where
        G: Generator<Yield = (), Return = Result<T, E>>,
    {
        type Item = T;
        type Error = E;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            match self.0.resume() {
                GeneratorState::Yielded(()) => Ok(Async::NotReady),
                GeneratorState::Complete(e) => e.into_result().map(Async::Ready),
            }
        }
    }

    pub fn async_stream<'a, G, T, E>(gen: G) -> impl 'a + Stream<Item = T, Error = E>
    where
        G: 'a + Generator<Yield = Result<T, StreamError<E>>, Return = ()>,
    {
        GenStream(gen)
    }

    struct GenStream<G>(G);

    impl<G, T, E> Stream for GenStream<G>
    where
        G: Generator<Yield = Result<T, StreamError<E>>, Return = ()>,
    {
        type Item = T;
        type Error = E;

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            match self.0.resume() {
                GeneratorState::Yielded(Ok(item)) => Ok(Async::Ready(Some(item))),
                GeneratorState::Yielded(Err(StreamError(e))) => match e {
                    StreamErrorInner::Error(e) => Err(e),
                    StreamErrorInner::NotReady => Ok(Async::NotReady),
                },
                GeneratorState::Complete(()) => Ok(Async::Ready(None)),
            }
        }
    }


    /// Auto trait used to implement
    ///  generic From<T> for StreamError<E> where T: Into<E>
    ///
    /// This makes using ? in yield do catch much more ergonomic.
    pub trait UserProvidedError {}
    #[allow(auto_impl)]
    impl UserProvidedError for ..{}
    impl<E> !UserProvidedError for StreamError<E> {}



    pub struct Value<A, B>(::std::marker::PhantomData<(A, B)>);
    // TODO: Don't show this message if From<T> is not implemented for E
    #[rustc_on_unimplemented(message = "futures-await: generic error type cannot yielded directly",
                             label = "
                           due to lack of negative constraint in rust trait system,
                           type `{Self}` cannot be yielded as an error directly.

                           try result.map_err(YourError::from)
                           e.g.
                                yield do catch {{ Ok(await!(future).map_err(GenericError::from)?) }}
                           instead of
                                yield do catch {{ Ok(await!(future)?) }}")]
    pub trait NotEq {}
    #[allow(auto_impl)]
    impl NotEq for .. {}
    impl<A> !NotEq for Value<A, A> {}

    pub struct StreamError<E>(StreamErrorInner<E>);




    impl<E> From<E> for StreamError<E> {
        fn from(e: E) -> Self {
            StreamError(StreamErrorInner::Error(e))
        }
    }


    impl<E, T> From<T> for StreamError<E>
    where
        T: Into<E>,
        Value<T, Self>: NotEq,
        Value<T, E>: NotEq,
    {
        fn from(err: T) -> Self {
            StreamError(StreamErrorInner::Error(err.into()))
        }
    }


    // impl<E, T> From<T> for StreamError<E>
    // where
    //     T: UserProvidedError + Into<E>,
    // {
    //     fn from(err: T) -> Self {
    //         StreamError(StreamErrorInner::Error(err.into()))
    //     }
    // }

    enum StreamErrorInner<E> {
        Error(E),
        NotReady,
    }



    pub trait YieldType {
        fn not_ready() -> Self;
    }
    /// Used for future
    impl YieldType for () {
        #[inline(always)]
        fn not_ready() -> Self {}
    }
    /// Used for stream
    impl<T, E> YieldType for Result<T, StreamError<E>> {
        #[inline(always)]
        fn not_ready() -> Self {
            Err(StreamError(StreamErrorInner::NotReady))
        }
    }
}
