//! Creates type annotations for yield and return.
//!
//!
//!
//! # Example error
//!
//!
//!```sh
//!
//! error[E0308]: mismatched types
//!   --> tests/lifetime.rs:13:12
//!    |
//! 13 |     return 123;
//!    |            ^^^ expected enum `std::result::Result`, found integral variable
//!    |
//!    = note: expected type `std::result::Result<u64, std::num::ParseIntError>`
//!               found type `{integer}`
//!
//!```
//!
//!
//!
/// FIXME: Maybe go back to quasi quotting?
/// I (kdy) removed many `quote`s to debug span.
///
use super::{Future, Mode, Stream};
use pmutil::prelude::*;
use proc_macro2::Span;
use quote::{ToTokens, Tokens};
use std::iter;
use syn::*;
use util::{quoter_from_tokens, quoter_from_tokens_or};

pub trait TypeAnn {
    /// Create type annotation statements for yield and return.
    ///
    /// brace_token is required because MyFuture<Result<_, _>> is used in type position.
    ///
    /// `return_type` is None for async_block! and async_stream_block!.
    ///
    /// FIXME: Use None to report error when #[async] function
    ///         does not return anything.
    fn mk_type_annotations(
        self,
        brace_token: tokens::Brace,
        return_type: Option<Type>,
    ) -> Vec<Expr>;
}


impl TypeAnn for Future {
    fn mk_type_annotations(
        self,
        brace_token: tokens::Brace,
        return_type: Option<Type>,
    ) -> Vec<Expr> {
        iter::once({
            // yield;
            make_yield_expr(None)
        }).chain(iter::once({
            // return Result<_, _>;
            // Useful when function returns non-result type.
            // let result_type = quoter_from_tokens_or(&return_type, brace_token.0.as_token())
            //     .quote_with(smart_quote!(Vars {}, (futures_await::__rt::Result<_, _>)))
            //     .parse();
            let result_type = Quote::from_tokens_or(&return_type, brace_token.0.as_token())
                .quote_with(smart_quote!(
                    Vars {},
                    (futures_await::__rt::std::result::Result<_, _>)
                ))
                .parse();

            make_return_expr(Some(make_expr_with_type(result_type)))
        }))
            .chain({
                // Annotate exact return type if available.
                return_type
                    .map(make_expr_with_type)
                    .map(Some)
                    .map(make_return_expr)
            })
            .collect()
    }
}

impl TypeAnn for Stream {
    fn mk_type_annotations(
        self,
        brace_token: tokens::Brace,
        return_type: Option<Type>,
    ) -> Vec<Expr> {
        // FIXME: Should handle trait object with multiple bounds.


        let bounds = return_type.as_ref().map(|t| {
            match *t {
                Type::ImplTrait(ref b) => b.bounds
                    .iter()
                    .filter_map(|bound| {
                        // Extract `Stream<Item = T, Error = E>` from impl Stream<...>
                        match **bound.item() {
                            TypeParamBound::Trait(ref poly, ..) => Some(poly),
                            _ => None,
                        }
                    })
                    .next()
                    .expect("#[async_stream]: expected impl Stream for return type"),
                _ => unimplemented!(
                    "#[async_stream] currently only suports 'impl Stream' for return type"
                ),
            }
        });


        iter::once(
            // yield Result<_, _>;
            {
                let result_type = quoter_from_tokens_or(&return_type, brace_token.0.as_token())
                    .quote_with(smart_quote!(
                        Vars {},
                        (futures_await::__rt::std::result::Result<_, _>)
                    ))
                    .parse();
                make_yield_expr(Some(make_expr_with_type(result_type)))
            },
        ).chain({
            // Exact type for O in Result<O, E>
            bounds.map(|bounds| {
                let ok_type = quoter_from_tokens(&bounds)
                    .quote_with(smart_quote!(Vars { Bounds: bounds }, {
                        <Bounds as futures_await::stream::Stream>::Item
                    }))
                    .parse();
                let expr = make_expr_with_type(ok_type);

                quoter_from_tokens(&bounds)
                    .quote_with(smart_quote!(Vars { expr }, {
                        yield futures_await::__rt::std::result::Result::Ok(expr)
                    }))
                    .parse()
            })
        })
            .chain({
                // Exact type for E in Result<O, E>
                bounds.map(|bounds| {
                    quoter_from_tokens(&bounds)
                        .quote_with({
                            let stream_error_type = quoter_from_tokens(&return_type)
                                .quote_with(smart_quote!(
                                    Vars { Bounds: bounds },
                                    (futures_await::__rt::StreamError<
                                        <Bounds as futures_await::stream::Stream>::Error,
                                    >)
                                ))
                                .parse();

                            let expr = make_expr_with_type(stream_error_type);
                            smart_quote!(Vars { expr }, {
                                yield futures_await::__rt::std::result::Result::Err(expr)
                            })
                        })
                        .parse()
                })
            })
            .chain({
                // return;
                iter::once(make_return_expr(None))
            })
            .collect()
    }
}


///
/// Creates ‎
///
///```rust
/// #[allow(unreachable_code)] ‎
/// { ‎
///    if false { ‎
///        // type annotations like ‎
///        // yield Ok(..);
///    } ‎
/// }
///```
///
pub fn make_type_annotations<M: Mode>(
    mode: M,
    brace_token: tokens::Brace,
    output: Option<Type>,
) -> Stmt {
    wrap_in_unreacable_block(
        mode.mk_type_annotations(brace_token, output)
            .into_iter()
            .map(|e| Stmt::Semi(box e, Span::call_site().as_token()))
            .collect(),
    )
}


/// Returned expression will panic or abort when executed.
///
///```rust,ignore
/// {
///     let _v: Type = ::std::process::abort();
///     _v
/// }
///```
///
fn make_expr_with_type(ty: Type) -> Expr {
    // Uses abort instead of unreachable as unreachable!()
    //   make reading cargo-expanded code much harder
    quoter_from_tokens(&ty)
        .quote_with(smart_quote!(Vars { Type: ty }, {
            {
                let _v: Type = futures_await::__rt::abort();
                _v
            }
        }))
        .parse()
}

fn make_yield_expr(expr: Option<Expr>) -> Expr {
    ExprKind::Yield(ExprYield {
        yield_token: Span::call_site().as_token(),
        expr: expr.map(Box::new),
    }).into()
}

fn make_return_expr(expr: Option<Expr>) -> Expr {
    ExprKind::Ret(ExprRet {
        return_token: Span::call_site().as_token(),
        expr: expr.map(Box::new),
    }).into()
}

///
///
///```ignore
///
/// #[allow(unreachable_code)]
/// {
///     if false {
///         #stmts
///     }
/// }
///
///```
///
fn wrap_in_unreacable_block(stmts: Vec<Stmt>) -> Stmt {
    Quote::new_call_site()
        .quote_with(smart_quote!(
            Vars {
                stmts: stmts.into_iter().fold(Tokens::new(), |mut t, node| {
                    node.to_tokens(&mut t);
                    t
                }),
            },
            {
                #[allow(unreachable_code)]
                {
                    if false {
                        stmts
                    }
                }
            }
        ))
        .parse()
}
