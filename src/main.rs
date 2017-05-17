#![feature(proc_macro, conservative_impl_trait, generators, generator_trait)]

use std::ops::Generator;

fn _foo(a: &i32) -> Box<Generator<Yield = (), Return = i32>> {
    Box::new((move |a: &i32| {
        if false {
            yield
        }
        *a
    })(a))
}

fn main() {}
