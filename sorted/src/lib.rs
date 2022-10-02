use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use std::collections::BTreeSet;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Arm, AttributeArgs, ExprMatch, Item, ItemFn, Meta, Pat};

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

fn process_input(_args: AttributeArgs, input: Item) -> syn::Result<TokenStream> {
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
    fn_parser.parse_wrapper(input)?;
    //dbg!("Input: {}", &input);
    Ok(input.to_token_stream())
}

#[derive(Default)]
struct FunctionSortedMatchParser {
    parse_meta_failure: Option<syn::Error>,
    match_not_sorted: Option<syn::Error>,
}

impl VisitMut for FunctionSortedMatchParser {
    fn visit_expr_match_mut(&mut self, i: &mut ExprMatch) {
        let mut removed_index = None;
        for (idx, attr) in i.attrs.iter().enumerate() {
            let meta = match attr.parse_meta() {
                Ok(meta) => meta,
                Err(e) => {
                    self.parse_meta_failure = Some(e);
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
    fn parse_wrapper(&mut self, i: &mut ItemFn) -> syn::Result<()> {
        self.visit_item_fn_mut(i);
        self.parse_meta_failure.to_owned().map_or(Ok(()), Err)?;
        self.match_not_sorted.to_owned().map_or(Ok(()), Err)
    }

    fn check_match_arms_sorted(&mut self, arms: &Vec<Arm>) {
        let mut match_arm_idents_set = BTreeSet::new();
        for arm in arms {
            if let Pat::TupleStruct(pat_ts) = &arm.pat {
                if let Some(path_seg) = pat_ts.path.segments.first() {
                    let ident_as_str = path_seg.ident.to_string();
                    if let Some(last_element) = match_arm_idents_set.iter().rev().last() {
                        if ident_as_str < *last_element {
                            self.match_not_sorted = Some(syn::Error::new(
                                path_seg.span(),
                                format!("{} should sort before {}", ident_as_str, *last_element),
                            ))
                        }
                    }
                    match_arm_idents_set.insert(ident_as_str);
                }
            }
        }
    }
}
