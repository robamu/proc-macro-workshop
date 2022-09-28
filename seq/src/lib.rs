use proc_macro2::{Group, TokenStream, TokenTree};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Ident, LitInt, Token};

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
    gen_output(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn recursive_modified_tt_collector(
    modified_tokens: &mut Vec<TokenTree>,
    loop_ident: &Ident,
    token_tree: &TokenTree,
    next_num: usize,
) {
    match token_tree {
        TokenTree::Group(g) => {
            let mut modified_tokens_for_group = Vec::new();
            for ref tt in g.stream() {
                recursive_modified_tt_collector(
                    &mut modified_tokens_for_group,
                    loop_ident,
                    tt,
                    next_num,
                );
            }
            let mut new_group = Group::new(
                g.delimiter().clone(),
                TokenStream::from_iter(modified_tokens_for_group),
            );
            new_group.set_span(g.span());
            modified_tokens.push(new_group.into());
        }
        TokenTree::Ident(ref i) => {
            if i == loop_ident {
                let lit_int = LitInt::new(&next_num.to_string(), token_tree.span());
                modified_tokens.push(lit_int.token().into());
            } else {
                modified_tokens.push(token_tree.clone());
            }
        }
        _ => {
            modified_tokens.push(token_tree.clone());
        }
    }
}

fn gen_output(input: SeqInfo) -> syn::Result<TokenStream> {
    let start = input.lit_start.base10_parse::<usize>()?;
    let end = input.lit_end.base10_parse::<usize>()?;
    let mut tt_repititions = Vec::new();
    for next_num in start..end {
        let content_copy = input.content.clone();
        let mut modified_tokens = Vec::new();
        for ref tt in content_copy {
            recursive_modified_tt_collector(&mut modified_tokens, &input.loop_ident, tt, next_num);
        }
        if next_num == 0 {}
        tt_repititions.push(TokenStream::from_iter(modified_tokens));
    }
    let output = quote! {
       #(#tt_repititions)*
    };
    Ok(output)
}
