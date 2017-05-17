#![feature(proc_macro, conservative_impl_trait, generators, generator_trait)]

use std::ops::Generator;

fn foo(a: &str) -> impl Generator<Yield = (), Return = String> {
    (move |a: &str| {
        if false {
            yield
        }
        a.to_string()
    })(a)
}

fn main() {
    let mut gen = {
        let a = String::from("foo");
        foo(&a)
    };
    String::from("bar");
    println!("{:?}", gen.resume(()));
}
