#![feature(proc_macro, conservative_impl_trait, generators, catch_expr)]

extern crate futures_await as futures;

use futures::prelude::*;



#[async_stream]
fn generic_ret_type<T, I>(iter: I) -> impl Stream<Item = T, Error = ()>
where
    I: IntoIterator<Item = T>,
{
    for item in iter {
        yield Ok(item);
    }
}

#[test]
fn generic_in_ret_type() {
    assert!(
        generic_ret_type("abc".chars())
            .wait()
            .eq(['a', 'b', 'c'].into_iter().cloned().map(Ok))
    );
}



#[async_stream]
fn chars(src: &str) -> impl Stream<Item = char, Error = ()> {
    for ch in src.chars() {
        yield Ok(ch);
    }
}

#[test]
fn lifetime() {
    assert_eq!(chars("abc").collect().wait(), Ok(vec!['a', 'b', 'c']));
    assert_eq!(chars("123").collect().wait(), Ok(vec!['1', '2', '3']));
}

#[async_stream]
fn use_question_mark(err: bool) -> impl Stream<Item = u32, Error = i32> {
    if err {
        yield do catch { Err(1)? };
    }
    yield Ok(2);
}

#[test]
fn basic_question_mark() {
    assert_eq!(use_question_mark(true).wait().next(), Some(Err(1)));
    assert_eq!(use_question_mark(false).wait().next(), Some(Ok(2)));
}
