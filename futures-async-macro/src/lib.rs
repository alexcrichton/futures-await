#![feature(proc_macro)]
#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::*;

#[proc_macro_attribute]
pub fn async(attribute: TokenStream, function: TokenStream) -> TokenStream {
    if attribute.to_string() != "" {
        panic!("the #[async] attribute currently takes no arguments");
    }

    let ast = syn::parse_item(&function.to_string())
                    .expect("failed to parse item");
    let Item { ident, vis: _, attrs, node } = ast;
    let all = match node {
        ItemKind::Fn(a, b, c, d, e, f) => (a, b, c, d, e, f),
        _ => panic!("#[async] can only be applied to functions"),
    };
    let (decl, _unsafety, _constness, _abi, _generics, block) = all;
    let FnDecl { inputs, output, variadic } = { *decl };
    let ref inputs = inputs;
    let output = match output {
        FunctionRetTy::Default => Ty::Tup(Vec::new()),
        FunctionRetTy::Ty(t) => t,
    };
    assert!(!variadic, "variadic functions cannot be async");

    // Actual #[async] transformation
    let output = quote! {
        #(#attrs)*
        fn #ident(#(#inputs),*)
            -> Box<::futures::Future<
                    Item = <#output as ::futures::__rt::FutureType>::Item,
                    Error = <#output as ::futures::__rt::FutureType>::Error,
               >>
        {
            Box::new(::futures::__rt::gen((move || {
                #[allow(unreachable_code)]
                {
                    if false {
                        yield
                    }
                }

                #block
            })()))
        }
    };
    output.parse().unwrap()
}
