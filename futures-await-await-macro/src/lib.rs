/// Ye Olde Await Macro
///
/// Basically a translation of polling to yielding. This crate's macro is
/// reexported in the `futures_await` crate, you should not use this crate
/// specifically. If I knew how to define this macro in the `futures_await`
/// crate I would. Ideally this crate would not exist.

// TODO: how to define this in the `futures_await` crate but have it still
// importable via `futurses_await::prelude::await`?

#[macro_export]
macro_rules! r#await {
    ($e:expr) => {{
        let mut future = $e;
        loop {
            match ::futures::Future::poll(&mut future) {
                ::futures::__rt::std::result::Result::Ok(::futures::Async::Ready(e)) => {
                    break ::futures::__rt::std::result::Result::Ok(e);
                }
                ::futures::__rt::std::result::Result::Ok(::futures::Async::NotReady) => {}
                ::futures::__rt::std::result::Result::Err(e) => {
                    break ::futures::__rt::std::result::Result::Err(e);
                }
            }
            yield ::futures::Async::NotReady
        }
    }};
}

///
/// Await an item from the stream
/// Basically it does same as `await` macro, but for streams
///

#[macro_export]
macro_rules! await_item {
    ($e:expr) => {{
        loop {
            match ::futures::Stream::poll(&mut $e) {
                ::futures::__rt::std::result::Result::Ok(::futures::Async::Ready(e)) => {
                    break ::futures::__rt::std::result::Result::Ok(e);
                }
                ::futures::__rt::std::result::Result::Ok(::futures::Async::NotReady) => {}
                ::futures::__rt::std::result::Result::Err(e) => {
                    break ::futures::__rt::std::result::Result::Err(e);
                }
            }

            yield ::futures::Async::NotReady
        }
    }};
}

#[macro_export]
macro_rules! stream_yield {
    ($e:expr) => {
        yield ::futures::Async::Ready($e)
    };
}
