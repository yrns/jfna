use proc_macro::TokenStream;
use quote::*;
use syn::{spanned::Spanned, *};

// TODO handle JanetAbstract
// TODO handle Result in the output (requires inner function)
// TODO handle Into<Janet> in output?
// TODO strip unneeded elements from the inner
// TODO handle mutable refs and pass to inner macro
// TODO handle options and update arity

/// Wraps a function making it callable from Janet. The wrapped function can accept any number of
/// arguments that implement [TryFrom<Janet>].
#[proc_macro_attribute]
pub fn jfna(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as syn::Item);

    let ts = if let syn::Item::Fn(f) = f {
        let mut outer = f.clone();
        // Rewrite the outer to receive &mut [Janet]. TODO: check mut
        outer.sig.inputs = parse_quote! { args: &mut [Janet] };

        let mut inner = f.clone();
        inner.sig.ident = format_ident!("{}_inner", inner.sig.ident);

        let (args, idents): (Vec<_>, Vec<_>) = f
            .sig
            .inputs
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let ident = format_ident!("arg{}", i);
                (match arg {
                    FnArg::Receiver(_) => {
                        quote_spanned! { arg.span() => compile_error!("found receiver") }
                    }
                    FnArg::Typed(pat) => {
                        // TODO better error message
                        // TODO check for nil if the inner input is an Option
                        let ty = &pat.ty;
                        quote! {
                            let #ident: #ty = args[#i].try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("wrong arg type"));
                        }
                    },
                }, ident)})
            .unzip();

        // Output.
        // TODO: insert #[janet_fn]
        // TODO: insert #[inline] on inner
        let outer_attrs = outer.attrs;
        let outer_vis = outer.vis;
        let outer_name = outer.sig.ident;
        let inner_name = inner.sig.ident.clone();

        quote! {
            #(#outer_attrs)* #outer_vis fn #outer_name(args: &mut [Janet]) -> Janet {
                #inner

                #(#args)*

                let res = #inner_name(#(#idents),*);
                res
            }
        }
    } else {
        quote_spanned! {
            f.span() => compile_error!("expected fn item");
        }
    };

    eprintln!("{}", ts.to_string());

    ts.into()
}
