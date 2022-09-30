use proc_macro2::Delimiter::{Brace, Bracket, Parenthesis};
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::buffer::{Cursor, TokenBuffer};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Ident, LitInt, Token};

struct SeqInfo {
    loop_ident: Ident,
    lit_start: syn::LitInt,
    lit_end: syn::LitInt,
    content: TokenStream,
}

struct TtCollectorBase<'a> {
    collected_tokens: Vec<TokenTree>,
    loop_ident: &'a Ident,
}

struct TtCollectorDefault<'a> {
    base: TtCollectorBase<'a>,
    current_index: usize,
}

struct TtCollectorInnerReps<'a> {
    base: TtCollectorBase<'a>,
    start: usize,
    end: usize,
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
    // dbg!("Content: {}", &input.content);
    let mut stream_repititions = Vec::new();
    // We meed to check the whole content for the the inner repition pattern #(...)*.
    // If this pattern is found, we need to repeat the inner repitition instead of repeating the
    // whole content. The TokenBuffer is suited for this task
    let tok_buf = TokenBuffer::new2(input.content);
    let mut current_cursor = Cursor::empty();
    let mut inner_repitition_found = false;

    while !current_cursor.eof() {
        if let Some((punct, _)) = current_cursor.punct() {
            if punct.as_char() == '#' && current_cursor.group(Bracket).is_some() {
                inner_repitition_found = true;
                break;
            }
        }
        let (_, next) = current_cursor.token_tree().unwrap();
        current_cursor = next;
    }
    Ok(if !inner_repitition_found {
        for next_num in start..end {
            current_cursor = tok_buf.begin();
            let mut tt_collector = TtCollectorDefault::new(&input.loop_ident, next_num);
            while !current_cursor.eof() {
                tt_collector.handle_cursor(&mut current_cursor);
            }
            stream_repititions.push(tt_collector.consume());
        }
        quote! {
            #(#stream_repititions)*
        }
    } else {
        let mut tt_collector = TtCollectorInnerReps::new(&input.loop_ident, start, end);
        while !current_cursor.eof() {
            // TODO: Implement
        }
        quote! {}
    })
}

impl<'a> TtCollectorBase<'a> {
    fn new(loop_ident: &'a Ident) -> Self {
        Self {
            collected_tokens: Vec::new(),
            loop_ident,
        }
    }

    fn consume(self) -> TokenStream {
        TokenStream::from_iter(self.collected_tokens)
    }
}

impl<'a> TtCollectorDefault<'a> {
    fn new(loop_ident: &'a Ident, current_index: usize) -> Self {
        Self {
            base: TtCollectorBase::new(loop_ident),
            current_index,
        }
    }
    fn consume(self) -> TokenStream {
        self.base.consume()
    }

    fn handle_cursor(&mut self, current_cursor: &mut Cursor) {
        if let Some((ident, next_cursor)) = current_cursor.ident() {
            if &ident == self.base.loop_ident {
                let lit_int = LitInt::new(&self.current_index.to_string(), Span::call_site());
                self.base.collected_tokens.push(lit_int.token().into());
                *current_cursor = next_cursor;
                return;
            }
        }
        if let Some((punct, next_cursor)) = current_cursor.punct() {
            if punct.as_char() == '~' {
                if let Some((ident, next_cursor)) = next_cursor.ident() {
                    if &ident == self.base.loop_ident {
                        if let Some(TokenTree::Ident(prefix)) = self.base.collected_tokens.last() {
                            let concat_str = prefix.to_string() + &self.current_index.to_string();
                            // Need to pop the last ident, will be replaced by completely new ident
                            self.base.collected_tokens.pop();
                            self.base
                                .collected_tokens
                                .push(Ident::new(&concat_str, Span::call_site()).into());
                            *current_cursor = next_cursor;
                            return;
                        }
                    }
                }
            }
        }
        if let Some((mut group_cursor, gspan, next_cursor)) = current_cursor.group(Parenthesis) {
            self.handle_group_cursor(Parenthesis, &mut group_cursor, gspan);
            *current_cursor = next_cursor;
            return;
        }
        if let Some((mut group_cursor, gspan, next_cursor)) = current_cursor.group(Brace) {
            self.handle_group_cursor(Brace, &mut group_cursor, gspan);
            *current_cursor = next_cursor;
            return;
        }
        let (tt, next_cursor) = current_cursor
            .token_tree()
            .expect("Cursor parsing configuration error. Reached unexpected EOF");
        // dbg!("Pushing TT {}", &tt);
        self.base.collected_tokens.push(tt);
        *current_cursor = next_cursor;
    }

    fn handle_group_cursor(&mut self, delim: Delimiter, group_cursor: &mut Cursor, gspan: Span) {
        let mut group_tt_collector =
            TtCollectorDefault::new(self.base.loop_ident, self.current_index);
        while !group_cursor.eof() {
            group_tt_collector.handle_cursor(group_cursor);
        }
        let mut group_token = Group::new(delim, group_tt_collector.consume());
        group_token.set_span(gspan);
        self.base.collected_tokens.push(group_token.into());
    }
}

impl<'a> TtCollectorInnerReps<'a> {
    fn new(loop_ident: &'a Ident, start: usize, end: usize) -> Self {
        Self {
            base: TtCollectorBase::new(loop_ident),
            start,
            end
        }
    }

    fn consume(self) -> TokenStream {
        self.base.consume()
    }
}