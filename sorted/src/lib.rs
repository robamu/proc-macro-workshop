use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, Item};

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_item = parse_macro_input!(input as Item);
    process_input(args, input_item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn process_input(_args: AttributeArgs, input: Item) -> syn::Result<TokenStream> {
    let mut variants_set = BTreeSet::new();
    match &input {
        Item::Enum(e) => {
            for variant in &e.variants {
                let variant_string = variant.ident.to_string();
                let rev_iter = variants_set.iter().rev();
                if let Some(last_element) = rev_iter.last() {
                    if variant_string < *last_element {
                        return Err(syn::Error::new(
                            variant.span(),
                            format!("{} should sort before {}", variant_string, last_element),
                        ));
                    }
                }
                variants_set.insert(variant_string);
            }
        }
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected enum or match expression",
            ))
        }
    }
    Ok(input.to_token_stream())
}
