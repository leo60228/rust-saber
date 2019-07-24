#![recursion_limit = "1024"]

extern crate proc_macro;

use proc_macro2::Span;
use quote::quote;
use std::convert::TryInto;
use std::panic;
use syn::parse::{Parse, ParseStream, Result};
use syn::{Abi, LitStr, Token};

struct HookArgs {
    address: u32,
    name: Option<String>,
}

macro_rules! proc_error {
    ($msg:expr) => {
        return syn::Error::new(Span::call_site(), $msg)
            .to_compile_error()
            .into();
    };
}

impl Parse for HookArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let int = input.parse::<syn::LitInt>()?;
        let string = input
            .parse::<Token![,]>()
            .and_then(|_| input.parse::<syn::LitStr>())
            .map(|lit| lit.value())
            .ok();
        Ok(HookArgs {
            address: int
                .value()
                .try_into()
                .map_err(|_| syn::Error::new(int.span(), "address too large"))?,
            name: string,
        })
    }
}

/// Hook a function to another function. This takes two arguments, the second of
/// which is optional. The first is the address of the hooked function relative
/// to the start of libil2cpp.so, and the second is the name of the mod to
/// initialize rust-saber with, defaulting to the crate name. The function used
/// must be unsafe. However, it does not need to have any specific ABI.
///
/// # Examples
/// ```rust,no_run
/// # #[repr(C)]
/// # #[derive(Default)]
/// # pub struct Color {
/// #     pub r: f32,
/// #     pub g: f32,
/// #     pub b: f32,
/// #     pub a: f32,
/// # }
/// #[rust_saber::hook(0x12DC59C, "sample_mod")]
/// pub unsafe fn get_color(orig: GetColorFn, this: *mut std::ffi::c_void) -> Color {
///     let orig_color = unsafe { orig(this) };
///     Color {
///         r: 1.0,
///         g: orig_color.g,
///         b: orig_color.b,
///         a: orig_color.a,
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn hook(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match panic::catch_unwind(move || hook_impl(attr, item)) {
        Ok(res) => res,
        Err(err) => {
            if let Ok(err) = err.downcast::<String>() {
                proc_error!(err);
            } else {
                panic!("internal macro error");
            }
        }
    }
}

fn hook_impl(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(attr as HookArgs);
    let addr = args.address;

    let name = match args.name {
        Some(name) => quote!(#name),
        None => quote!(env!("CARGO_PKG_NAME")),
    };

    let input = syn::parse_macro_input!(item as syn::ItemFn);
    if input.unsafety.is_none() {
        proc_error!("Hook must be unsafe!");
    }

    let orig_type = syn::TypeBareFn {
        lifetimes: None,
        unsafety: Some(Token![unsafe](Span::call_site())),
        abi: Some(Abi {
            extern_token: Token![extern](Span::call_site()),
            name: Some(LitStr::new("C", Span::call_site())),
        }),
        fn_token: input.decl.fn_token.clone(),
        paren_token: input.decl.paren_token.clone(),
        variadic: input.decl.variadic.clone(),
        inputs: input
            .decl
            .inputs
            .iter()
            .skip(1)
            .map(|arg| match arg {
                syn::FnArg::Captured(cap) => syn::BareFnArg {
                    name: match &cap.pat {
                        syn::Pat::Ident(ident) => Some((
                            syn::BareFnArgName::Named(ident.ident.clone()),
                            cap.colon_token.clone(),
                        )),
                        syn::Pat::Wild(wild) => Some((
                            syn::BareFnArgName::Wild(wild.underscore_token),
                            cap.colon_token.clone(),
                        )),
                        _ => None,
                    },
                    ty: cap.ty.clone(),
                },
                syn::FnArg::Ignored(ignored) => syn::BareFnArg {
                    name: None,
                    ty: ignored.clone(),
                },
                _ => panic!("Unsupported argument!"),
            })
            .collect(),
        output: input.decl.output.clone(),
    };

    let orig_type_ident = if let syn::FnArg::Captured(cap) = input
        .decl
        .inputs
        .first()
        .clone()
        .expect("No arguments in hook!")
        .into_value()
    {
        if let syn::Type::Path(path) = &cap.ty {
            path.path
                .segments
                .last()
                .clone()
                .unwrap()
                .into_value()
                .ident
                .clone()
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
    wrapper.abi = Some(Abi {
        extern_token: Token![extern](Span::call_site()),
        name: Some(LitStr::new("C", Span::call_site())),
    });
    wrapper.decl.inputs = wrapper.decl.inputs.into_iter().skip(1).collect();
    let args: syn::punctuated::Punctuated<syn::Ident, Token![,]> = wrapper
        .decl
        .inputs
        .iter()
        .map(|arg| match arg {
            syn::FnArg::Captured(cap) => match &cap.pat {
                syn::Pat::Ident(ident) => ident.ident.clone(),
                _ => unimplemented!(),
            },
            _ => panic!("Unsupported argument!"),
        })
        .collect();
    wrapper.block = Box::new(syn::parse_quote!({
        #ident(#orig_ident.unwrap(), #args)
    }));

    let codegen = quote! {
        #input

        type #orig_type_ident = #orig_type;

        #[used]
        #[allow(non_upper_case_globals)]
        #[link_section = ".init_array"]
        #[no_mangle]
        static #ctor_ident: [extern "C" fn(); 1] = {
            #wrapper

            static mut orig: Option<#orig_type_ident> = None;

            extern fn apply() {
                ::rust_saber::init_once(#name);
                unsafe { orig = Some(std::mem::transmute(::rust_saber::hook(wrapper as #orig_type_ident as u32, #addr))); }
            };

            [apply]
        };
    };

    codegen.into()
}
