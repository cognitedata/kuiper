use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, Data, DeriveInput, Generics, Ident, LitStr, Pat, Result,
    Signature, Token, WhereClause,
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
        if attr.path().get_ident().map(|i| i.to_string()) == Some("pass_through".to_string()) {
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
        let wh = funcs.first().unwrap().where_clause.clone();
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
                panic!("Function {funcname} must have self for pass through");
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
                        let seg = a.path().segments.last();
                        match seg {
                            None => false,
                            Some(seg) => seg.ident == "pass_through_exclude",
                        }
                    })
                    .map(|ex| {
                        let to_exclude: IdentList = ex.parse_args().unwrap();
                        to_exclude.items.contains(&funcname)
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
        let mut imp = match trt {
            Some(x) => {
                let generics = generics.unwrap();
                quote! { impl #generics #x #generics for #name }
            }
            None => quote! { impl #name },
        };

        if let Some(wh) = wh {
            imp = quote! { #imp #wh };
        }

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
            ok = next_result.is_ok();
        }
        Ok(IdentList { items: result })
    }
}

struct FuncAndError {
    sign: Signature,
    err: String,
    trt: Option<Ident>,
    generics: Option<Generics>,
    where_clause: Option<WhereClause>,
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
                    where_clause: None,
                })
            }
        }
        let trt: Ident = input.parse()?;
        let generics: Generics = input.parse().unwrap();
        match input.parse::<Token![,]>() {
            Ok(_) => (),
            Err(_) => {
                return Ok(FuncAndError {
                    sign,
                    err: errit.value(),
                    trt: Some(trt),
                    generics: Some(generics),
                    where_clause: None,
                })
            }
        }
        let wh: WhereClause = input.parse()?;
        Ok(FuncAndError {
            sign,
            err: errit.value(),
            trt: Some(trt),
            generics: Some(generics),
            where_clause: Some(wh),
        })
    }
}

#[proc_macro_derive(SourceData)]
/// Macro for deriving the SourceData trait for a struct.
///
/// This only works for structs with named fields, where each field also implements SourceData.
pub fn source_data_derive(d: TokenStream) -> TokenStream {
    let en = parse_macro_input!(d as DeriveInput);
    let name = en.ident.clone();

    let mut keys_block = quote! {};
    let mut get_key_block = quote! {};

    match &en.data {
        Data::Struct(fields) => {
            for field in &fields.fields {
                let Some(ident) = &field.ident else {
                    return syn::Error::new(
                        name.span(),
                        "SourceData can only be derived for structs with named fields",
                    )
                    .to_compile_error()
                    .into();
                };

                let field_name = ident.to_string();
                keys_block.extend(quote! {
                    #field_name,
                });
                get_key_block.extend(quote! {
                    #field_name => &self.#ident,
                });
            }
        }
        _ => {
            return syn::Error::new(name.span(), "SourceData can only be derived for structs")
                .to_compile_error()
                .into()
        }
    }

    let output = quote! {
        impl kuiper_lang::source::SourceData for #name {
            fn resolve(&self) -> kuiper_lang::ResolveResult<'_> {
                ResolveResult::Owned(serde_json::to_value(self).unwrap_or(serde_json::Value::Null))
            }

            fn get_key(&self, key: &str) -> &dyn kuiper_lang::source::SourceData {
                match key {
                    #get_key_block
                    _ => &kuiper_lang::NULL_CONST,
                }
            }

            fn keys(&self) -> Box<dyn Iterator<Item = &str> + '_> {
                Box::new([
                    #keys_block
                ].into_iter())
            }
        }
    };

    output.into()
}
