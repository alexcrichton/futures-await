//!
//!
//!
//! # Note
//!
//! Fully quailified trait reference (`<T as IntoFuture>::Ok`) in
//! impl trait position seems lazy evaluated.
//! So we can't use that while declaring function.
//! Instead, we use `MyFuture<T> where T = Result<_, _>`
use super::{Expander, Mode};
use pmutil::prelude::*;
use proc_macro2::Span;
use std::iter;
use syn::*;
use syn::delimited::Element;
use util::{call_site, ElementExt};


impl<M: Mode> Expander<M> {
    pub(super) fn handle_ret_ty(self, f: ItemFn) -> ItemFn {
        let decl = *f.decl;
        let brace_token = f.block.brace_token;


        ItemFn {
            decl: Some(decl)
                .map(|decl| {
                    FnDecl {
                        output: self.ret_ty_to_impl_trait(decl.output, brace_token),
                        ..decl
                    }
                })
                .map(|decl| self.add_ret_lt_bounds(decl))
                .map(|decl| {
                    // #[async(boxed)] is handled here.
                    let output = self.sanitize_returned_trait(decl.output);

                    box FnDecl { output, ..decl }
                })
                .unwrap(),
            ..f
        }
    }

    /// Step 1. Make return type to impl trait.
    fn ret_ty_to_impl_trait(self, ty: ReturnType, brace_token: tokens::Brace) -> ReturnType {
        match ty {
            ReturnType::Default => panic!("#[async] function should return something"),
            ReturnType::Type(ty, rarrow) => {
                let ty = self.mode
                    .mk_trait_to_return(self.boxed.is_some(), brace_token, ty);




                ReturnType::Type(Type::ImplTrait(ty), rarrow)
            }
        }
    }

    /// Step 2. Add lifetime representing the returned future.
    ///
    /// Step 3. Make borrowed value live longer than returned future.
    fn add_ret_lt_bounds(self, mut decl: FnDecl) -> FnDecl {
        match decl.output {
            ReturnType::Type(Type::ImplTrait(ref mut impl_trait), ..) => {
                // check if user specified a lifetime.
                let user_specifed_lt = impl_trait.bounds.items().any(|b| match *b {
                    TypeParamBound::Region(..) => true,
                    _ => false,
                });

                if !user_specifed_lt {
                    // Note: Currently, this does not show #[async] from error message.
                    let ret_lt = Lifetime {
                        span: SynSpan(Span::call_site()),
                        sym: Term::intern(M::DEFAULT_LIFETIME),
                    };
                    decl.inputs = decl.inputs
                        .into_iter()
                        .map(|el| {
                            // `&str` -> `&'ret str`
                            el.map_item(|arg| match arg {
                                FnArg::Captured(ArgCaptured {
                                    pat,
                                    colon_token,
                                    ty:
                                        Type::Reference(TypeReference {
                                            and_token,
                                            lifetime: None,
                                            ty,
                                        }),
                                }) => FnArg::Captured(ArgCaptured {
                                    pat,
                                    colon_token,
                                    ty: Type::Reference(TypeReference {
                                        and_token,
                                        ty,
                                        lifetime: Some(ret_lt.clone()),
                                    }),
                                }),
                                _ => arg,
                            })
                        })
                        .collect();

                    // prepend our lifetime.
                    decl.generics.params = iter::once(Element::Delimited(
                        GenericParam::Lifetime(LifetimeDef {
                            colon_token: None,
                            lifetime: ret_lt.clone(),
                            bounds: Default::default(),
                            attrs: Default::default(),
                        }),
                        Span::call_site().as_token(),
                    )).chain({
                        decl.generics.params.into_iter().map(|el| {
                            el.map_item(|p| {
                                match p {
                                    GenericParam::Lifetime(mut lt) => {
                                        lt.bounds.push_default(ret_lt.clone());
                                        GenericParam::Lifetime(lt)
                                    }
                                    GenericParam::Type(mut p) => {
                                        // TODO(kdy): Handle bounds in where clause.

                                        // Skip if type parameter has a lifetime bound.
                                        let has_lt_bound = p.bounds.items().any(|b| match *b {
                                            TypeParamBound::Region(..) => true,
                                            _ => false,
                                        });
                                        if has_lt_bound {
                                            return GenericParam::Type(p);
                                        }

                                        p.bounds
                                            .push_default(TypeParamBound::Region(ret_lt.clone()));
                                        GenericParam::Type(p)
                                    }
                                    // do nothing for const parameters
                                    GenericParam::Const(c) => GenericParam::Const(c),
                                }
                            })
                        })
                    })
                        .collect();


                    impl_trait
                        .bounds
                        .push_next(TypeParamBound::Region(ret_lt), call_site());
                }

                //TODO(kdy): Handle user specified lifetime
            }
            _ => unreachable!("Other type than impl trait?"),
        }

        decl
    }

    /// Step 4. Handle #[async(boxed)].
    ///
    /// Parameter `ty` must be Type::ImplTrait
    fn sanitize_returned_trait(self, ty: ReturnType) -> ReturnType {
        let (TypeImplTrait { bounds, impl_token }, rarrow) = match ty {
            ReturnType::Type(Type::ImplTrait(ty), rarrow) => (ty, rarrow),
            _ => unreachable!(
                "sanitize_returned_trait wants return type to be `impl Trait + 'lifetime`"
            ),
        };

        match self.boxed {
            Some(boxed) => {
                // We should use ::futures in type position.
                // ::futures::__rt::Box<#bounds>
                let path = Quote::new(boxed)
                    .quote_with(smart_quote!(
                        Vars {
                            TraitBounds: bounds,
                        },
                        (::futures::__rt::Box<TraitBounds>)
                    ))
                    .parse::<Path>();


                ReturnType::Type(TypePath { path, qself: None }.into(), rarrow)
            }
            None => ReturnType::Type(TypeImplTrait { impl_token, bounds }.into(), rarrow),
        }
    }
}
