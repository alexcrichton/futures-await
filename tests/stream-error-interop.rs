//! This files tests how #[async_stream], await!, ? works
//! well with each other.

#![feature(proc_macro, conservative_impl_trait, generators, catch_expr)]


extern crate futures_await as futures;

use futures::prelude::*;



#[derive(Debug, PartialEq, Eq)]
enum MyError {
    Str(String),
    Foo(FooError),
    Bar(BarError),
    TwoFrom(OtherWrapper),
}

impl From<String> for MyError {
    fn from(s: String) -> Self {
        MyError::Str(s)
    }
}
impl From<FooError> for MyError {
    fn from(e: FooError) -> Self {
        MyError::Foo(e)
    }
}
impl From<BarError> for MyError {
    fn from(e: BarError) -> Self {
        MyError::Bar(e)
    }
}

impl From<OtherWrapper> for MyError {
    fn from(e: OtherWrapper) -> Self {
        MyError::TwoFrom(e)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FooError;

#[derive(Debug, PartialEq, Eq)]
struct BarError;


#[derive(Debug, PartialEq, Eq)]
struct OtherWrapper(ErrFromWrppaedLib);

#[derive(Debug, PartialEq, Eq)]
struct ErrFromWrppaedLib;

impl From<ErrFromWrppaedLib> for OtherWrapper {
    fn from(s: ErrFromWrppaedLib) -> Self {
        OtherWrapper(s)
    }
}

fn foo() -> Result<(), FooError> {
    Err(FooError)
}

#[async]
fn bar() -> Result<(), BarError> {
    //
    Err(BarError)
}

#[async_stream]
fn use_generic_try() -> impl Stream<Item = (), Error = MyError> {
    yield do catch {
        Err(String::from("first"))?;

        Ok(()) // unreachable
    };


    yield do catch {
        foo()?;

        Ok(()) // unreachable
    };

    yield do catch {
        await!(bar())?;

        Ok(()) // unreachable
    };
}

#[async_stream]
fn _cast_twice() -> impl Stream<Item = (), Error = MyError> {
    yield do catch {
        let err: Result<_, _> = Err(ErrFromWrppaedLib);

        err.map_err(OtherWrapper::from)?;

        Ok(())
    };
}


#[test]
fn genric_try_works() {
    let mut iter = use_generic_try().wait();

    assert_eq!(iter.next(), Some(Err(MyError::Str(String::from("first")))));
    assert_eq!(iter.next(), Some(Err(MyError::Foo(FooError))));
    assert_eq!(iter.next(), Some(Err(MyError::Bar(BarError))));
}
