use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::BTreeSet;
use std::iter::Peekable;
use syn::punctuated::Iter;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Arm, AttributeArgs, ExprMatch, Item, ItemFn, Meta, Pat, PathSegment};

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let input_item = parse_macro_input!(input as Item);
    let mut output_ts = input_item.to_token_stream();
    if let Err(e) = process_input(args, &input_item) {
        output_ts.extend(e.into_compile_error())
    }
    output_ts.into()
}

fn process_input(_args: AttributeArgs, input: &Item) -> syn::Result<TokenStream> {
    let mut variants_set = BTreeSet::new();
    match &input {
        Item::Enum(e) => {
            for variant in &e.variants {
                let variant_string = variant.ident.to_string();
                if let Some(last_element) = variants_set.iter().rev().last() {
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

#[proc_macro_attribute]
pub fn check(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = parse_macro_input!(args as AttributeArgs);
    let mut input = parse_macro_input!(input as ItemFn);
    check_and_replace_matches_with_sort_attr(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn check_and_replace_matches_with_sort_attr(input: &mut ItemFn) -> syn::Result<TokenStream> {
    let mut fn_parser = FunctionSortedMatchParser::default();
    fn_parser.visit_item_fn_mut(input);
    let mut output_ts = input.to_token_stream();
    // Add syn errors converted to compile errors to the output.
    output_ts.extend(TokenStream::from_iter(
        fn_parser
            .syn_errors
            .into_iter()
            .map(|e| e.into_compile_error()),
    ));
    Ok(output_ts)
}

#[derive(Default)]
struct FunctionSortedMatchParser {
    syn_errors: Vec<syn::Error>,
}

impl VisitMut for FunctionSortedMatchParser {
    fn visit_expr_match_mut(&mut self, i: &mut ExprMatch) {
        let mut removed_index = None;
        for (idx, attr) in i.attrs.iter().enumerate() {
            let meta = match attr.parse_meta() {
                Ok(meta) => meta,
                Err(e) => {
                    self.syn_errors.push(e);
                    return;
                }
            };
            if let Meta::Path(path) = meta {
                if let Some(path_seg) = path.segments.first() {
                    if path_seg.ident == "sorted" {
                        self.check_match_arms_sorted(&i.arms);
                        removed_index = Some(idx);
                    }
                }
            }
        }
        if let Some(index_to_remove) = removed_index {
            i.attrs.remove(index_to_remove);
        }
    }
}

impl FunctionSortedMatchParser {
    fn check_next_arm_ident(
        &mut self,
        next_ident_as_str: String,
        set: &mut BTreeSet<String>,
        error_tokens: TokenStream,
    ) {
        if !next_ident_as_str.is_empty() {
            if let Some(last_element) = set.iter().rev().last() {
                if &next_ident_as_str < last_element {
                    self.syn_errors.push(syn::Error::new_spanned(
                        error_tokens,
                        format!("{} should sort before {}", next_ident_as_str, last_element),
                    ))
                }
            }
            set.insert(next_ident_as_str);
        }
    }
    fn check_match_arms_sorted(&mut self, arms: &Vec<Arm>) {
        let mut match_arm_idents_set = BTreeSet::new();
        let full_path_from_segments = |mut iter: Peekable<Iter<PathSegment>>| {
            let mut full_path_str = String::new();
            while let Some(pseg) = iter.next() {
                full_path_str += &pseg.ident.to_string();
                if iter.peek().is_some() {
                    full_path_str += "::";
                }
            }
            full_path_str
        };
        for arm in arms {
            match &arm.pat {
                Pat::Struct(s) => {
                    let full_path_str = full_path_from_segments(s.path.segments.iter().peekable());
                    self.check_next_arm_ident(
                        full_path_str,
                        &mut match_arm_idents_set,
                        s.path.to_token_stream(),
                    );
                }
                Pat::TupleStruct(pat_ts) => {
                    let full_path_str =
                        full_path_from_segments(pat_ts.path.segments.iter().peekable());
                    self.check_next_arm_ident(
                        full_path_str,
                        &mut match_arm_idents_set,
                        pat_ts.path.to_token_stream(),
                    );
                }
                Pat::Path(p) => {
                    let full_path_str = full_path_from_segments(p.path.segments.iter().peekable());
                    self.check_next_arm_ident(
                        full_path_str,
                        &mut match_arm_idents_set,
                        p.path.to_token_stream(),
                    );
                }
                _ => (),
            }
        }
    }
}
