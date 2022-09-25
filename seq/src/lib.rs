use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parse_macro_input, token, DeriveInput, Field, Ident, Token};

enum Item {
    SeqStruct(SeqStruct),
}

struct SeqStruct {
    loop_ident: Ident,
    in_token: Token![in],
    lit_start: syn::LitInt,
    range_token: Token![..],
    lit_end: syn::LitInt,
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            input.parse().map(Item::SeqStruct)
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for SeqStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(SeqStruct {
            loop_ident: input.parse()?,
            in_token: input.parse()?,
            lit_start: input.parse()?,
            range_token: input.parse()?,
            lit_end: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    //let input = parse_macro_input!(input as Item);

    proc_macro::TokenStream::new()
}
