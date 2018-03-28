#![feature(proc_macro, conservative_impl_trait, generators, underscore_lifetimes, pin)]

extern crate futures_await as futures;

use futures::stable::{block_on_stable, StableExecutor};
use futures::executor::{block_on, ThreadPool};
use futures::prelude::*;

#[async]
fn foo() -> Result<i32, i32> {
    Ok(1)
}

#[async]
fn bar(x: &i32) -> Result<i32, i32> {
    Ok(*x)
}

#[async]
fn baz(x: i32) -> Result<i32, i32> {
    await!(bar(&x))
}

#[async(pinned)]
fn boxed(x: i32) -> Result<i32, i32> {
    Ok(x)
}

#[async(pinned_send)]
fn boxed_send(x: i32) -> Result<i32, i32> {
    Ok(x)
}

#[async(pinned_send)]
fn spawnable() -> Result<(), Never> {
    Ok(())
}

#[async_stream(item = u64)]
fn _stream1() -> Result<(), i32> {
    fn integer() -> u64 { 1 }
    let x = &integer();
    stream_yield!(0);
    stream_yield!(*x);
    Ok(())
}

#[async]
pub fn uses_async_for() -> Result<Vec<u64>, i32> {
    let mut v = vec![];
    #[async]
    for i in _stream1() {
        v.push(i);
    }
    Ok(v)
}

#[test]
fn main() {
    assert_eq!(block_on_stable(foo()), Ok(1));
    assert_eq!(block_on_stable(bar(&1)), Ok(1));
    assert_eq!(block_on_stable(baz(17)), Ok(17));
    assert_eq!(block_on(boxed(17)), Ok(17));
    assert_eq!(block_on(boxed_send(17)), Ok(17));
    assert_eq!(block_on_stable(uses_async_for()), Ok(vec![0, 1]));
}

#[test]
fn run_pinned_future_in_thread_pool() {
    let mut pool = ThreadPool::new();
    pool.spawn_pinned(spawnable()).unwrap();
}
