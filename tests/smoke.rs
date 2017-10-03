//! A bunch of ways to use async/await syntax.
//!
//! This is mostly a test for this repository itself, not necessarily serving
//! much more purpose than that.

#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;

use std::io;

use futures::prelude::*;

#[async]
fn foo() -> Result<i32, i32> {
    Ok(1)
}

#[async]
extern fn _foo1() -> Result<i32, i32> {
    Ok(1)
}

#[async]
unsafe fn _foo2() -> io::Result<i32> {
    Ok(1)
}

#[async]
unsafe extern fn _foo3() -> io::Result<i32> {
    Ok(1)
}

#[async]
pub fn _foo4() -> io::Result<i32> {
    Ok(1)
}

#[async]
fn _foo5<T: Clone + 'static>(t: T) -> Result<T, i32> {
    Ok(t.clone())
}

#[async]
fn _foo6(ref a: i32) -> Result<i32, i32> {
    Err(*a)
}

#[async]
fn _foo7<T>(t: T) -> Result<T, i32>
    where T: Clone + 'static,
{
    Ok(t.clone())
}

#[async(boxed)]
fn _foo8(a: i32, b: i32) -> Result<i32, i32> {
    return Ok(a + b)
}

#[async]
fn _bar() -> Result<i32, i32> {
    await!(foo())
}

#[async]
fn _bar2() -> Result<i32, i32> {
    let a = await!(foo())?;
    let b = await!(foo())?;
    Ok(a + b)
}

#[async]
fn _bar3() -> Result<i32, i32> {
    let (a, b) = await!(foo().join(foo()))?;
    Ok(a + b)
}

#[async]
fn _bar4() -> Result<i32, i32> {
    let mut cnt = 0;
    #[async]
    for x in futures::stream::iter_ok::<_, i32>(vec![1, 2, 3, 4]) {
        cnt += x;
    }
    Ok(cnt)
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
}

#[async]
fn loop_in_loop() -> Result<bool, i32> {
    let mut cnt = 0;
    let vec = vec![1, 2, 3, 4];
    #[async]
    for x in futures::stream::iter_ok::<_, i32>(vec.clone()) {
        #[async]
        for y in futures::stream::iter_ok::<_, i32>(vec.clone()) {
            cnt += x * y;
        }
    }

    let sum = (1..5).map(|x| (1..5).map(|y| x * y).sum::<i32>()).sum::<i32>();
    Ok(cnt == sum)
}

#[test]
fn test_block_closure() {
    let closure = async_closure!(
        |x| {
            match x {
                1 => Ok(()),
                _ => Err(1),
            }
        }
    );

    assert_eq!(closure(1).wait(), Ok(()))
}

#[test]
fn test_capture() {
    let x = 1;

    let closure = async_closure!(
        || {
            match x {
                1 => Ok(()),
                _ => Err(1)
            }
        }
    );

    assert_eq!(closure().wait(), Ok(()))
}

#[test]
fn test_move_capture() {
    let s = "captured".to_string();

    let closure = async_closure!(
        move || {
            if s == "captured" {
                Ok(())
            }
            else {
                Err(1)
            }
        }
    );

    assert_eq!(closure().wait(), Ok(()));
}

#[test]
fn test_loop_in_closure() {
    let closure = async_closure!(
        || {
            let mut cnt = 0;

            #[async]
            for x in futures::stream::iter_ok::<_, i32>(vec![1, 2, 3, 4]) {
                cnt += x;
            }

            if cnt == 10 {
                Ok(())
            }
            else {
                Err(cnt)
            }
        }
    );

    assert_eq!(closure().wait(), Ok(()));
}

/* I wasn't able to figure out how to pass the closure's Expr body to the
 *  ExpandAsyncFor Folder. I'm sure it's easy as hell, but I am very new to
 *  proc_macros (and actually just macros in general).
 */
/*#[test]
fn test_expr_closure() {
    let closure = async_closure!(
        |wha| match wha {
            1 => Ok(()),
            _ => Err(1),
        }
    );

    assert_eq!(closure(1).wait(), Ok(()))
}*/
