use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Ident, Token};

struct SeqInfo {
    loop_ident: Ident,
    lit_start: syn::LitInt,
    lit_end: syn::LitInt,
    content: TokenStream,
}

impl Parse for SeqInfo {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let loop_ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let lit_start = input.parse()?;
        input.parse::<Token![..]>()?;
        let lit_end = input.parse()?;
        let content;
        braced!(content in input);
        let content = content.parse()?;
        Ok(SeqInfo {
            loop_ident,
            lit_start,
            lit_end,
            content,
        })
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as SeqInfo);

    proc_macro::TokenStream::new()
}
