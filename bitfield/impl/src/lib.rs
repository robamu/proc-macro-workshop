use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Field, Ident, Token, Type, Visibility};

const MAX_BIT_WIDTH: usize = 64;

#[derive(Debug)]
struct StructInfo {
    vis: Option<Visibility>,
    ident: Ident,
    fields: Vec<Field>,
}

impl Parse for StructInfo {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse::<Visibility>().ok();
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        let fields;
        braced!(fields in input);
        let fields = fields.parse_terminated::<Field, Token![,]>(Field::parse_named)?;
        Ok(Self {
            vis,
            ident,
            fields: fields.into_iter().collect(),
        })
    }
}

#[proc_macro]
pub fn make_bitwidth_markers(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output = TokenStream::new();
    for i in 0..MAX_BIT_WIDTH {
        let bitwidth_ident = format_ident!("B{}", i);
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
            }
        })
    }
    output.into()
}

#[proc_macro_attribute]
pub fn bitfield(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as StructInfo);
    let out_ident = &input.ident;
    let out_vis = input.vis;
    let mut path_vec = Vec::new();
    for field in &input.fields {
        if let Type::Path(p) = &field.ty {
            path_vec.push(p.path.clone());
        }
    }
    let compile_time_bits_calculation = quote! {
        (#(<#path_vec as Specifier>::BITS)+*) / 8
    };

    let output = quote! {
        #out_vis struct #out_ident {
            raw_data: [u8; #compile_time_bits_calculation]
        }
    };
    output.into()
}
