use self::type_ann::TypeAnn;
use pmutil::prelude::*;
use proc_macro::TokenStream;
use proc_macro2::{self, Span, Term};
use quote::ToTokens;
use syn::*;
use syn::fold::Folder;
use util::{self, quoter, quoter_from_tokens, quoter_from_tokens_or};

mod type_ann;
mod for_loop;
mod ret_ty;

#[derive(Copy, Clone)]
pub struct Expander<M: Mode> {
    /// Span of `boxed` in `#[async(boxed)]`
    boxed: Option<Span>,
    mode: M,
}


pub trait Mode: TypeAnn + Copy {
    /// Default lifetime of returned value.
    const DEFAULT_LIFETIME: &'static str;

    /// e.g. `gen` for `::futures_await::__rt::gen`
    const RT_GEN_FN_NAME: &'static str;

    /// parameter `for_boxed` is required to bypass bug described in
    /// module documentation of `ret_ty`.
    fn mk_trait_to_return(
        self,
        for_boxed: bool,
        brace_token: tokens::Brace,
        ret_ty: Type,
    ) -> TypeImplTrait;
}

#[derive(Copy, Clone)]
pub struct Future;

impl Mode for Future {
    const DEFAULT_LIFETIME: &'static str = "'__returned_future";
    const RT_GEN_FN_NAME: &'static str = "async_future";

    fn mk_trait_to_return(
        self,
        for_boxed: bool,
        brace_token: tokens::Brace,
        ret_ty: Type,
    ) -> TypeImplTrait {
        match ret_ty {
            Type::Path(..) => {
                // Path used for `::futures` in `::futures::Future<..>`.
                // This *should* be spanned call_site to make inner functions to work.
                let futures_glob =
                    Quote::new_call_site().quote_with(smart_quote!(Vars {}, { ::futures }));


                let bound: TypeParamBound = if for_boxed {
                    let poly = quoter_from_tokens(&ret_ty).quote_with(smart_quote!(
                        Vars {
                            futures_glob: &futures_glob,
                            ReturnType: &ret_ty,
                        },
                        { ReturnType as futures_glob::__rt::IsResult }
                    ));

                    Quote::from_tokens(&ret_ty)
                        .quote_with(smart_quote!(
                            Vars {
                                // futures_glob,
                                Result: poly,
                            },
                            (::futures::Future<Item = <Result>::Ok, Error = <Result>::Err>)
                        ))
                        .parse()
                } else {
                    quoter_from_tokens(&ret_ty)
                        .quote_with(smart_quote!(
                            Vars {
                                futures_glob,
                                Result: &ret_ty,
                            },
                            (futures_glob::__rt::MyFuture<Result>)
                        ))
                        .parse()
                };


                TypeImplTrait {
                    // Span of `impl` token is used when bound has problems.
                    // e.g. impl MyFuture<u32>
                    impl_token: brace_token.0.as_token(),
                    bounds: vec![bound].into(),
                }
            }
            _ => unimplemented!(
                "#[async]: Handling other return type than result or typedef of result
                    (in TypePath)"
            ),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Stream;

impl Mode for Stream {
    const DEFAULT_LIFETIME: &'static str = "'__returned_stream";
    const RT_GEN_FN_NAME: &'static str = "async_stream";

    fn mk_trait_to_return(
        self,
        _for_boxed: bool,
        _brace_token: tokens::Brace,
        ret_ty: Type,
    ) -> TypeImplTrait {
        // TODO(kdy): Better something something...
        match ret_ty {
            Type::ImplTrait(t) => return t,
            _ => {}
        }
        unimplemented!("return type except impl Stream<> for #[async_stream]")
    }
}

pub fn expand_async_fn<M: Mode>(mode: M, attr: TokenStream, function: TokenStream) -> TokenStream {
    fn get_span_of_boxed(attr: proc_macro2::TokenStream) -> Option<Span> {
        if attr.is_empty() {
            return None;
        }

        let s = attr.to_string();
        if s == "( boxed )" {
            let tts: Vec<_> = attr.into_iter().collect();
            assert_eq!(
                tts.len(),
                1,
                "#[async]: length of attr as TokenStream should be 1"
            );
            return Some(tts[0].span);
        }

        panic!("#[async] macro only accpets `boxed` currently")
    }

    // FIXME: Add option to specify lifetime.

    let boxed = get_span_of_boxed(attr.into());


    let f = parse(function)
        .map(|i: Item| match i {
            Item::Fn(item) => item,
            _ => panic!("#[async] can only be applied to functions"),
        })
        .expect("failed to parse tokens as a function");

    let f: Item = Expander { boxed, mode }.expand(f).into();
    let f = f.into_tokens();
    // println!("Expanded: {}", f);
    f.into()
}

/// Make async body to impl Future (or Stream)
///
/// mode - Future or Stream
/// block - generator body
/// output_ty - Some(output_ty) for #[async] fn
pub fn expand_async_block<M: Mode>(mode: M, block: Block, output_ty: Option<Type>) -> Block {
    /// Make function body into `__rt::gen(|| { #body })`.
    fn call_rt_gen<M: Mode>(mode: M, block: Block, return_type: Option<Type>) -> Block {
        let brace_token = block.brace_token;
        let gen_function = Term::intern(M::RT_GEN_FN_NAME);

        let gen_function = quoter_from_tokens_or(&return_type, (brace_token.0).0)
            .quote_with(smart_quote!(
                Vars { gen_function },
                { futures_await::__rt::gen_function }
            ))
            .parse();


        // move || {
        //     #ensure
        //     #block
        // }
        let gen_closure = Expr::from(ExprKind::from(ExprClosure {
            capture: CaptureBy::Value(brace_token.0.as_token()),
            or1_token: brace_token.0.as_token(),
            or2_token: brace_token.0.as_token(),

            decl: box FnDecl {
                output: ReturnType::Default,
                // no input `||`
                inputs: Default::default(),
                generics: Default::default(),
                variadic: false,
                // not nessacary for closure
                fn_token: Default::default(),
                paren_token: Default::default(),
                dot_tokens: Default::default(),
            },
            body: box Expr::from(ExprKind::from(ExprBlock {
                block: Block {
                    brace_token,
                    stmts: vec![
                        type_ann::make_type_annotations(mode, brace_token, return_type),
                        Stmt::Expr(box ExprKind::from(ExprBlock { block }).into()),
                    ],
                },
            })),
        }));

        Block {
            brace_token,
            stmts: vec![
                //
                // call ::futures_await::__rt::async_future
                Stmt::Expr(box ExprKind::from(ExprCall {
                    func: box Expr::from(ExprKind::from(ExprPath {
                        qself: None,
                        path: gen_function,
                    })),
                    args: vec![(gen_closure, None)].into(),
                    paren_token: brace_token.0.as_token(),
                }).into()),
            ],
        }
    }


    let block = for_loop::ExpandAsyncFor.fold_block(block);
    let block = call_rt_gen(mode, block, output_ty);
    block
}


impl<M: Mode> Expander<M> {
    pub fn expand(self, function: ItemFn) -> ItemFn {
        let function = capture_inputs(function);

        let ret_ty = match function.decl.output {
            ReturnType::Default => panic!("async function should return something"),
            ReturnType::Type(ref ty, _) => ty.clone(),
        };


        let function = self.handle_ret_ty(function);


        let ItemFn { block, .. } = function;


        // Handle async for loops for body.
        let block = expand_async_block(self.mode, *block, Some(ret_ty));


        // Prepend extern crate futures_await;
        let block = self.handle_boxed_body(block);
        let block = box util::prepend_extern_crate_rt(block);

        ItemFn { block, ..function }
    }


    /// Call `::std::boxed::Box::new` if `boxed` is specified.
    ///
    ///
    fn handle_boxed_body(self, block: Block) -> Block {
        // Span of #[async(boxed)]
        let boxed = match self.boxed {
            Some(boxed) => boxed,
            None => return block,
        };

        // This span is used when function body (= block)
        // returns other than return type.
        let brace_token = block.brace_token;

        let box_fn_path = quoter(boxed)
            .quote_with(smart_quote!(
                Vars {},
                { futures_await::__rt::std::boxed::Box::new }
            ))
            .parse();

        let box_stmt = Stmt::Expr(box Expr::from(ExprKind::from(ExprCall {
            func: box box_fn_path,

            args: vec![Expr::from(ExprKind::from(ExprBlock { block }))].into(),
            paren_token: boxed.as_token(),
        })));

        Block {
            brace_token,
            stmts: vec![box_stmt],
        }
    }
}

fn capture_inputs(f: ItemFn) -> ItemFn {
    let block = f.block;
    let decl = *f.decl;
    assert!(!decl.variadic, "variadic functions cannot be async");


    // We've got to get a bit creative with our handling of arguments. For a
    // number of reasons we translate this:
    //
    //          // ...
    //      }
    //      fn foo(ref a: u32) -> Result<u32, u32> {
    //
    // into roughly:
    //
    //      fn foo(__arg_0: u32) -> impl Future<...> {
    //          gen(move || {
    //              let ref a = __arg0;
    //
    //          })
    //              // ...
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
    for (i, input) in decl.inputs.into_iter().enumerate() {
        let input = input.into_item();

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
                pat:
                    Pat::Ident(PatIdent {
                        mode: BindingMode::ByValue(_),
                        ..
                    }),
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
                    mode: BindingMode::ByValue(Mutability::Immutable),
                    ident: ident,
                    at_token: None,
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


    //  let block_inner = quote! {
    //     #( let #patterns = #temp_bindings; )*
    //     #block
    // };
    // Block with temp bindings
    let block = box Block {
        brace_token: block.brace_token.0.as_token(),
        stmts: patterns
            .into_iter()
            .zip(temp_bindings)
            .map(|(pat, binding)| {
                let init = ExprKind::from(ExprPath {
                    qself: None,
                    path: binding.into(),
                });


                Stmt::Local(box Local {
                    attrs: Default::default(),
                    let_token: Default::default(),
                    eq_token: Default::default(),
                    semi_token: Default::default(),
                    colon_token: None,
                    ty: None,

                    pat: box pat,
                    init: Some(box init.into()),
                })
            })
            .chain(block.stmts)
            .collect(),
    };



    ItemFn {
        block,
        decl: box FnDecl {
            inputs: inputs_no_patterns.into(),
            ..decl
        },
        ..f
    }
}
