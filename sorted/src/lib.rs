use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
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

fn process_input(args: AttributeArgs, input: Item) -> syn::Result<TokenStream> {
    match input {
        Item::Enum(_) => {}
        _ => {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected enum or match expression",
            ))
        }
    }
    Ok(input.to_token_stream())
}
