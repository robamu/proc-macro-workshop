use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let input_copy = input.clone();
    let input_item = parse_macro_input!(input_copy as syn::Item);
    process_input(input_item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn process_input(input: syn::Item) -> syn::Result<TokenStream> {
    Ok(input.to_token_stream())
}
