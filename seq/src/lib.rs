use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Ident, LitInt, Token};

struct SeqInfo {
    loop_ident: Ident,
    lit_start: syn::LitInt,
    lit_end: syn::LitInt,
    content: TokenStream,
}

struct TtCollector<'a> {
    collected_tokens: Vec<TokenTree>,
    loop_ident: &'a Ident,
    current_index: usize,
    last_ident_for_tilde_check: Option<Ident>,
    ident_and_tilde_found: bool,
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

fn gen_output(input: SeqInfo) -> syn::Result<TokenStream> {
    let start = input.lit_start.base10_parse::<usize>()?;
    let end = input.lit_end.base10_parse::<usize>()?;
    //dbg!("Content: {}", &input.content);
    let mut tt_repititions = Vec::new();
    for next_num in start..end {
        let content_copy = input.content.clone();
        let mut tt_collector = TtCollector::new(next_num, &input.loop_ident);
        for ref tt in content_copy {
            tt_collector.parse_next_tt(tt);
        }
        tt_repititions.push(tt_collector.consume());
    }
    let output = quote! {
       #(#tt_repititions)*
    };
    Ok(output)
}

impl<'a> TtCollector<'a> {
    fn new(current_index: usize, loop_ident: &'a Ident) -> Self {
        Self {
            collected_tokens: Vec::new(),
            current_index,
            loop_ident,
            last_ident_for_tilde_check: None,
            ident_and_tilde_found: false,
        }
    }

    fn consume(self) -> TokenStream {
        TokenStream::from_iter(self.collected_tokens)
    }

    fn push_last_ident(&mut self) {
        if let Some(ident) = self.last_ident_for_tilde_check.take() {
            self.collected_tokens.push(ident.into());
        }
    }

    /// This function works recursively when a group is found.
    fn parse_next_tt(&mut self, tt: &TokenTree) {
        let mut do_push_last_ident = true;
        let tt_to_push = match tt {
            TokenTree::Group(g) => {
                let mut tt_collector = TtCollector::new(self.current_index, self.loop_ident);
                for ref tt in g.stream() {
                    tt_collector.parse_next_tt(tt);
                }
                let mut new_group = Group::new(g.delimiter(), tt_collector.consume());
                new_group.set_span(g.span());
                Some(new_group.into())
            }
            TokenTree::Ident(i) => {
                if i == self.loop_ident {
                    if self.ident_and_tilde_found {
                        let ident_str = self.last_ident_for_tilde_check.take().unwrap().to_string()
                            + &self.current_index.to_owned().to_string();
                        self.ident_and_tilde_found = false;
                        Some(Ident::new(&ident_str, Span::call_site()).into())
                    } else {
                        let lit_int = LitInt::new(&self.current_index.to_string(), tt.span());
                        Some(lit_int.token().into())
                    }
                } else {
                    // Push old identifier if there is one, but do not push the current one
                    if let Some(last_ident) = self.last_ident_for_tilde_check.replace(i.clone()) {
                        self.collected_tokens.push(last_ident.into());
                    }
                    do_push_last_ident = false;
                    None
                }
            }
            TokenTree::Punct(p) => match p.as_char() {
                '~' => {
                    if self.last_ident_for_tilde_check.is_some() {
                        self.ident_and_tilde_found = true;
                        do_push_last_ident = false;
                        None
                    } else {
                        Some(tt.clone())
                    }
                }
                _ => Some(tt.clone()),
            },
            _ => Some(tt.clone()),
        };
        if do_push_last_ident {
            self.push_last_ident();
        }
        if let Some(tt_to_push) = tt_to_push {
            self.collected_tokens.push(tt_to_push)
        }
    }
}
