use pmutil::prelude::*;
use proc_macro2::Span;
use syn::*;
use syn::fold::Folder;

pub struct ExpandAsyncFor;

impl Folder for ExpandAsyncFor {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let expr = fold::fold_expr(self, expr);
        if expr.attrs.len() != 1 {
            return expr;
        }
        // TODO: more validation here
        if expr.attrs[0].path.segments.get(0).item().ident != "async" {
            return expr;
        }
        let all = match expr.node {
            ExprKind::ForLoop(item) => item,
            _ => panic!("only for expressions can have #[async]"),
        };
        let ExprForLoop {
            pat,
            expr,
            body,
            label,
            colon_token,
            ..
        } = all;

        let loop_body = Quote::new_call_site()
            .quote_with(smart_quote!(Vars { pat, body }, {
                {
                    let pat = {
                        extern crate futures_await;
                        let r = futures_await::Stream::poll(&mut __stream)?;
                        match r {
                            futures_await::Async::Ready(e) => match e {
                                futures_await::__rt::Some(e) => e,
                                futures_await::__rt::None => break,
                            },
                            futures_await::Async::NotReady => {
                                yield futures_await::__rt::YieldType::not_ready();
                                continue;
                            }
                        }
                    };

                    body
                }
            }))
            .parse();

        let loop_expr = ExprLoop {
            body: loop_body,
            label,
            colon_token,
            loop_token: Span::call_site().as_token(),
        };

        // Basically just expand to a `poll` loop
        Quote::new_call_site()
            .quote_with(smart_quote!(Vars { expr, loop_expr }, {
                {
                    let mut __stream = expr;
                    loop_expr
                }
            }))
            .parse()
    }

    // Don't recurse into items
    fn fold_item(&mut self, item: Item) -> Item {
        item
    }
}
