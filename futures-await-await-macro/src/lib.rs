#![no_std]

/// Ye Olde Await Macro
///
/// Basically a translation of polling to yielding. This crate's macro is
/// reexported in the `futures_await` crate, you should not use this crate
/// specifically. If I knew how to define this macro in the `futures_await`
/// crate I would. Ideally this crate would not exist.

// TODO: how to define this in the `futures_await` crate but have it still
// importable via `futurses_await::prelude::await`?

#[macro_export]
macro_rules! await {
    ($e:expr) => ({
        let mut future = $e;
        loop {
            let poll = {
                let pin = unsafe {
                    ::futures::__rt::core::mem::Pin::new_unchecked(&mut future)
                };
                ::futures::__rt::in_ctx(|ctx| ::futures::__rt::StableFuture::poll(pin, ctx))
            };
            // Allow for #[feature(never_type)] and Future<Error = !>
            #[allow(unreachable_code, unreachable_patterns)]
            match poll {
                ::futures::__rt::core::result::Result::Ok(::futures::__rt::Async::Ready(e)) => {
                    break ::futures::__rt::core::result::Result::Ok(e)
                }
                ::futures::__rt::core::result::Result::Ok(::futures::__rt::Async::Pending) => {}
                ::futures::__rt::core::result::Result::Err(e) => {
                    break ::futures::__rt::core::result::Result::Err(e)
                }
            }
            yield ::futures::__rt::Async::Pending
        }
    })
}

///
/// Await an item from the stream
/// Basically it does same as `await` macro, but for streams
///

#[macro_export]
macro_rules! await_item {
    ($e:expr) => ({
        loop {
            let poll = ::futures::__rt::in_ctx(|ctx| ::futures::Stream::poll_next(&mut $e, ctx));
            // Allow for #[feature(never_type)] and Stream<Error = !>
            #[allow(unreachable_code, unreachable_patterns)]
            match poll {
                ::futures::__rt::core::result::Result::Ok(::futures::__rt::Async::Ready(e)) => {
                    break ::futures::__rt::core::result::Result::Ok(e)
                }
                ::futures::__rt::core::result::Result::Ok(::futures::__rt::Async::Pending) => {}
                ::futures::__rt::core::result::Result::Err(e) => {
                    break ::futures::__rt::core::result::Result::Err(e)
                }
            }

            yield ::futures::__rt::Async::Pending
        }
    })
}

// TODO: This macro needs to use an extra temporary variable because of
// rust-lang/rust#44197, once that's fixed this should just use $e directly
// inside the yield expression
#[macro_export]
macro_rules! stream_yield {
    ($e:expr) => ({
        let e = $e;
        yield ::futures::__rt::Async::Ready(e)
    })
}
