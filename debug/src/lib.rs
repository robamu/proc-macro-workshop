use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Field, GenericParam, Lit, Meta};

fn handle_field(field: &Field, field_formatters: &mut Vec<TokenStream>) -> syn::Result<()> {
    if let Some(fident) = &field.ident {
        let mut field_modifier = None;
        for attr in &field.attrs {
            match attr.parse_meta()? {
                Meta::NameValue(meta_nv) => {
                    if let Lit::Str(lit_str) = &meta_nv.lit {
                        field_modifier = Some(lit_str.value());
                    } else {
                        unimplemented!();
                    }
                }
                _ => {
                    unimplemented!();
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

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = &input.ident;
    let ident_as_str = struct_ident.to_string();

    let mut field_formatters = Vec::new();

    let mut impl_tt = None;
    for generic in &input.generics.params {
        let (impl_generics, type_generics, _) = input.generics.split_for_impl();
        let gen_ident = if let GenericParam::Type(ty) = generic {
            &ty.ident
        } else {
            return syn::Error::new(generic.span(), "Can only deal with type generics")
                .to_compile_error()
                .into();
        };
        impl_tt = Some(quote! {
            impl #impl_generics fmt::Debug for #struct_ident #type_generics
                where #gen_ident: fmt::Debug
        });
    }
    if impl_tt.is_none() {
        impl_tt = Some(quote! { impl fmt::Debug for #struct_ident });
    }
    let impl_tt = impl_tt.unwrap();
    match input.data {
        Data::Struct(s_data) => {
            for field in s_data.fields {
                match handle_field(&field, &mut field_formatters) {
                    Ok(_) => {}
                    Err(e) => return e.into_compile_error().into(),
                }
            }
        }
        _ => {
            unimplemented!();
        }
    }
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
