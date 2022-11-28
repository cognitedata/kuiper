use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, DeriveInput, Generics, Ident, LitStr, Pat, Result, Signature,
    Token, Type,
};

#[proc_macro_derive(PassThrough, attributes(pass_through_exclude))]
pub fn pass_through_derive(_: TokenStream) -> TokenStream {
    TokenStream::new()
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

#[proc_macro_attribute]
pub fn pass_through(args: TokenStream, input: TokenStream) -> TokenStream {
    let en = parse_macro_input!(input as DeriveInput);
    let data = match &en.data {
        syn::Data::Enum(x) => x,
        syn::Data::Union(_) | syn::Data::Struct(_) => panic!("Input must be an enum"),
    };

    let args: FuncAndError = parse_macro_input!(args);
    let func: Signature = args.sign;

    let name = en.ident.clone();
    let funcname = func.ident.clone();

    let funcargs = &func.inputs;
    let has_self = funcargs.iter().any(|f| match f {
        syn::FnArg::Receiver(_) => true,
        syn::FnArg::Typed(_) => false,
    });
    let mapped_funcargs: Vec<&Box<Pat>> = funcargs
        .iter()
        .filter_map(|f| match f {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(x) => Some(&x.pat),
        })
        .collect();

    if !has_self {
        panic!("Functions must have self for pass through");
    }

    let mut arms = quote! {};
    let mut any_excluded = false;
    for variant in &data.variants {
        let exclude = variant.attrs.iter().find(|a| {
            let seg = a.path.segments.last();
            match seg {
                None => false,
                Some(seg) => seg.ident.to_string() == "pass_through_exclude",
            }
        });

        if let Some(exclude) = exclude {
            let to_exclude: IdentList = exclude.parse_args().unwrap();
            if to_exclude
                .items
                .iter()
                .any(|idt| idt.to_string() == funcname.to_string())
            {
                any_excluded = true;
                continue;
            }

            // let ident: Ident = parse_macro_input!(tokens);
        }

        let itemident = Ident::new("a", Span::call_site());
        let path = &variant.ident;
        arms.extend(quote! { #name::#path(a) => #itemident.#funcname(#(#mapped_funcargs),*), });
    }

    let err = args.err;

    let pb = match &args.trt {
        Some(_) => quote! {},
        None => quote! { pub },
    };

    let imp = match args.trt {
        Some(x) => {
            let generics = args.generics;
            let generics = generics.unwrap();
            let dt = quote! { #generics}.to_string();
            quote! { impl #generics #x #generics for #name }
        }
        None => quote! { impl #name },
    };

    let mut res: proc_macro2::TokenStream = if any_excluded {
        quote! {
            #imp {
                #pb #func {
                    match self {
                        #arms
                        _ => panic!(#err)
                    }
                }
            }
        }
        .into()
    } else {
        quote! {
            #imp {
                #pb #func {
                    match self {
                        #arms
                    }
                }
            }
        }
        .into()
    };

    res.extend(en.into_token_stream());
    res.into()
}
