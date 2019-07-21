#![recursion_limit = "1024"]

extern crate proc_macro;

use std::convert::TryInto;
use quote::quote;
use syn::{Abi, Token, LitStr};
use syn::parse::{Parse, ParseStream, Result};
use proc_macro2::Span;

struct HookArgs {
    address: u32,
    name: String,
}

impl Parse for HookArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let int = input.parse::<syn::LitInt>()?;
        input.parse::<Token![,]>()?;
        let string = input.parse::<syn::LitStr>()?;
        Ok(HookArgs {
            address: int.value().try_into().map_err(|_| syn::Error::new(int.span(), "address too large"))?,
            name: string.value(),
        })
    }
}

#[proc_macro_attribute]
pub fn hook(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(attr as HookArgs);
    let addr = args.address;
    let name = args.name;

    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    input.abi = Some(Abi {
        extern_token: Token![extern](Span::call_site()),
        name: Some(LitStr::new("C", Span::call_site())),
    });

    let orig_type = syn::TypeBareFn {
        lifetimes: None,
        unsafety: Some(Token![unsafe](Span::call_site())),
        abi: input.abi.clone(),
        fn_token: input.decl.fn_token.clone(),
        paren_token: input.decl.paren_token.clone(),
        variadic: input.decl.variadic.clone(),
        inputs: input.decl.inputs.iter().skip(1).map(|arg| match arg {
            syn::FnArg::Captured(cap) => syn::BareFnArg {
                name: match &cap.pat {
                    syn::Pat::Ident(ident) => Some((syn::BareFnArgName::Named(ident.ident.clone()), cap.colon_token.clone())),
                    syn::Pat::Wild(wild) => Some((syn::BareFnArgName::Wild(wild.underscore_token), cap.colon_token.clone())),
                    _ => None,
                },
                ty: cap.ty.clone(),
            },
            syn::FnArg::Ignored(ignored) => syn::BareFnArg {
                name: None,
                ty: ignored.clone(),
            },
            _ => panic!("Unsupported argument!"),
        }).collect(),
        output: input.decl.output.clone(),
    };

    let orig_type_ident = if let syn::FnArg::Captured(cap) = input.decl.inputs.first().clone().unwrap().into_value() {
        if let syn::Type::Path(path) = &cap.ty {
            path.path.segments.last().clone().unwrap().into_value().ident.clone()
        } else {
            panic!("Invalid orig!");
        }
    } else {
        panic!("Missing orig!");
    };

    let mut ctor_ident_str = input.ident.to_string();
    ctor_ident_str.push_str("_apply");
    let ctor_ident = syn::Ident::new(&ctor_ident_str, Span::call_site());
    let ident = &input.ident;
    let orig_ident = syn::Ident::new("orig", Span::call_site());

    let mut wrapper = input.clone();
    wrapper.ident = syn::Ident::new("wrapper", Span::call_site());
    wrapper.decl.inputs = wrapper.decl.inputs.into_iter().skip(1).collect();
    let args: syn::punctuated::Punctuated<syn::Ident, Token![,]> = wrapper.decl.inputs.iter().map(|arg| match arg {
        syn::FnArg::Captured(cap) => match &cap.pat {
            syn::Pat::Ident(ident) => ident.ident.clone(),
            _ => unimplemented!(),
        },
        _ => panic!("Unsupported argument!"),
    }).collect();
    wrapper.block = Box::new(syn::parse_quote!({
        #ident(unsafe { #orig_ident.unwrap() }, #args)
    }));

    let codegen = quote! {
        #input

        type #orig_type_ident = #orig_type;

        #[used]
        #[allow(non_upper_case_globals)]
        #[link_section = ".init_array"]
        #[no_mangle]
        static #ctor_ident: extern "C" fn() = {
            #wrapper

            static mut orig: Option<#orig_type_ident> = None;

            extern fn apply() {
                ::rust_saber::init_once(#name);
                unsafe { orig = Some(std::mem::transmute(::rust_saber::hook(wrapper as #orig_type_ident as u32, #addr))); }
            };

            apply
        };
    };

    codegen.into()
}
