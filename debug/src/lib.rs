use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Data, DeriveInput, Field, GenericArgument,
    GenericParam, Lit, Meta, NestedMeta, PathArguments, PathSegment, Type,
};

enum TraitBoundCfg<'a> {
    Default,
    NoTraitBoundGeneration,
    CustomBound(syn::TypePath),
    BoundOverride(&'a TokenStream),
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = &input.ident;
    let ident_as_str = struct_ident.to_string();

    let mut field_formatters = Vec::new();
    let mut generic_idents = HashMap::new();

    let mut bound_override_on_struct = None;
    for attr in &input.attrs {
        match attr.parse_meta() {
            Ok(Meta::List(meta_list)) => {
                for meta in &meta_list.nested {
                    if let NestedMeta::Meta(Meta::NameValue(meta_nv)) = meta {
                        if let Some(attr_path_seg) = meta_nv.path.segments.first() {
                            if attr_path_seg.ident == "bound" {
                                if let Lit::Str(lit_str) = &meta_nv.lit {
                                    dbg!("Found lit string: {}", lit_str);
                                    let lit_as_str = lit_str.to_token_stream();
                                    bound_override_on_struct = Some(quote! { #lit_as_str });
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                return syn::Error::new(attr.span(), "Expected `debug(bound = \"...\"`)")
                    .into_compile_error()
                    .into()
            }
        }
    }
    for generic in &input.generics.params {
        let gen_ident = if let GenericParam::Type(ty) = generic {
            &ty.ident
        } else {
            return syn::Error::new(generic.span(), "Can only deal with type generics")
                .to_compile_error()
                .into();
        };
        if let Some(ref bound_override) = bound_override_on_struct {
            generic_idents.insert(
                gen_ident.clone(),
                TraitBoundCfg::BoundOverride(bound_override),
            );
        } else {
            generic_idents.insert(gen_ident.clone(), TraitBoundCfg::Default);
        }
    }

    match input.data {
        Data::Struct(s_data) => {
            for field in s_data.fields {
                match handle_field(&field, &mut field_formatters, &mut generic_idents) {
                    Ok(_) => {}
                    Err(e) => return e.into_compile_error().into(),
                }
            }
        }
        _ => {
            return syn::Error::new(input.span(), "Can only use on regular data structs")
                .into_compile_error()
                .into()
        }
    }
    let mut trait_bounds = Vec::new();
    if !generic_idents.is_empty() {
        for (ident, info) in &generic_idents {
            let mut push_custom_path = |custom_ty_path| {
                trait_bounds.push(quote! {
                   #custom_ty_path: core::fmt::Debug
                })
            };
            match info {
                TraitBoundCfg::Default => {
                    trait_bounds.push(quote! {
                        #ident: core::fmt::Debug
                    });
                }
                TraitBoundCfg::NoTraitBoundGeneration => {}
                TraitBoundCfg::BoundOverride(custom_ty_path) => {
                    push_custom_path(*custom_ty_path);
                }
                TraitBoundCfg::CustomBound(custom_ty_path) => {
                    push_custom_path(&custom_ty_path.to_token_stream());
                }
            }
        }
    }
    let impl_tt = if generic_idents.is_empty() {
        quote! { impl core::fmt::Debug for #struct_ident }
    } else {
        let (impl_generics, type_generics, _) = input.generics.split_for_impl();
        if !trait_bounds.is_empty() {
            quote! {
                impl #impl_generics core::fmt::Debug for #struct_ident #type_generics where
                    #(#trait_bounds),*
            }
        } else {
            quote! {
                impl #impl_generics core::fmt::Debug for #struct_ident #type_generics
            }
        }
    };

    let output = quote! {

        #impl_tt {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                f.debug_struct(#ident_as_str)
                    #(#field_formatters)*
                    .finish()
            }
        }
    };
    output.into()
}

fn handle_field(
    field: &Field,
    field_formatters: &mut Vec<TokenStream>,
    generic_idents: &mut HashMap<Ident, TraitBoundCfg>,
) -> syn::Result<()> {
    if let Some(fident) = &field.ident {
        let mut field_modifier = None;
        for attr in &field.attrs {
            match attr.parse_meta()? {
                Meta::NameValue(meta_nv) => {
                    if let Lit::Str(lit_str) = &meta_nv.lit {
                        field_modifier = Some(lit_str.value());
                    } else {
                        return Err(syn::Error::new(
                            meta_nv.span(),
                            "Expected literal string argument",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new(attr.span(), "Expected name value pair"));
                }
            }
        }
        if let Type::Path(ty_path) = &field.ty {
            for (idx, ty_seg) in ty_path.path.segments.iter().enumerate() {
                if idx == 0 {
                    if let PathArguments::AngleBracketed(gen_args) = &ty_seg.arguments {
                        handle_generic_args(ty_seg, gen_args, generic_idents);
                    }
                }
            }
        }
        let fident_str = fident.to_string();
        if let Some(modifier) = field_modifier {
            field_formatters.push(quote! {
                .field(#fident_str, &format_args!(#modifier, &self.#fident))
            })
        } else {
            field_formatters.push(quote! {
                .field(#fident_str, &self.#fident)
            })
        }
    }
    Ok(())
}

fn handle_generic_args(
    ty_seg: &PathSegment,
    gen_args: &AngleBracketedGenericArguments,
    generic_idents: &mut HashMap<Ident, TraitBoundCfg>,
) {
    for (idx, arg) in gen_args.args.iter().enumerate() {
        if idx == 0 {
            if let GenericArgument::Type(Type::Path(generic_path)) = arg {
                let mut detected_phantom_data = false;
                let mut first_generic = None;
                for (idx, gen_type) in generic_path.path.segments.iter().enumerate() {
                    if idx == 0 {
                        first_generic = Some(&gen_type.ident);
                        // Common special case: Do not emit trait bound T: Debug if T is only used
                        // inside PhantomData
                        if ty_seg.ident == "PhantomData"
                            && generic_idents.contains_key(&gen_type.ident)
                        {
                            *generic_idents.get_mut(&gen_type.ident).unwrap() =
                                TraitBoundCfg::NoTraitBoundGeneration;
                            detected_phantom_data = true;
                        }
                    } else if idx == 1 && !detected_phantom_data {
                        *generic_idents
                            .get_mut(first_generic.unwrap())
                            .expect("Generic not found in generic map") =
                            TraitBoundCfg::CustomBound(generic_path.clone());
                    }
                }
            }
        }
    }
}
