//! Procedural macro for the `#[async]` attribute.
//!
//! This crate is an implementation of the `#[async]` attribute as a procedural
//! macro. This is nightly-only for now as it's using the unstable features of
//! procedural macros. Furthermore it's generating code that's using a new
//! keyword, `yield`, and a new construct, generators, both of which are also
//! unstable.
//!
//! Currently this crate depends on `syn` and `quote` to do all the heavy
//! lifting, this is just a very small shim around creating a closure/future out
//! of a generator.
#![feature(box_syntax, proc_macro)]
#![feature(trace_macros)]
#![recursion_limit = "128"]

#[macro_use]
extern crate pmutil;
extern crate proc_macro2;
extern crate proc_macro;
extern crate quote;
extern crate syn;
extern crate synom;


use self::async_macro::{expand_async_block, expand_async_fn, Future, Stream};
use proc_macro::{Delimiter, Span, TokenNode, TokenStream, TokenTree};
use quote::ToTokens;
use syn::fold::Folder;

#[macro_use]
mod util;
mod async_macro;
mod await_macro;


#[proc_macro_attribute]
pub fn async(attribute: TokenStream, function: TokenStream) -> TokenStream {
    expand_async_fn(Future, attribute, function)
}

#[proc_macro_attribute]
pub fn async_stream(attribute: TokenStream, function: TokenStream) -> TokenStream {
    expand_async_fn(Stream, attribute, function)
}

#[proc_macro]
pub fn async_block(input: TokenStream) -> TokenStream {
    let block = syn::parse(TokenStream::from(TokenTree {
        kind: TokenNode::Group(Delimiter::Brace, input),
        span: Span::call_site(),
    })).expect("failed to parse block of async future");

    let block = Some(block)
        .map(|block| expand_async_block(Future, block, None))
        .map(util::prepend_extern_crate_rt)
        .unwrap();

    proc_macro2::TokenStream::from(block.into_tokens()).into()
}

#[proc_macro]
pub fn async_stream_block(input: TokenStream) -> TokenStream {
    let block = syn::parse(TokenStream::from(TokenTree {
        kind: TokenNode::Group(Delimiter::Brace, input),
        span: Span::call_site(),
    })).expect("failed to parse block of async stream");

    let block = Some(block)
        .map(|block| expand_async_block(Stream, block, None))
        .map(util::prepend_extern_crate_rt)
        .unwrap();


    proc_macro2::TokenStream::from(block.into_tokens()).into()
}

#[proc_macro]
pub fn await(input: TokenStream) -> TokenStream {
    let expr = syn::parse(input).expect("failed to parse expression");

    let tokens = await_macro::ExpandAwait.fold_expr(expr).into_tokens();

    // println!("{}", tokens);

    tokens.into()
}
