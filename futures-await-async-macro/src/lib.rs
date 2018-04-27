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

#![feature(proc_macro)]
#![recursion_limit = "128"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::{Delimiter, TokenStream, TokenTree};
use proc_macro2::Span;
use quote::{ToTokens, Tokens};
use syn::fold::Fold;
use syn::punctuated::Punctuated;
use syn::*;

macro_rules! quote_cs {
    ($($t:tt)*) => (quote_spanned!(Span::call_site() => $($t)*))
}

fn async_inner<F>(
	boxed: bool,
	function: TokenStream,
	gen_function: Tokens,
	return_ty: F,
) -> TokenStream
where
	F: FnOnce(&Type) -> proc_macro2::TokenStream,
{
	// Parse our item, expecting a function. This function may be an actual
	// top-level function or it could be a method (typically dictated by the
	// arguments). We then extract everything we'd like to use.
	let ItemFn {
		ident,
		vis,
		unsafety,
		constness,
		abi,
		block,
		decl,
		attrs,
		..
	} = match syn::parse(function).expect("failed to parse tokens as a function") {
		Item::Fn(item) => item,
		_ => panic!("#[async] can only be applied to functions"),
	};
	let FnDecl {
		inputs,
		output,
		variadic,
		generics,
		fn_token,
		..
	} = { *decl };
	let where_clause = &generics.where_clause;
	assert!(variadic.is_none(), "variadic functions cannot be async");
	let (output, rarrow_token) = match output {
		ReturnType::Type(rarrow_token, t) => (*t, rarrow_token),
		ReturnType::Default => (
			TypeTuple {
				elems: Default::default(),
				paren_token: Default::default(),
			}.into(),
			Default::default(),
		),
	};

	// We've got to get a bit creative with our handling of arguments. For a
	// number of reasons we translate this:
	//
	//      fn foo(ref a: u32) -> Result<u32, u32> {
	//          // ...
	//      }
	//
	// into roughly:
	//
	//      fn foo(__arg_0: u32) -> impl Future<...> {
	//          gen(move || {
	//              let ref a = __arg0;
	//
	//              // ...
	//          })
	//      }
	//
	// The intention here is to ensure that all local function variables get
	// moved into the generator we're creating, and they're also all then bound
	// appropriately according to their patterns and whatnot.
	//
	// We notably skip everything related to `self` which typically doesn't have
	// many patterns with it and just gets captured naturally.
	let mut inputs_no_patterns = Vec::new();
	let mut patterns = Vec::new();
	let mut temp_bindings = Vec::new();
	for (i, input) in inputs.into_iter().enumerate() {
		// `self: Box<Self>` will get captured naturally
		let mut is_input_no_pattern = false;
		if let FnArg::Captured(ref arg) = input {
			if let Pat::Ident(PatIdent { ref ident, .. }) = arg.pat {
				if ident == "self" {
					is_input_no_pattern = true;
				}
			}
		}
		if is_input_no_pattern {
			inputs_no_patterns.push(input);
			continue;
		}

		match input {
			FnArg::Captured(ArgCaptured {
				pat: syn::Pat::Ident(syn::PatIdent { by_ref: None, .. }),
				..
			}) => {
				inputs_no_patterns.push(input);
			}

			// `ref a: B` (or some similar pattern)
			FnArg::Captured(ArgCaptured {
				pat,
				ty,
				colon_token,
			}) => {
				patterns.push(pat);
				let ident = Ident::from(format!("__arg_{}", i));
				temp_bindings.push(ident.clone());
				let pat = PatIdent {
					by_ref: None,
					mutability: None,
					ident: ident,
					subpat: None,
				};
				inputs_no_patterns.push(
					ArgCaptured {
						pat: pat.into(),
						ty,
						colon_token,
					}.into(),
				);
			}

			// Other `self`-related arguments get captured naturally
			_ => {
				inputs_no_patterns.push(input);
			}
		}
	}

	// This is the point where we handle
	//
	//      #[async]
	//      for x in y {
	//      }
	//
	// Basically just take all those expression and expand them.
	let block = ExpandAsyncFor.fold_block(*block);

	let return_ty = return_ty(&output);

	let block_inner = quote_cs! {
			#( let #patterns = #temp_bindings; )*
			#block
	};
	let mut result = Tokens::new();
	block.brace_token.surround(&mut result, |tokens| {
		block_inner.to_tokens(tokens);
	});
	syn::token::Semi([block.brace_token.0]).to_tokens(&mut result);

	let gen_body_inner = quote_cs! {
			let __e: #output = #result

			// Ensure that this closure is a generator, even if it doesn't
			// have any `yield` statements.
			#[allow(unreachable_code)]
			{
					return __e;
					loop { yield ::futures::Async::NotReady }
			}
	};
	let mut gen_body = Tokens::new();
	block.brace_token.surround(&mut gen_body, |tokens| {
		gen_body_inner.to_tokens(tokens);
	});

	// Give the invocation of the `gen` function the same span as the output
	// as currently errors related to it being a result are targeted here. Not
	// sure if more errors will highlight this function call...
	let output_span = first_last(&output);
	let gen_function = respan(gen_function.into(), &output_span);
	let body_inner = quote_cs! {
			#gen_function (move || -> #output #gen_body)
	};
	let body_inner = if boxed {
		let body = quote_cs! { ::futures::__rt::std::boxed::Box::new(#body_inner) };
		respan(body.into(), &output_span)
	} else {
		body_inner.into()
	};
	let mut body = Tokens::new();
	block.brace_token.surround(&mut body, |tokens| {
		body_inner.to_tokens(tokens);
	});

	let output = quote_cs! {
			#(#attrs)*
			#vis #unsafety #abi #constness
			#fn_token #ident #generics(#(#inputs_no_patterns),*)
					#rarrow_token #return_ty
					#where_clause
			#body
	};

	// println!("{}", output);
	output.into()
}

#[proc_macro_attribute]
pub fn async(attribute: TokenStream, function: TokenStream) -> TokenStream {
	// Handle arguments to the #[async] attribute, if any
	let (boxed, send) = match &attribute.to_string() as &str {
		"boxed" => (true, false),
		"boxed_send" => (true, true),
		"" => (false, false),
		_ => panic!("the #[async] attribute currently only takes `boxed` as an arg"),
	};

	async_inner(
		boxed,
		function,
		quote_cs! { ::futures::__rt::gen },
		|output| {
			// TODO: can we lift the restriction that `futures` must be at the root of
			//       the crate?
			let output_span = first_last(&output);
			let return_ty = if boxed && !send {
				quote_cs! {
						::futures::__rt::std::boxed::Box<::futures::Future<
								Item = <! as ::futures::__rt::IsResult>::Ok,
								Error = <! as ::futures::__rt::IsResult>::Err,
						>>
				}
			} else if boxed && send {
				quote_cs! {
						::futures::__rt::std::boxed::Box<::futures::Future<
								Item = <! as ::futures::__rt::IsResult>::Ok,
								Error = <! as ::futures::__rt::IsResult>::Err,
						> + Send>
				}
			} else {
				// Dunno why this is buggy, hits weird typecheck errors in tests
				//
				// quote_cs! {
				//     impl ::futures::Future<
				//         Item = <#output as ::futures::__rt::MyTry>::MyOk,
				//         Error = <#output as ::futures::__rt::MyTry>::MyError,
				//     >
				// }
				quote_cs! { impl ::futures::__rt::MyFuture<!> + 'static }
			};
			let return_ty = respan(return_ty.into(), &output_span);
			replace_bang(return_ty, &output)
		},
	)
}

#[proc_macro_attribute]
pub fn async_stream(attribute: TokenStream, function: TokenStream) -> TokenStream {
	// Handle arguments to the #[async_stream] attribute, if any
	let args =
		syn::parse::<AsyncStreamArgs>(attribute).expect("failed to parse attribute arguments");

	let mut boxed = false;
	let mut item_ty = None;

	for arg in args.0 {
		match arg {
			AsyncStreamArg(term, None) => {
				if term == "boxed" {
					if boxed {
						panic!("duplicate 'boxed' argument to #[async_stream]");
					}
					boxed = true;
				} else {
					panic!("unexpected #[async_stream] argument '{}'", term);
				}
			}
			AsyncStreamArg(term, Some(ty)) => {
				if term == "item" {
					if item_ty.is_some() {
						panic!("duplicate 'item' argument to #[async_stream]");
					}
					item_ty = Some(ty);
				} else {
					panic!(
						"unexpected #[async_stream] argument '{}'",
						quote_cs!(#term = #ty)
					);
				}
			}
		}
	}

	let boxed = boxed;
	let item_ty = item_ty.expect("#[async_stream] requires item type to be specified");

	async_inner(
		boxed,
		function,
		quote_cs! { ::futures::__rt::gen_stream },
		|output| {
			let output_span = first_last(&output);
			let return_ty = if boxed {
				quote_cs! {
						::futures::__rt::std::boxed::Box<::futures::Stream<
								Item = !,
								Error = <! as ::futures::__rt::IsResult>::Err,
						>>
				}
			} else {
				quote_cs! { impl ::futures::__rt::MyStream<!, !> + 'static }
			};
			let return_ty = respan(return_ty.into(), &output_span);
			replace_bangs(return_ty, &[&item_ty, &output])
		},
	)
}

#[proc_macro]
pub fn async_block(input: TokenStream) -> TokenStream {
	let mut t_tree = TokenTree::Group(proc_macro::Group::new(Delimiter::Brace, input));
	t_tree.set_span(proc_macro::Span::def_site());
	let input = TokenStream::from(t_tree);

	let expr = syn::parse(input).expect("failed to parse tokens as an expression");
	let expr = ExpandAsyncFor.fold_expr(expr);

	let mut tokens = quote_cs! {
			::futures::__rt::gen
	};

	// Use some manual token construction here instead of `quote_cs!` to ensure
	// that we get the `call_site` span instead of the default span.
	let span = Span::call_site();
	syn::token::Paren(span).surround(&mut tokens, |tokens| {
		syn::token::Move(span).to_tokens(tokens);
		syn::token::OrOr([span, span]).to_tokens(tokens);
		syn::token::Brace(span).surround(tokens, |tokens| {
			(quote_cs! {
					if false { yield ::futures::Async::NotReady }
			}).to_tokens(tokens);
			expr.to_tokens(tokens);
		});
	});

	tokens.into()
}

#[proc_macro]
pub fn async_stream_block(input: TokenStream) -> TokenStream {
	let mut t_tree = TokenTree::Group(proc_macro::Group::new(Delimiter::Brace, input));
	t_tree.set_span(proc_macro::Span::def_site());
	let input = TokenStream::from(t_tree);
	let expr = syn::parse(input).expect("failed to parse tokens as an expression");
	let expr = ExpandAsyncFor.fold_expr(expr);

	let mut tokens = quote_cs! {
			::futures::__rt::gen_stream
	};

	// Use some manual token construction here instead of `quote_cs!` to ensure
	// that we get the `call_site` span instead of the default span.
	let span = Span::call_site();
	syn::token::Paren(span).surround(&mut tokens, |tokens| {
		syn::token::Move(span).to_tokens(tokens);
		syn::token::OrOr([span, span]).to_tokens(tokens);
		syn::token::Brace(span).surround(tokens, |tokens| {
			(quote_cs! {
					if false { yield ::futures::Async::NotReady }
			}).to_tokens(tokens);
			expr.to_tokens(tokens);
		});
	});

	tokens.into()
}

struct ExpandAsyncFor;

impl Fold for ExpandAsyncFor {
	fn fold_expr(&mut self, expr: Expr) -> Expr {
		let expr = fold::fold_expr(self, expr);
		let mut async = false;
		{
			let attrs = match expr {
				Expr::ForLoop(syn::ExprForLoop { ref attrs, .. }) => attrs,
				_ => return expr,
			};
			if attrs.len() == 1 {
				// TODO: more validation here
				if attrs[0].path.segments.first().unwrap().value().ident == "async" {
					async = true;
				}
			}
		}
		if !async {
			return expr;
		}
		let all = match expr {
			Expr::ForLoop(item) => item,
			_ => panic!("only for expressions can have #[async]"),
		};
		let ExprForLoop {
			pat,
			expr,
			body,
			label,
			..
		} = all;

		// Basically just expand to a `poll` loop
		let tokens = quote_cs! {{
				let mut __stream = #expr;
				#label
				loop {
						let #pat = {
								extern crate futures_await;
								let r = futures_await::Stream::poll(&mut __stream)?;
								match r {
										futures_await::Async::Ready(e) => {
												match e {
														futures_await::__rt::std::option::Option::Some(e) => e,
														futures_await::__rt::std::option::Option::None => break,
												}
										}
										futures_await::Async::NotReady => {
												yield futures_await::Async::NotReady;
												continue
										}
								}
						};

						#body
				}
		}};
		syn::parse(tokens.into()).unwrap()
	}

	// Don't recurse into items
	fn fold_item(&mut self, item: Item) -> Item {
		item
	}
}

fn first_last(tokens: &ToTokens) -> (Span, Span) {
	let mut spans = Tokens::new();
	tokens.to_tokens(&mut spans);
	let good_tokens = proc_macro2::TokenStream::from(spans)
		.into_iter()
		.collect::<Vec<_>>();
	let first_span = good_tokens
		.first()
		.map(|t| t.span())
		.unwrap_or(Span::def_site());
	let last_span = good_tokens.last().map(|t| t.span()).unwrap_or(first_span);
	(first_span, last_span)
}

fn respan(
	input: proc_macro2::TokenStream,
	&(first_span, last_span): &(Span, Span),
) -> proc_macro2::TokenStream {
	let mut new_tokens = input.into_iter().collect::<Vec<_>>();
	if let Some(token) = new_tokens.first_mut() {
		token.set_span(first_span);
	}
	for token in new_tokens.iter_mut().skip(1) {
		token.set_span(last_span);
	}
	new_tokens.into_iter().collect()
}

fn replace_bang(input: proc_macro2::TokenStream, tokens: &ToTokens) -> proc_macro2::TokenStream {
	let mut new_tokens = Tokens::new();
	for token in input.into_iter() {
		match token {
			proc_macro2::TokenTree::Op(op) if op.op() == '!' => tokens.to_tokens(&mut new_tokens),
			_ => token.to_tokens(&mut new_tokens),
		}
	}
	new_tokens.into()
}

fn replace_bangs(
	input: proc_macro2::TokenStream,
	replacements: &[&ToTokens],
) -> proc_macro2::TokenStream {
	let mut replacements = replacements.iter().cycle();
	let mut new_tokens = Tokens::new();
	for token in input.into_iter() {
		match token {
			proc_macro2::TokenTree::Op(op) if op.op() == '!' => {
				replacements.next().unwrap().to_tokens(&mut new_tokens);
			}
			_ => token.to_tokens(&mut new_tokens),
		}
	}
	new_tokens.into()
}

struct AsyncStreamArg(syn::Ident, Option<syn::Type>);

impl synom::Synom for AsyncStreamArg {
	named!(parse -> Self, do_parse!(
        i: syn!(syn::Ident) >>
        p: option!(do_parse!(
            syn!(syn::token::Eq) >>
            p: syn!(syn::Type) >>
            (p))) >>
        (AsyncStreamArg(i, p))));
}

struct AsyncStreamArgs(Vec<AsyncStreamArg>);

impl synom::Synom for AsyncStreamArgs {
	named!(parse -> Self, map!(
        option!(parens!(call!(Punctuated::<AsyncStreamArg, syn::token::Comma>::parse_separated_nonempty))),
        |p| AsyncStreamArgs(p.map(|d| d.1.into_iter().collect()).unwrap_or_default())
    ));
}
