use pmutil::prelude::SpanExt;
use pmutil::synom_ext::FromSpan;
use proc_macro2::Span;
use std::iter;
use syn::{Block, Item, ItemExternCrate, Stmt, VisInherited};
use syn::delimited::Element;

pub fn call_site<T: FromSpan>() -> T {
    FromSpan::from_span(Span::call_site())
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
