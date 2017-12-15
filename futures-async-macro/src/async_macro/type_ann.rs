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
use proc_macro2::{self, Delimiter, Span, TokenNode, TokenStream};
use quote::{ToTokens, Tokens};
use std::iter;
use syn::*;
use util::call_site;

pub trait TypeAnn {
    /// Create type annotation statements for yield and return.
    ///
    ///
    ///
    /// FIXME: Use None to report error when #[async] function
    ///         does not return anything.
    fn mk_type_annotations(self, output: Option<Type>) -> Vec<Expr>;
}


impl TypeAnn for Future {
    fn mk_type_annotations(self, output: Option<Type>) -> Vec<Expr> {
        let yield_ann: Expr = Quote::new_call_site()
            .quote_with(smart_quote!(Vars {}, { yield }))
            .parse();


        // Return type is Result<_, _>
        // Useful when function returns non-result type.
        let ret_res = ExprKind::Ret(ExprRet {
            expr: Some(box make_expr_with_ty(
                // Span for this is important when return "value"
                //  (in function body) is not a result.
                Quote::from_tokens_or(&output, Span::call_site())
                    .quote_with(smart_quote!(Vars {}, (futures_await::__rt::Result<_, _>)))
                    .parse(),
            )),
            return_token: call_site(),
        }).into();

        // Annotate exact return type if available.
        let ret_exact = output
            .map(make_expr_with_ty)
            .map(|e| {
                ExprRet {
                    expr: Some(box e),
                    return_token: call_site(),
                }
            })
            .map(ExprKind::from)
            .map(Expr::from);

        iter::once(yield_ann)
            .chain(iter::once(ret_res))
            .chain(ret_exact)
            .collect()
    }
}

impl TypeAnn for Stream {
    fn mk_type_annotations(self, output: Option<Type>) -> Vec<Expr> {
        // FIXME: Should handle trait object with multiple bounds.
        let b = match output {
            Some(Type::ImplTrait(ref b)) => b.bounds
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
        };

        let ok_ty = make_expr_with_ty(
            Quote::new_call_site()
                .quote_with(smart_quote!(Vars { Bounds: b }, {
                    <Bounds as futures_await::stream::Stream>::Item
                }))
                .parse(),
        );

        let err_ty = make_expr_with_ty(
            Quote::new_call_site()
                .quote_with(smart_quote!(Vars { Bounds: b }, {
                    <Bounds as futures_await::stream::Stream>::Error
                }))
                .parse(),
        );

        vec![
            Quote::new_call_site()
                .quote_with(smart_quote!(
                    Vars { ok_ty },
                    { yield futures_await::__rt::Ok(ok_ty) }
                ))
                .parse(),
            Quote::new_call_site()
                .quote_with(smart_quote!(Vars { err_ty }, {
                    yield futures_await::__rt::Err(futures_await::__rt::StreamError::from(err_ty))
                }))
                .parse(),
            // return type is ()
            ExprKind::Ret(ExprRet {
                expr: None,
                return_token: call_site(),
            }).into(),
        ]
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
pub fn make_type_annotations<M: Mode>(mode: M, output: Option<Type>) -> Stmt {
    wrap_in_unreacable_block(
        mode.mk_type_annotations(output)
            .into_iter()
            .map(|e| Stmt::Semi(box e, call_site()))
            .collect(),
    )
}


/// Returned expression will panic or abort when executed.
///
///```ignore
///
/// {
///     let _v: #ty = unreachable!();
///     _v
/// }
///```
///
fn make_expr_with_ty(ty: Type) -> Expr {
    let span = Span::call_site();


    ExprKind::Block(ExprBlock {
        block: Block {
            brace_token: span.as_token(),
            stmts: vec![
                Stmt::Local(box Local {
                    ty: Some(box ty),
                    attrs: Default::default(),
                    colon_token: Some(span.as_token()),
                    let_token: span.as_token(),
                    eq_token: Some(span.as_token()),
                    semi_token: span.as_token(),
                    pat: box PatIdent {
                        ident: span.new_ident("_v"),
                        mode: BindingMode::ByValue(Mutability::Immutable),
                        subpat: None,
                        at_token: None,
                    }.into(),
                    init: Some(box ExprKind::Macro(Macro {
                        path: span.new_ident("unreachable").into(),
                        bang_token: span.as_token(),
                        // maybe more helpful message?
                        tokens: vec![
                            TokenTree(proc_macro2::TokenTree {
                                span: span,
                                kind: TokenNode::Group(
                                    Delimiter::Parenthesis,
                                    TokenStream::empty(),
                                ),
                            }),
                        ],
                    }).into()),
                }),
                Stmt::Expr(box ExprKind::Path(ExprPath {
                    qself: None,
                    path: span.new_ident("_v").into(),
                }).into()),
            ],
        },
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
