use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Meta};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = &input.ident;
    let ident_as_str = struct_ident.to_string();

    let mut field_idents = Vec::new();

    for attr in input.attrs {
        match attr.parse_meta()? {
            Meta::Path(_) => {}
            Meta::List(_) => {}
            Meta::NameValue(_) => {}
        }
    }
    match input.data {
        Data::Struct(s_data) => {
            for field in s_data.fields {
                if let Some(fident) = field.ident {
                    field_idents.push(fident)
                }
            }
        }
        _ => {
            unimplemented!();
        }
    }
    let mut field_formatters = Vec::new();
    for fident in field_idents {
        let fident_str = fident.to_string();
        field_formatters.push(quote! {
            .field(#fident_str, &self.#fident)
        })
    }
    let output = quote! {
        use core::fmt;

        impl fmt::Debug for #struct_ident {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
                f.debug_struct(#ident_as_str)
                    #(#field_formatters)*
                    .finish()
            }
        }
    };
    output.into()
}
