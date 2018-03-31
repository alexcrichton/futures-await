#![deprecated(note="\
    `futures-await` has been merged into the main `futures` repository, \
    please switch to depending directly on `futures` with the `nightly` \
    feature activated\
")]

#![feature(use_extern_macros)]

extern crate futures;

pub use futures::*;
