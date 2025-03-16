use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Ident, ItemTrait, Meta, MetaList, Path};
use syn::{ItemEnum, Result, parse_macro_input};

pub(crate) fn expand_impl_macro_for_enum(
    attrs: Punctuated<Meta, Comma>,
    item: ItemTrait,
) -> Result<TokenStream> {
    println!("attr: {:#?}", attrs);
    println!("item: {:#?}", item);

    let mut macro_name = Option::<Ident>::None;

    for meta in attrs {
        match meta {
            Meta::NameValue(meta_name_value) => {
                let name = match meta_name_value.path.get_ident() {
                    Some(v) => v,
                    None => {
                        return Err(syn::Error::new(
                            meta_name_value.path.span(),
                            "expected ident",
                        ));
                    }
                };

                if name != "name" {
                    return Err(syn::Error::new(name.span(), "expected 'name'"));
                }

                let value = match meta_name_value.value {
                    syn::Expr::Path(expr_lit) => match expr_lit.path.get_ident() {
                        Some(v) => format_ident!("{}",v),
                        None => {
                            return Err(syn::Error::new(
                                expr_lit.path.span(),
                                "expected ident",
                            ));
                        }
                    },
                    // syn::Expr::Lit(expr_lit) => match expr_lit.lit {
                    //     syn::Lit::Str(lit_str) => Ident::new(&lit_str.value(), lit_str.span()),
                    //     _ => {
                    //         return Err(syn::Error::new(
                    //             expr_lit.span(),
                    //             "expected string literal",
                    //         ));
                    //     }
                    // },
                    oth => unimplemented!("meta={:?}", oth),
                };

                macro_name.replace(value);
            }
            oth => unimplemented!("meta={:?}", oth),
        }
    }

    if macro_name.is_none() {
        return Err(syn::Error::new(
            item.span(),
            "expected 'name' attribute",
        ));
    }


    

    Ok(quote!(#item))
}
