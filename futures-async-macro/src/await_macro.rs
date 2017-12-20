//! Heterogenous await! macro.

use pmutil::prelude::*;
use syn::*;
use syn::fold::Folder;


/// Yield-to-poll converter
///
///
pub struct ExpandAwait;

impl Folder for ExpandAwait {
    fn fold_block(&mut self, mut block: Block) -> Block {
        let len = block.stmts.len();
        if len == 0 {
            // Use type system for error reporting.
            return expand_await_block(block_to_expr(block));
        }

        let last = block.stmts.remove(len - 1);
        let last = match last {
            // if it's last statement and we're in return expression,
            // await! it.
            Stmt::Expr(expr) => Stmt::Expr(box self.fold_expr(*expr)),

            _ => {
                // Use type system for error reporting.
                block.stmts.push(last);
                return expand_await_block(block_to_expr(block));
            }
        };
        block.stmts.push(last);

        block
    }

    fn fold_stmt(&mut self, _: Stmt) -> Stmt {
        unreachable!("ExpandAwait::fold_stmt must not be called")
    }

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        use syn::ExprKind::*;


        Expr {
            node: match expr.node {
                // recurse into child if current expression is just a wrapper.
                Group(..) | Paren(..) | Block(..) => return fold::fold_expr(self, expr),

                If(e) => If(ExprIf {
                    if_true: self.fold_block(e.if_true),
                    if_false: e.if_false.map(|b| box self.fold_expr(*b)),
                    ..e
                }),

                IfLet(e) => IfLet(ExprIfLet {
                    if_true: self.fold_block(e.if_true),
                    if_false: e.if_false.map(|e| box self.fold_expr(*e)),
                    ..e
                }),

                Match(e) => Match(ExprMatch {
                    arms: e.arms
                        .into_iter()
                        .map(|arm| {
                            Arm {
                                body: box self.fold_expr(*arm.body),
                                ..arm
                            }
                        })
                        .collect(),
                    ..e
                }),

                // TODO?
                _ => return block_to_expr(expand_await_block(expr)),
            },
            ..expr
        }
    }

    /// Don't recurse into locals as locals cannot be value of await!(expr).
    fn fold_local(&mut self, local: Local) -> Local {
        local
    }

    /// Don't recurse into items.
    fn fold_item(&mut self, item: Item) -> Item {
        item
    }
}

/// Make expanded version of `await!(expr)` with appropriate span.
///
fn expand_await_block(expr: Expr) -> Block {
    // Long names help debugging type inference failure.


    //TODO: use quoter_for_tokensc
    Quote::from_tokens(&expr)
        .quote_with(smart_quote!(Vars { fut_expr: expr }, {
            {
                let mut future_in_await = fut_expr;

                loop {
                    extern crate futures_await;

                    match futures_await::Future::poll(&mut future_in_await) {
                        futures_await::__rt::std::result::Result::Ok(
                            futures_await::Async::Ready(await_ok),
                        ) => {
                            break futures_await::__rt::std::result::Result::Ok(await_ok);
                        }
                        futures_await::__rt::std::result::Result::Ok(
                            futures_await::Async::NotReady,
                        ) => {}
                        futures_await::__rt::std::result::Result::Err(await_err) => {
                            break futures_await::__rt::std::result::Result::Err(await_err);
                        }
                    }
                    yield futures_await::__rt::YieldType::not_ready();
                }
            }
        }))
        .parse()
}

fn block_to_expr(block: Block) -> Expr {
    Expr::from(ExprKind::Block(ExprBlock { block }))
}
