use proc_macro::TokenStream;
use quote::*;
use syn::{spanned::Spanned, *};

// TODO handle JanetAbstract
// TODO strip unneeded elements from the inner
// TODO handle mutable refs and pass to inner macro
// TODO handle options and update arity

fn is_type(ty: &Type, id: &str) -> bool {
    match ty {
        Type::Path(TypePath { path, .. }) => path
            .segments
            .last()
            .map(|s| s.ident == id)
            .unwrap_or_default(),
        _ => false,
    }
}

fn is_result(ty: &Type) -> bool {
    is_type(ty, "Result")
}

/// Wraps a function making it callable from Janet. The wrapped function can accept any number of
/// arguments that implement [TryFrom<Janet>].
#[proc_macro_attribute]
pub fn jfna(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as syn::Item);

    let ts = if let syn::Item::Fn(f) = f {
        let output_result = matches!(f.sig.output, ReturnType::Type(_, ref ty) if is_result(ty));

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
                    FnArg::Typed(PatType { pat, ty, .. }) => {
                        // TODO check for nil if the inner input is an Option
                        quote! {
                            let #ident: #ty = args[#i].try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("wrong arg type ({}): {}: {} got: {:?}", #i, stringify!(#pat), stringify!(#ty), args[#i]));
                        }
                    },
                }, ident)})
            .unzip();

        // Output.
        // TODO: insert #[janet_fn] attributes
        let outer_attrs = outer.attrs;
        let outer_vis = outer.vis;
        let outer_name = outer.sig.ident;
        let inner_name = inner.sig.ident.clone();

        let call_inner = quote! { #inner_name(#(#idents),*) };

        // If the output type is a Result, unwrap it first.
        let call_inner = if output_result {
            quote! {
                let res = #call_inner.unwrap_or_else(|e| ::janetrs::jpanic!("error: {}", e));
                res.into()
            }
        } else {
            quote! {
                let res = #call_inner;
                res.into()
            }
        };

        quote! {
            #[janet_fn]
            #(#outer_attrs)* #outer_vis fn #outer_name(args: &mut [::janetrs::Janet]) -> ::janetrs::Janet {
                #[inline]
                #inner

                #(#args)*

                #call_inner
            }
        }
    } else {
        quote_spanned! {
            f.span() => compile_error!("expected fn item");
        }
    };

    //eprintln!("{}", ts.to_string());

    ts.into()
}
