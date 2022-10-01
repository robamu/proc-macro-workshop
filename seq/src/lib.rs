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
    inclusive_upper_range: bool,
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
        let inclusive_upper_range = input.parse::<Token![=]>().is_ok();
        let lit_end = input.parse()?;
        let content;
        braced!(content in input);
        let content = content.parse()?;
        Ok(SeqInfo {
            loop_ident,
            lit_start,
            lit_end,
            inclusive_upper_range,
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
    let mut end = input.lit_end.base10_parse::<usize>()?;
    if input.inclusive_upper_range {
        end += 1;
    }
    // dbg!("Content: {}", &input.content);
    let mut stream_repititions = Vec::new();
    // We meed to check the whole content for the the inner repition pattern #(...)*.
    // If this pattern is found, we need to repeat the inner repitition instead of repeating the
    // whole content. The TokenBuffer is suited for this task
    let tok_buf = TokenBuffer::new2(input.content);
    let mut current_cursor = tok_buf.begin();
    let inner_repitition_found = recursive_inner_reps_check(&mut current_cursor);
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
        current_cursor = tok_buf.begin();
        while !current_cursor.eof() {
            tt_collector.handle_cursor(&mut current_cursor);
        }
        tt_collector.consume()
    })
}

fn recursive_inner_reps_check(current_cursor: &mut Cursor) -> bool {
    while !current_cursor.eof() {
        if check_for_inner_reps(current_cursor) {
            return true;
        }

        if group_checks(current_cursor) {
            return true;
        }
        let (_, next_cursor) = current_cursor.token_tree().unwrap();
        *current_cursor = next_cursor;
    }
    false
}

fn group_checks(current_cursor: &Cursor) -> bool {
    if let Some((mut gcursor, _, _)) = current_cursor.group(Brace) {
        if recursive_inner_reps_check(&mut gcursor) {
            return true;
        }
    }
    if let Some((mut gcursor, _, _)) = current_cursor.group(Parenthesis) {
        if recursive_inner_reps_check(&mut gcursor) {
            return true;
        }
    }
    if let Some((mut gcursor, _, _)) = current_cursor.group(Bracket) {
        if recursive_inner_reps_check(&mut gcursor) {
            return true;
        }
    }
    false
}

fn check_for_inner_reps(current_cursor: &Cursor) -> bool {
    if let Some((punct, next_cursor)) = current_cursor.punct() {
        if punct.as_char() == '#' {
            if let Some((_, _, next_cursor)) = next_cursor.group(Parenthesis) {
                if let Some((punct, _)) = next_cursor.punct() {
                    if punct.as_char() == '*' {
                        return true;
                    }
                }
            }
        }
    }
    false
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
            end,
        }
    }

    fn consume(self) -> TokenStream {
        self.base.consume()
    }

    fn consume_as_tok_vec(self) -> Vec<TokenTree> {
        self.base.collected_tokens
    }

    fn handle_cursor(&mut self, current_cursor: &mut Cursor) {
        if let Some((punct, cursor_after_hash)) = current_cursor.punct() {
            if punct.as_char() == '#' {
                if let Some((gcursor, _, cursor_after_group)) = cursor_after_hash.group(Parenthesis)
                {
                    if let Some((punct, next_cursor)) = cursor_after_group.punct() {
                        if punct.as_char() == '*' {
                            for next_idx in self.start..self.end {
                                let mut group_cursor_local = gcursor;
                                let mut inner_collector = TtCollectorInnerReps::new(
                                    self.base.loop_ident,
                                    self.start,
                                    self.end,
                                );
                                while !group_cursor_local.eof() {
                                    inner_collector
                                        .handle_inner_rep_cursor(&mut group_cursor_local, next_idx);
                                }
                                self.base
                                    .collected_tokens
                                    .append(&mut inner_collector.consume_as_tok_vec());
                            }
                            *current_cursor = next_cursor;
                            return;
                        }
                    }
                }
            }
        }
        if self.check_outer_groups(current_cursor) {
            return;
        }
        let (tt, next_cursor) = current_cursor
            .token_tree()
            .expect("Cursor parsing configuration error. Reached unexpected EOF");
        self.base.collected_tokens.push(tt);
        *current_cursor = next_cursor;
    }

    fn check_outer_groups(&mut self, current_cursor: &mut Cursor) -> bool {
        let mut group_check = |delim| {
            if let Some((mut group_cursor, gspan, next)) = current_cursor.group(delim) {
                self.handle_group_cursor_outer(delim, &mut group_cursor, gspan);
                *current_cursor = next;
                return true;
            }
            false
        };
        if group_check(Parenthesis) {
            return true;
        }
        if group_check(Brace) {
            return true;
        }
        if group_check(Bracket) {
            return true;
        }
        false
    }

    fn handle_inner_rep_cursor(&mut self, group_cursor: &mut Cursor, current_index: usize) {
        if let Some((ident, next_cursor)) = group_cursor.ident() {
            if &ident == self.base.loop_ident {
                let lit_int = LitInt::new(&current_index.to_string(), Span::call_site());
                self.base.collected_tokens.push(lit_int.token().into());
                *group_cursor = next_cursor;
                return;
            }
        }
        if let Some((punct, next_cursor)) = group_cursor.punct() {
            if punct.as_char() == '~' {
                if let Some((ident, next_cursor)) = next_cursor.ident() {
                    if &ident == self.base.loop_ident {
                        if let Some(TokenTree::Ident(prefix)) = self.base.collected_tokens.last() {
                            let concat_str = prefix.to_string() + &current_index.to_string();
                            // Need to pop the last ident, will be replaced by completely new ident
                            self.base.collected_tokens.pop();
                            self.base
                                .collected_tokens
                                .push(Ident::new(&concat_str, Span::call_site()).into());
                            *group_cursor = next_cursor;
                            return;
                        }
                    }
                }
            }
        }
        if let Some((mut inner_group, gspan, next_cursor)) = group_cursor.group(Parenthesis) {
            self.handle_group_cursor_inner(Parenthesis, &mut inner_group, gspan, current_index);
            *group_cursor = next_cursor;
            return;
        }
        if let Some((mut inner_group, gspan, next_cursor)) = group_cursor.group(Brace) {
            self.handle_group_cursor_inner(Brace, &mut inner_group, gspan, current_index);
            *group_cursor = next_cursor;
            return;
        }
        let (tt, next_cursor) = group_cursor
            .token_tree()
            .expect("Inner cursor parsing configuration error. Reached unexpected EOF");
        self.base.collected_tokens.push(tt);
        *group_cursor = next_cursor;
    }

    fn handle_group_cursor_outer(
        &mut self,
        delim: Delimiter,
        group_cursor: &mut Cursor,
        gspan: Span,
    ) {
        let mut group_tt_collector =
            TtCollectorInnerReps::new(self.base.loop_ident, self.start, self.end);
        while !group_cursor.eof() {
            group_tt_collector.handle_cursor(group_cursor);
        }
        let mut group_token = Group::new(delim, group_tt_collector.consume());
        group_token.set_span(gspan);
        self.base.collected_tokens.push(group_token.into());
    }

    fn handle_group_cursor_inner(
        &mut self,
        delim: Delimiter,
        group_cursor: &mut Cursor,
        gspan: Span,
        current_index: usize,
    ) {
        let mut group_tt_collector =
            TtCollectorInnerReps::new(self.base.loop_ident, self.start, self.end);
        while !group_cursor.eof() {
            group_tt_collector.handle_inner_rep_cursor(group_cursor, current_index);
        }
        let mut group_token = Group::new(delim, group_tt_collector.consume());
        group_token.set_span(gspan);
        self.base.collected_tokens.push(group_token.into());
    }
}
