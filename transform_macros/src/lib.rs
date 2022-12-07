use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, DeriveInput, Generics, Ident, LitStr, Pat, Result, Signature,
    Token,
};

#[proc_macro_derive(PassThrough, attributes(pass_through_exclude, pass_through))]
pub fn pass_through_derive(d: TokenStream) -> TokenStream {
    let en = parse_macro_input!(d as DeriveInput);
    let data = match &en.data {
        syn::Data::Enum(x) => x,
        syn::Data::Union(_) | syn::Data::Struct(_) => panic!("Input must be an enum"),
    };
    let name = en.ident.clone();

    let mut by_trait_or_none = HashMap::<Option<String>, Vec<FuncAndError>>::new();
    for attr in en.attrs {
        if attr.path.get_ident().map(|i| i.to_string()) == Some("pass_through".to_string()) {
            let args: FuncAndError = attr.parse_args().unwrap();
            let key = args.trt.clone().map(|i| i.to_string());
            if let Some(funcs) = by_trait_or_none.get_mut(&key) {
                funcs.push(args)
            } else {
                by_trait_or_none.insert(key, vec![args]);
            }
        }
    }

    let mut output = quote! {};
    for (_, funcs) in by_trait_or_none {
        let trt = funcs.first().unwrap().trt.clone();
        let generics = funcs.first().unwrap().generics.clone();
        let mut methods = quote! {};
        for func in funcs {
            let sign: Signature = func.sign;
            let funcname = sign.ident.clone();
            let funcargs = &sign.inputs;
            let has_self = funcargs.iter().any(|f| match f {
                syn::FnArg::Receiver(_) => true,
                syn::FnArg::Typed(_) => false,
            });
            if !has_self {
                panic!("Function {} must have self for pass through", funcname);
            }

            let mapped_funcargs: Vec<&Box<Pat>> = funcargs
                .iter()
                .filter_map(|f| match f {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(x) => Some(&x.pat),
                })
                .collect();

            let mut arms = quote! {};
            let mut any_excluded = false;
            for variant in &data.variants {
                let exclude = variant
                    .attrs
                    .iter()
                    .find(|a| {
                        let seg = a.path.segments.last();
                        match seg {
                            None => false,
                            Some(seg) => seg.ident.to_string() == "pass_through_exclude",
                        }
                    })
                    .map(|ex| {
                        let to_exclude: IdentList = ex.parse_args().unwrap();
                        to_exclude
                            .items
                            .iter()
                            .any(|idt| idt.to_string() == funcname.to_string())
                    })
                    .unwrap_or(false);
                any_excluded |= exclude;
                if exclude {
                    continue;
                }
                let itemident = Ident::new("a", Span::call_site());
                let path = &variant.ident;
                arms.extend(
                    quote! { #name::#path(a) => #itemident.#funcname(#(#mapped_funcargs),*), },
                );
            }

            if any_excluded {
                let err = func.err;
                arms.extend(quote! { _ => panic!(#err)})
            }

            let pb = match &trt {
                Some(_) => quote! {},
                None => quote! { pub },
            };

            methods.extend(quote! {
                #pb #sign {
                    match self {
                        #arms
                    }
                }
            });
        }
        let imp = match trt {
            Some(x) => {
                let generics = generics.unwrap();
                quote! { impl #generics #x #generics for #name }
            }
            None => quote! { impl #name },
        };

        output.extend(quote! {
            #imp {
                #methods
            }
        })
    }

    output.into()
}

struct IdentList {
    items: Vec<Ident>,
}

impl Parse for IdentList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = Vec::new();
        let mut ok = true;
        while ok {
            result.push(input.parse()?);
            let next_result = input.parse::<Token![,]>();
            ok = match next_result {
                Result::Ok(_) => true,
                Result::Err(_) => false,
            };
        }
        Ok(IdentList { items: result })
    }
}

struct FuncAndError {
    sign: Signature,
    err: String,
    trt: Option<Ident>,
    generics: Option<Generics>,
}

impl Parse for FuncAndError {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let sign: Signature = input.parse()?;
        input.parse::<Token![,]>()?;
        let errit: LitStr = input.parse()?;
        match input.parse::<Token![,]>() {
            Ok(_) => (),
            Err(_) => {
                return Ok(FuncAndError {
                    sign,
                    err: errit.value(),
                    trt: None,
                    generics: None,
                })
            }
        }
        let trt: Ident = input.parse()?;
        Ok(FuncAndError {
            sign,
            err: errit.value(),
            trt: Some(trt),
            generics: Some(input.parse().unwrap()),
        })
    }
}
