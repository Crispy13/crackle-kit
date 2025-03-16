mod expand;

use expand::expand_impl_macro_for_enum;
use proc_macro::TokenStream;
use syn::{parse::{self, Parser}, parse_macro_input, ItemEnum, ItemImpl, ItemTrait, Meta, MetaList};

#[proc_macro_attribute]
pub fn impl_macro_for_enum(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
        .parse(attr)
    {
        Ok(v) => v,
        Err(err) => return err.into_compile_error().into(),
    };
    // let attr = parse_macro_input!(attr as MetaList);

    let item = parse_macro_input!(item as ItemTrait);

    expand_impl_macro_for_enum(attr, item)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

