use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{braced, parse_macro_input, Field, Ident, Token, Type, Visibility, Data};

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

enum Width {
    U8,
    U16,
    U32,
    U64,
}

#[proc_macro]
pub fn make_bitwidth_markers(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output = TokenStream::new();
    for ref i in 0..MAX_BIT_WIDTH {
        let div = i / 8;
        let rem = i % 8;
        let width = match div {
            0 => Width::U8,
            1 => {
                if rem == 0 {
                    Width::U8
                } else {
                    Width::U16
                }
            }
            2 => {
                if rem == 0 {
                    Width::U16
                } else {
                    Width::U32
                }
            }
            3 => Width::U32,
            4 => {
                if rem == 0 {
                    Width::U32
                } else {
                    Width::U64
                }
            }
            _ => Width::U64,
        };
        let type_ident = match width {
            Width::U8 => quote! { u8 },
            Width::U16 => quote! { u16 },
            Width::U32 => quote! { u32 },
            Width::U64 => quote! { u64 },
        };
        let bitwidth_ident = format_ident!("B{}", i);
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
                type UTYPE = #type_ident;
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
    let mut const_offsets = TokenStream::new();
    let mut setters = TokenStream::new();
    let mut getters = TokenStream::new();
    let mut previous_const = None;
    let mut previous_specifier = None;
    for field in &input.fields {
        if let Some(ident) = &field.ident {
            let offset_ident = format_ident!("OFFSET_{}", ident.to_string().to_uppercase());
            let scoped_offset_ident = quote! { Self::#offset_ident };
            if let Type::Path(p) = &field.ty {
                let path = p.path.clone();
                let specifier_path = quote! { <#path as bitfield::Specifier> };

                if let Some(previous_const) = previous_const {
                    const_offsets.extend(quote! {
                        const #offset_ident: usize = Self::#previous_const + #previous_specifier::BITS;
                    });
                } else {
                    const_offsets.extend(quote! {
                        const #offset_ident: usize = 0;
                    });
                }
                previous_const = Some(offset_ident.clone());
                previous_specifier = Some(specifier_path.clone());
                path_vec.push(path);
                let setter_name = format_ident!("set_{}", ident);
                let getter_name = format_ident!("get_{}", ident);
                setters.extend(quote! {
                    pub fn #setter_name(&mut self, val: #specifier_path::UTYPE) {
                        self.set(val as u64, #scoped_offset_ident, #specifier_path::BITS);
                    }
                });
                getters.extend(quote! {
                    pub fn #getter_name(&self) -> #specifier_path::UTYPE {
                        self.get(#scoped_offset_ident, #specifier_path::BITS) as #specifier_path::UTYPE
                    }
                })
            }
        }
    }

    let full_len_bits = quote! { (#(<#path_vec as Specifier>::BITS)+*) };
    let full_len_bytes = quote! {
       #full_len_bits / 8
    };
    let output = quote! {
        #[repr(C)]
        #out_vis struct #out_ident {
            raw_data: [u8; #full_len_bytes]
        }

        impl #out_ident {
            #const_offsets

            const FULL_LEN_MOD_EIGHT: usize = #full_len_bits % 8;

            pub fn new() -> Self {
                bitfield::checks::width_check::<
                    <bitfield::checks::NumDummy<{ Self::FULL_LEN_MOD_EIGHT }> as bitfield::checks::NumToGeneric>
                    ::GENERIC
                >();
                Self {
                    raw_data: [0; #full_len_bytes]
                }
            }

            // These two functions were taken from the reference implementation,
            // which is vastly superior to what I hacked together.
            // https://github.com/dtolnay/proc-macro-workshop/issues/55
            pub fn set(&mut self, val: u64, offset: usize, width: usize) {
                for i in 0..width {
                    let mask = 1 << i;
                    let val_bit_is_set = val & mask == mask;
                    let offset = i + offset;
                    let byte_index = offset / 8;
                    let bit_offset = offset % 8;
                    let byte = &mut self.raw_data[byte_index];
                    let mask = 1 << bit_offset;
                    if val_bit_is_set {
                        *byte |= mask;
                    } else {
                        *byte &= !mask;
                    }

                }
            }
            pub fn get(&self, offset: usize, width: usize) -> u64 {
                let mut val = 0;
                for i in 0..width {
                    let offset = i + offset;
                    let byte_index = offset / 8;
                    let bit_offset = offset % 8;
                    let byte = self.raw_data[byte_index];
                    let mask = 1 << bit_offset;
                    if byte & mask == mask {
                        val |= 1 << i;
                    }
                }
                val
            }
            pub fn raw_data(&self) -> &[u8] {
                self.raw_data.as_ref()
            }

            #setters
            #getters
        }
    };
    output.into()
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    dbg!("{}", &input);
    if let Data::Enum(enumeration) = input.data {
    } else {
        // TODO: generate compiler error
    }
    let output = TokenStream::new();
    output.into()
}
