//! Creates type annotations for yield and return.
use syn::*;
use super::{first_last, respan};
use quote::{ToTokens, Tokens};
use std::iter;

pub trait TypeData {
    fn set_output(&mut self, output: &Type);
    /// This method returns vec because two annotation is required.
    ///
    /// For future, first one is `Result<_, _>` which allows rustc to say
    ///  `return type should be Result` regardless of user-defined return type.
    ///
    ///
    fn annotate_return_type(&mut self) -> Vec<Type>;
    fn annotate_yield_type(&mut self) -> Vec<Type>;
}
pub struct Future {
    /// Some for #[async] and None for async_block.
    pub output: Option<Type>,
}

pub struct Stream {
    /// Some for #[async_stream] and None for async_stream_block.
    pub output: Option<Type>,
    /// Some for #[async_stream] and None for async_stream_block.
    pub item: Option<Type>,
}

fn parse_ty(t: Tokens) -> Type {
    parse(t.into()).expect("failed to parse type")
}
/// Alternative way to clone
pub fn reparse(ty: &Type) -> Type {
    let mut tokens = Tokens::new();
    ty.to_tokens(&mut tokens);
    parse_ty(tokens)
}

impl TypeData for Future {
    fn set_output(&mut self, output: &Type) {
        self.output = Some(reparse(output));
    }
    fn annotate_return_type(&mut self) -> Vec<Type> {
        iter::once({
            // this should come first because rustc always prefer the first one.
            parse_ty(quote_cs!(::futures::__rt::std::result::Result<_, _>))
        }).chain(self.output.take())
            .collect()
    }
    fn annotate_yield_type(&mut self) -> Vec<Type> {
        vec![parse_ty(quote_cs!(::futures::Async<::futures::__rt::Mu>))]
    }
}

impl TypeData for Stream {
    fn set_output(&mut self, output: &Type) {
        self.output = Some(reparse(output));
    }
    fn annotate_return_type(&mut self) -> Vec<Type> {
        iter::once(parse_ty(
            quote_cs!(::futures::__rt::std::result::Result<(), _>),
        )).chain(self.output.take())
            .collect()
    }
    fn annotate_yield_type(&mut self) -> Vec<Type> {
        iter::once(parse_ty(quote_cs!(::futures::Async<_>)))
            .chain(
                self.item
                    .take()
                    .map(|item_ty| parse_ty(quote_cs!(::futures::Async<#item_ty>))),
            )
            .collect()
    }
}

pub fn make_type_annotations<D: TypeData>(mut data: D) -> Tokens {
    data.annotate_return_type()
        .into_iter()
        .map(make_expr_with_type)
        .map(|expr| quote!(return #expr;))
        .chain(
            data.annotate_yield_type()
                .into_iter()
                .map(make_expr_with_type)
                .map(|expr| quote!(yield #expr;)),
        )
        .fold(Tokens::new(), |mut t, stmt| {
            stmt.to_tokens(&mut t);
            t
        })
}

/// Returned expression will panic or abort if executed.
///
///```rust,ignore
/// {
///     let _v: Type = ::std::process::abort();
///     _v
/// }
///```
///
fn make_expr_with_type(ty: Type) -> Tokens {
    // use abort instead of unreachable because unreachable!()
    //   make reading cargo-expanded code too hard.
    let sp = first_last(&ty);
    let val = respan(quote!(_v).into(), &sp);
    let abort_expr = respan(quote!(::futures::__rt::std::process::abort()).into(), &sp);

    quote_cs!({
        let #val: #ty = #abort_expr;
        #val
    })
}
