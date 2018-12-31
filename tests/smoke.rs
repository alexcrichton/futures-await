//! A bunch of ways to use async/await syntax.
//!
//! This is mostly a test for this repository itself, not necessarily serving
//! much more purpose than that.

#![feature(generators, proc_macro_hygiene)]

extern crate futures_await as futures;
extern crate futures_cpupool;

use std::io;

use futures::prelude::{r#await, *};
use futures_cpupool::CpuPool;

#[r#async]
fn foo() -> Result<i32, i32> {
    Ok(1)
}

#[r#async]
extern "C" fn _foo1() -> Result<i32, i32> {
    Ok(1)
}

#[r#async]
unsafe fn _foo2() -> io::Result<i32> {
    Ok(1)
}

#[r#async]
unsafe extern "C" fn _foo3() -> io::Result<i32> {
    Ok(1)
}

#[r#async]
pub fn _foo4() -> io::Result<i32> {
    Ok(1)
}

#[r#async]
fn _foo5<T: Clone + 'static>(t: T) -> Result<T, i32> {
    Ok(t.clone())
}

#[r#async]
fn _foo6(ref a: i32) -> Result<i32, i32> {
    Err(*a)
}

#[r#async]
fn _foo7<T>(t: T) -> Result<T, i32>
where
    T: Clone + 'static,
{
    Ok(t.clone())
}

#[r#async(boxed)]
fn _foo8(a: i32, b: i32) -> Result<i32, i32> {
    return Ok(a + b);
}

#[r#async(boxed_send)]
fn _foo9() -> Result<(), ()> {
    Ok(())
}

#[r#async]
fn _bar() -> Result<i32, i32> {
    r#await!(foo())
}

#[r#async]
fn _bar2() -> Result<i32, i32> {
    let a = r#await!(foo())?;
    let b = r#await!(foo())?;
    Ok(a + b)
}

#[r#async]
fn _bar3() -> Result<i32, i32> {
    let (a, b) = r#await!(foo().join(foo()))?;
    Ok(a + b)
}

#[r#async]
fn _bar4() -> Result<i32, i32> {
    let mut cnt = 0;
    #[r#async]
    for x in futures::stream::iter_ok::<_, i32>(vec![1, 2, 3, 4]) {
        cnt += x;
    }
    Ok(cnt)
}

#[async_stream(item = u64)]
fn _stream1() -> Result<(), i32> {
    stream_yield!(0);
    stream_yield!(1);
    Ok(())
}

#[async_stream(item = T)]
fn _stream2<T: Clone + 'static>(t: T) -> Result<(), i32> {
    stream_yield!(t.clone());
    stream_yield!(t.clone());
    Ok(())
}

#[async_stream(item = i32)]
fn _stream3() -> Result<(), i32> {
    let mut cnt = 0;
    #[r#async]
    for x in futures::stream::iter_ok::<_, i32>(vec![1, 2, 3, 4]) {
        cnt += x;
        stream_yield!(x);
    }
    Err(cnt)
}

#[async_stream(boxed, item = u64)]
fn _stream4() -> Result<(), i32> {
    stream_yield!(0);
    stream_yield!(1);
    Ok(())
}

mod foo {
    pub struct Foo(pub i32);
}

#[async_stream(boxed, item = foo::Foo)]
pub fn stream5() -> Result<(), i32> {
    stream_yield!(foo::Foo(0));
    stream_yield!(foo::Foo(1));
    Ok(())
}

#[async_stream(boxed, item = i32)]
pub fn _stream6() -> Result<(), i32> {
    #[r#async]
    for foo::Foo(i) in stream5() {
        stream_yield!(i * i);
    }
    Ok(())
}

#[async_stream(item = ())]
pub fn _stream7() -> Result<(), i32> {
    stream_yield!(());
    Ok(())
}

#[async_stream(item = [u32; 4])]
pub fn _stream8() -> Result<(), i32> {
    stream_yield!([1, 2, 3, 4]);
    Ok(())
}

// struct A(i32);
//
// impl A {
//     #[async]
//     fn a_foo(self) -> Result<i32, i32> {
//         Ok(self.0)
//     }
//
//     #[async]
//     fn _a_foo2(self: Box<Self>) -> Result<i32, i32> {
//         Ok(self.0)
//     }
// }

// trait B {
//     #[async]
//     fn b(self) -> Result<i32, i32>;
// }
//
// impl B for A {
//     #[async]
//     fn b(self) -> Result<i32, i32> {
//         Ok(self.0)
//     }
// }

#[async_stream(item = u64)]
fn await_item_stream() -> Result<(), i32> {
    stream_yield!(0);
    stream_yield!(1);
    Ok(())
}

#[r#async]
fn test_await_item() -> Result<(), ()> {
    let mut stream = await_item_stream();

    assert_eq!(await_item!(stream), Ok(Some(0)));
    assert_eq!(await_item!(stream), Ok(Some(1)));
    assert_eq!(await_item!(stream), Ok(None));

    Ok(())
}

#[test]
fn main() {
    assert_eq!(foo().wait(), Ok(1));
    assert_eq!(_bar().wait(), Ok(1));
    assert_eq!(_bar2().wait(), Ok(2));
    assert_eq!(_bar3().wait(), Ok(2));
    assert_eq!(_bar4().wait(), Ok(10));
    assert_eq!(_foo6(8).wait(), Err(8));
    // assert_eq!(A(11).a_foo().wait(), Ok(11));
    assert_eq!(loop_in_loop().wait(), Ok(true));
    assert_eq!(test_await_item().wait(), Ok(()));
}

#[r#async]
fn loop_in_loop() -> Result<bool, i32> {
    let mut cnt = 0;
    let vec = vec![1, 2, 3, 4];
    #[r#async]
    for x in futures::stream::iter_ok::<_, i32>(vec.clone()) {
        #[r#async]
        for y in futures::stream::iter_ok::<_, i32>(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5)
        .map(|x| (1..5).map(|y| x * y).sum::<i32>())
        .sum::<i32>();
    Ok(cnt == sum)
}

#[async_stream(item = i32)]
fn poll_stream_after_error_stream() -> Result<(), ()> {
    stream_yield!(5);
    Err(())
}

#[test]
fn poll_stream_after_error() {
    let mut s = poll_stream_after_error_stream();
    assert_eq!(s.poll(), Ok(Async::Ready(Some(5))));
    assert_eq!(s.poll(), Err(()));
    assert_eq!(s.poll(), Ok(Async::Ready(None)));
}

#[test]
fn run_boxed_future_in_cpu_pool() {
    let pool = CpuPool::new_num_cpus();
    pool.spawn(_foo9()).wait().unwrap();
}
