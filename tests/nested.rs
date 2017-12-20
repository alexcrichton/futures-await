#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures_await as futures;


use futures::prelude::*;


#[test]
fn _nested_test_attr() {
    // ensures that #[test] attr does not break this.

    #[async]
    fn _inner() -> Result<(), ()> {
        Ok(())
    }
    #[async(boxed)]
    fn _inner_boxed() -> Result<(), ()> {
        Ok(())
    }
}

fn _nested() {
    #[async]
    fn _inner() -> Result<(), ()> {
        Ok(())
    }
    #[async(boxed)]
    fn _inner_boxed() -> Result<(), ()> {
        Ok(())
    }
}
