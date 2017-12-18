#![feature(proc_macro, trace_macros, conservative_impl_trait, generators)]

extern crate futures_await as futures;


use futures::future::lazy;
use futures::prelude::*;

#[test]
fn test_block() {
    assert_eq!(block(true).wait(), Ok(2));
    assert_eq!(block(false).wait(), Ok(0));
    assert_eq!(boxed_block(true).wait(), Ok(2));
    assert_eq!(boxed_block(false).wait(), Ok(0));
}

#[async]
fn block(cond: bool) -> Result<i32, i32> {
    await!({
        // rustfmt-friendly block.
        if cond {
            lazy(|| futures::lazy(|| Ok(2)))
        } else {
            futures::future::ok(0)
        }
    })
}

#[async(boxed)]
fn boxed_block(cond: bool) -> Result<i32, i32> {
    await!({
        {
            // rustfmt-friendly block.
            if cond {
                lazy(|| futures::lazy(|| Ok(2)))
            } else {
                futures::future::ok(0)
            }
        }
    })
}

#[test]
fn test_early_return() {
    assert_eq!(early_return(true).wait(), Ok(1));
    assert_eq!(early_return(false).wait(), Err(2));
}

#[async]
fn early_return(cond: bool) -> Result<i32, i32> {
    await!(if cond {
        futures::lazy(|| Ok(1)).map(|x| x)
    } else {
        lazy(|| futures::lazy(|| Err(2)))
    })
}
