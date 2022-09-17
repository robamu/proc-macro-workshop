use syn::{parse_macro_input, DeriveInput};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.to_token_stream();
    let builder_ident = Ident::new(&format!("{}Builder", input.ident), Span::call_site());
    let tokens = quote! {
        impl #ident {
            pub fn builder() {}
        }

        pub struct #builder_ident {

        }
    };
    TokenStream::from(tokens)
}
