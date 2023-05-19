// use janetrs::{
//     janet_abstract::AbstractError, IsJanetAbstract, Janet, JanetAbstract, JanetConversionError,
// };
use proc_macro::TokenStream;
use quote::*;
use syn::{spanned::Spanned, *};

// enum MaybeAbstractError {
//     Abstract(AbstractError),
//     Kind(JanetConversionError),
// }

// We don't need this anymore but JanetAbstract needs a lifetime...
// enum MaybeAbstract {
//     Abstract(JanetAbstract),
//     Janet(Janet),
// }

// Support/test more native rust types (i.e. &str).
// TODO accept a final "rest" arg with a subslice?

fn is_container_type<'a>(ty: &'a Type, container_id: &'static str) -> Option<&'a Type> {
    match ty {
        Type::Path(TypePath { path, .. }) => path.segments.last().and_then(|s| {
            if s.ident == container_id {
                match &s.arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args, ..
                    }) => args.first().and_then(|arg| match arg {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    }),
                    _ => None,
                }
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn is_result(ty: &Type) -> Option<&Type> {
    is_container_type(ty, "Result")
}

fn is_option(ty: &Type) -> Option<&Type> {
    is_container_type(ty, "Option")
}

/// Wraps a function making it callable from Janet. The wrapped function can accept any number of
/// arguments that implement [TryFrom<Janet>]. Options in the inputs are nil-checked. Consecutive
/// trailing Options are considered optional when checking arity. A Result in the output is
/// unwrapped before returning to Janet.
#[proc_macro_attribute]
pub fn jfna(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as syn::Item);

    //eprintln!("{:?}", _attr);

    let ts = if let syn::Item::Fn(f) = f {
        let output_result =
            matches!(f.sig.output, ReturnType::Type(_, ref ty) if is_result(ty).is_some());

        let mut outer = f.clone();
        // Rewrite the outer to receive &mut [Janet]. TODO: check mut
        outer.sig.inputs = parse_quote! { args: &mut [Janet] };

        let mut inner = f.clone();
        inner.sig.ident = format_ident!("{}_inner", inner.sig.ident);

        // Strip inner attrs?
        inner.vis = Visibility::Inherited;

        // The assumption here is that any ref type is an abstract type and anything else converts
        // directly from a Janet.
        let unwrap_arg = |id, ty: &Type, i| {
            match ty {
                Type::Path(ty) => quote! {
                    let #id: #ty = args.get_panic(#i);
                },
                Type::Reference(TypeReference { mutability, .. }) => {
                    let get_fn = match mutability {
                        Some(_) => quote! { get_mut },
                        _ => quote! { get },
                    };
                    let id_abs = format_ident!("{}_abstract", id);
                    quote! {
                        let #id_abs: JanetAbstract = args.get_panic(#i);
                        let #id: #ty = #id_abs.#get_fn().unwrap_or_else(|e| ::janetrs::jpanic!("bad slot #{}: {}", #i, e));
                    }
                }
                //Type::Slice(_) => todo!(),
                //Type::Tuple(_) => todo!(),
                _ => {
                    quote_spanned! { ty.span() => ::janetrs::panic!("invalid input type: {}", ty) }
                }
            }
        };

        let (unwrap_args, (idents, is_opts)): (Vec<_>, (Vec<_>, Vec<_>)) = f
            .sig
            .inputs
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                // Create a new local identifier for each argument.
                let ident = format_ident!("arg{}", i);

                // Track optional arguments.
                let mut opt = false;

                let ts = match arg {
                    FnArg::Receiver(_) => {
                        quote_spanned! { arg.span() => compile_error!("found receiver") }
                    }
                    // Check that pat is an identifier?
                    FnArg::Typed(PatType { ty, .. }) => {
                        // If we are expecting an Option, check for nil and convert. TODO: get_option on JanetArgs?
                        if let Some(ty) = is_option(ty) {
                            opt = true;
                            let try_unwrap = unwrap_arg(ident.clone(), ty, i);
                            quote! {
                                let #ident = if #i >= args.len() || args[#i].is_nil() {
                                    None
                                } else {
                                    #try_unwrap
                                    Some(#ident)
                                };
                            }
                        } else {
                            unwrap_arg(ident.clone(), ty.as_ref(), i)
                        }
                    }
                };
                (ts, (ident, opt))
            })
            .unzip();

        // Trailing Options are optional.
        let nargs = f.sig.inputs.len();
        let nopts = is_opts.iter().rev().take_while(|o| **o).count();
        let janet_fn_attrs = vec![if nopts > 0 {
            let min = nargs - nopts;
            quote! { arity(range(#min, #nargs)) }
        } else {
            quote! { arity(fix(#nargs)) }
        }];

        // Output.
        let outer_attrs = outer.attrs;
        let outer_vis = outer.vis;
        let outer_name = outer.sig.ident;
        let inner_name = inner.sig.ident.clone();

        let call_inner = quote! { #inner_name(#(#idents),*) };

        // If the output type is a Result, unwrap it first.
        let call_inner = if output_result {
            quote! {
                let res = #call_inner.unwrap_or_else(|e| ::janetrs::jpanic!("error: {}", e));
                res.into() // Janet
            }
        } else {
            quote! {
                let res = #call_inner;
                res.into() // Janet
            }
        };

        quote! {
            #[janet_fn(#(#janet_fn_attrs),*)]
            #(#outer_attrs)* #outer_vis fn #outer_name(args: &mut [::janetrs::Janet]) -> ::janetrs::Janet {
                #[inline]
                #inner

                #(#unwrap_args)*

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
