use pmutil::prelude::*;
use pmutil::synom_ext::FromSpan;
use pmutil::respan::Respan;
use quote::ToTokens;
use proc_macro2::{Span, TokenNode};
use std::iter;
use syn::{Block, Item, ItemExternCrate, Stmt, VisInherited};
use syn::delimited::Element;

pub fn call_site<T: FromSpan>() -> T {
    FromSpan::from_span(Span::call_site())
}



//
///
/// Returned quoter respans `futures` and `futures_await`
///   with `call_site`, as it's requied to show `std::result::Result`
///   instead of `::futures::__rt::Result`
pub fn quoter<S>(respan: S) -> Quote
where
    S: 'static + Respan,
{
    struct Spanner<S>(S);
    impl<S: Respan> Respan for Spanner<S> {
        fn span_for(&self, kind: &TokenNode) -> Span {
            let span = self.0.span_for(kind);
            match kind {
                &TokenNode::Term(ref term)
                    if term.as_str() == "futures_await" || term.as_str() == "futures" =>
                {
                    Span::call_site()
                }
                _ => span,
            }
        }
    }
    Quote::new(Spanner(respan))
}
//
///
///
pub fn quoter_from_tokens<T>(t: &T) -> Quote
where
    T: ToTokens,
{
    quoter(t.first_last())
}
//
///
///
pub fn quoter_from_tokens_or<T>(t: &Option<T>, span: Span) -> Quote
where
    T: ToTokens,
{
    match t {
        &Some(ref tokens) => quoter(tokens.first_last()),
        &None => quoter(span),
    }
}

/// Prepend `extern crate futures_await`
pub fn prepend_extern_crate_rt(block: Block) -> Block {
    fn extern_crate_futures_await() -> Stmt {
        Stmt::Item(box Item::ExternCrate(ItemExternCrate {
            attrs: Default::default(),
            vis: VisInherited {}.into(),
            extern_token: call_site(),
            crate_token: call_site(),
            ident: Span::call_site().new_ident("futures_await"),
            rename: None,
            semi_token: call_site(),
        }))
    }

    Block {
        stmts: iter::once(extern_crate_futures_await())
            .chain(block.stmts)
            .collect(),
        ..block
    }
}


/// Extension trait for syn::delimited::Element
pub trait ElementExt<T, D> {
    fn map_item<F>(self, map: F) -> Self
    where
        F: FnOnce(T) -> T;
}

impl<T, D> ElementExt<T, D> for Element<T, D> {
    fn map_item<F>(self, map: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        match self {
            Element::Delimited(t, d) => Element::Delimited(map(t), d),
            Element::End(t) => Element::End(map(t)),
        }
    }
}
