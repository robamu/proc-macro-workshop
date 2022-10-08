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
    for ref i in 0..MAX_BIT_WIDTH {
        let div = i / 8;
        let rem = i % 8;
        let value_type = match div {
            0 => quote! { u8 },
            1 => {
                if rem == 0 {
                    quote! { u8 }
                } else {
                    quote! { u16 }
                }
            }
            2 => {
                if rem == 0 {
                    quote! { u16 }
                } else {
                    quote! { u32 }
                }
            }
            3 => quote! { u32 },
            4 => {
                if rem == 0 {
                    quote! { u32 }
                } else {
                    quote! { u64 }
                }
            }
            _ => {
                quote! { u64 }
            }
        };
        let bitwidth_ident = format_ident!("B{}", i);
        let mask_val = 2_usize.pow(i.clone() as u32) - 1;
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
                const MASK: usize = #mask_val;
                type UTYPE = #value_type;
            }
        })
    }
    output.into()
}

/*
Simple generic setter approach:

0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
            1 0   0 0 1 0 1 1 1 0   1 1

Value: 100010111011
Offset: 6
Width: 12
First Seg Width: 8 - (Offset % 8) = 2
Last Seg Width: (Offset + Width) % 8 = 2
Second Seg Width (Only 8 left) = 8

Last Seg: Value & 0b11
Second Seg: (Value >> LastSegWidth) & 0xff
First Seg: (Value >> LastSegWidth + Segs * 8) & 0b11

First Byte &= ~FirstSegWidth
FirstByte |= First Seg

Second Byte &= ~0xff
SecondByte |= Second Seg

ShiftToFront: 8 - Width

ThirdByte &= ~ (LastSegWidth << ShiftToFront(Width))
ThirdByte |= (LastSeg << ShiftToFront(Width))
 */
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
    let mut const_offsets = TokenStream::new();
    let mut setters = TokenStream::new();
    let mut getters = TokenStream::new();
    let mut preceeding_const = None;
    for field in &input.fields {
        if let Some(ident) = &field.ident {
            let ident_upper_case = format_ident!("OFFSET_{}", ident.to_string().to_uppercase());
            if let Type::Path(p) = &field.ty {
                let path = p.path.clone();
                let fully_qualified_path = quote! { <#path as Specifier> };
                if let Some(previous_const) = preceeding_const {
                    const_offsets.extend(quote! {
                        const #ident_upper_case: usize = Self::#previous_const + #fully_qualified_path::BITS;
                    });
                    preceeding_const = Some(ident_upper_case);
                } else {
                    const_offsets.extend(quote! {
                        const #ident_upper_case: usize = #fully_qualified_path::BITS;
                    });
                    preceeding_const = Some(ident_upper_case);
                }
                path_vec.push(path);
                let setter_name = format_ident!("set_{}", ident);
                let getter_name = format_ident!("get_{}", ident);
                setters.extend(quote! {
                    pub fn #setter_name(&mut self, val: #fully_qualified_path::UTYPE) {}
                });
                getters.extend(quote! {
                    pub fn #getter_name(&self) -> #fully_qualified_path::UTYPE {
                        0
                    }
                })
            }
        }
    }

    let compile_time_len = quote! {
       (#(<#path_vec as Specifier>::BITS)+*) / 8
    };
    let output = quote! {
        #out_vis struct #out_ident {
            raw_data: [u8; #compile_time_len]
        }

        impl #out_ident {
            pub fn new() -> Self {
                Self {
                    raw_data: [0; #compile_time_len]
                }
            }
            #const_offsets
            #setters
            #getters
        }
    };
    output.into()
}
