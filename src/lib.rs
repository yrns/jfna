use proc_macro::TokenStream;
use quote::*;
use syn::{spanned::Spanned, *};

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
/// unwrapped before returning to Janet. If the input type is a reference it is assumed to implement
/// [IsJanetAbstract] and unwrapped accordingly.
#[proc_macro_attribute]
pub fn jfna(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as syn::Item);

    //eprintln!("{:?}", _attr);

    let ts = if let syn::Item::Fn(f) = f {
        let output_result =
            matches!(f.sig.output, ReturnType::Type(_, ref ty) if is_result(ty).is_some());

        let mut outer = f.clone();
        // Rewrite the outer to receive &mut [Janet].
        outer.sig.inputs = parse_quote! { args: &mut [Janet] };

        let mut inner = f.clone();
        inner.sig.ident = format_ident!("{}_inner", inner.sig.ident);

        // Strip inner attrs?
        inner.vis = Visibility::Inherited;

        let (unwrap_args, (idents, is_opts)): (Vec<_>, (Vec<_>, Vec<_>)) = f
            .sig
            .inputs
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                // Create a new local identifier for each argument.
                let ident = format_ident!("arg{}", i);

                // This is used to hold the abstract value. The lifetime of the inner reference is
                // not tied to the original Janet, so we have to keep this around until we call the
                // inner function.
                let ident_abs = format_ident!("arg{}_abs", i);

                // Track optional arguments.
                let mut opt = false;

                let ts = match arg {
                    FnArg::Receiver(_) => {
                        quote_spanned! { arg.span() => compile_error!("found receiver") }
                    }
                    // Check that pat is an identifier?
                    FnArg::Typed(PatType { ty, .. }) => {
                        // This covers Option<T> and T, where T is any of T or &T or &mut T:
                        if let Some(inner_ty) = is_option(ty) {
                            opt = true;
                            match inner_ty {
                                Type::Path(_) => quote! {
                                    let #ident: #ty = args.get(#i).filter(|j| !j.is_nil()).map(|j| j.try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("bad slot #{}, expected {}, got {}", #i, stringify!(ty), j.kind())));
                                },
                                Type::Reference(TypeReference { mutability, .. }) => {
                                    let get = match mutability {
                                        Some(_) => quote! { get_mut },
                                        _ => quote! { get },
                                    };
                                    quote! {
                                        let #ident_abs: Option<JanetAbstract> = args.get(#i).filter(|j| !j.is_nil()).map(|j| j.try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("bad slot #{}, expected Abstract, got {}", #i, j.kind())));
                                        let #ident: #ty = #ident_abs.as_ref().map(|a| a.#get().unwrap_or_else(|e| ::janetrs::jpanic!("bad slot #{}, expected {}, got abstract error {}", #i, stringify!(#ty), e)));
                                    }
                                }
                                _ => {
                                    quote_spanned! { ty.span() => ::janetrs::panic!("invalid input type: {}", ty) }
                                }
                            }
                        } else {
                            match ty.as_ref() {
                                Type::Path(_) => quote! {
                                    let #ident: #ty = args.get(#i).unwrap().try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("bad slot #{}, expected {}, got {}", #i, stringify!(ty), j.kind()));
                                },
                                Type::Reference(TypeReference { mutability, .. }) => {
                                    let get = match mutability {
                                        Some(_) => quote! { get_mut },
                                        _ => quote! { get },
                                    };
                                    quote! {
                                        let arg = args.get(#i).unwrap();
                                        let #ident_abs: JanetAbstract = arg.try_unwrap().unwrap_or_else(|_e| ::janetrs::jpanic!("bad slot #{}, expected Abstract, got {}", #i, arg.kind()));
                                        let #ident: #ty = #ident_abs.#get().unwrap_or_else(|e| ::janetrs::jpanic!("bad slot #{}, expected {}, got abstract error {}", #i, stringify!(#ty), e));
                                    }
                                }
                                _ => {
                                    quote_spanned! { ty.span() => ::janetrs::panic!("invalid input type: {}", ty) }
                                }
                            }
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
