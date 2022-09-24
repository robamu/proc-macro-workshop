use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, Data, DeriveInput, Field, GenericArgument, GenericParam, Lit, Meta,
    PathArguments, Type,
};

#[derive(Default)]
struct GenericsInfo {
    no_trait_bound_generation: bool,
}

fn handle_field(
    field: &Field,
    field_formatters: &mut Vec<TokenStream>,
    generic_idents: &mut HashMap<Ident, GenericsInfo>,
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
        match &field.ty {
            Type::Path(ty_path) => {
                if let Some(ty_seg) = ty_path.path.segments.first() {
                    // Common special case: Do not emit trait bound T: Debug if T is only used
                    // inside PhantomData
                    if ty_seg.ident == "PhantomData" {
                        if let PathArguments::AngleBracketed(gen_args) = &ty_seg.arguments {
                            for arg in &gen_args.args {
                                if let GenericArgument::Type(Type::Path(generic_path)) = arg {
                                    if let Some(generic_type) = generic_path.path.segments.first() {
                                        if generic_idents.contains_key(&generic_type.ident) {
                                            generic_idents
                                                .get_mut(&generic_type.ident)
                                                .unwrap()
                                                .no_trait_bound_generation = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
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

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = &input.ident;
    let ident_as_str = struct_ident.to_string();

    let mut field_formatters = Vec::new();
    let mut generic_idents = HashMap::new();

    for generic in &input.generics.params {
        let gen_ident = if let GenericParam::Type(ty) = generic {
            &ty.ident
        } else {
            return syn::Error::new(generic.span(), "Can only deal with type generics")
                .to_compile_error()
                .into();
        };
        generic_idents.insert(gen_ident.clone(), GenericsInfo::default());
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
        for (ident, info) in generic_idents {
            if !info.no_trait_bound_generation {
                trait_bounds.push(quote! {
                   #ident: fmt::Debug
                });
            }
        }
    }
    let impl_tt = if trait_bounds.is_empty() {
        quote! { impl fmt::Debug for #struct_ident }
    } else {
        let (impl_generics, type_generics, _) = input.generics.split_for_impl();
        quote! {
            impl #impl_generics fmt::Debug for #struct_ident #type_generics where
                #(#trait_bounds),*
        }
    };

    let output = quote! {
        use core::fmt;

        #impl_tt {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                f.debug_struct(#ident_as_str)
                    #(#field_formatters)*
                    .finish()
            }
        }
    };
    output.into()
}
