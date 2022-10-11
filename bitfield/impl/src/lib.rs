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
        let writer_impl = match width {
            Width::U8 => {
                quote! {
                    let info = Self::start_end_info(offset);
                    let (first_byte_idx, second_byte_idx) = (info.start_idx, info.end_idx);
                    let shift = Self::lshift_from_end(second_byte_idx, offset);
                    let mask = mask_from_width(Self::BITS as u8);
                    // First two cases: 1 - 8 bit field in same byte
                    if info.end_on_byte_boundary {
                        raw[first_byte_idx] = (raw[first_byte_idx] & !(mask as u8)) | val;
                    } else if first_byte_idx == second_byte_idx {
                        raw[first_byte_idx] =
                            (raw[first_byte_idx] & !((mask as u8) << shift))
                            | (val << shift) as u8;
                    } else {
                        // Bitfield spans two bytes
                        let second_seg_width = Self::last_seg_width(offset);
                        let first_seg_mask = mask_from_width(Self::first_seg_width(offset));
                        let second_seg_mask = mask_from_width(second_seg_width);
                        let second_seg = val & second_seg_mask;
                        let first_seg = (val >> second_seg_width) & first_seg_mask;
                        raw[first_byte_idx] = (raw[first_byte_idx] & !first_seg_mask)
                            | (first_seg as u8);
                        raw[second_byte_idx] = (raw[second_byte_idx] & !(second_seg_mask << shift))
                            | ((second_seg << shift) as u8);
                    }
                }
            }
            Width::U16 | Width::U32 | Width::U64 => {
                quote! {
                    let info = Self::start_end_info(offset);
                    let (first_byte_idx, last_byte_idx) = (info.start_idx, info.end_idx);
                    let first_seg_width = Self::first_seg_width(offset);
                    let first_seg_mask = mask_from_width(first_seg_width);
                    let last_seg_width = Self::last_seg_width(offset);
                    let mut rem_shift = (last_byte_idx - first_byte_idx - 1) * 8 + last_seg_width as usize;
                    raw[first_byte_idx] = (raw[first_byte_idx] & !first_seg_mask)
                        | ((val >> rem_shift) as u8 & first_seg_mask);
                    for i in first_byte_idx + 1..last_byte_idx {
                        rem_shift -= 8;
                        raw[i] = (val >> rem_shift) as u8 & 0xff;
                    }
                    if !info.end_on_byte_boundary {
                        let shift = Self::lshift_from_end(last_byte_idx, offset);
                        let last_seg_mask = mask_from_width(last_seg_width);
                        raw[last_byte_idx] = (raw[last_byte_idx] & !(last_seg_mask << shift))
                            | ((val & last_seg_mask as Self::UTYPE) << shift) as u8
                    }
                }
            }
        };
        let reader_impl = match width {
            Width::U8 => {
                quote! {
                    let info = Self::start_end_info(offset);
                    let (first_byte_idx, second_byte_idx) = (info.start_idx, info.end_idx);
                    let mask = mask_from_width(Self::BITS as u8);
                    if info.end_on_byte_boundary {
                        raw[first_byte_idx] & mask as u8
                    } else {
                        let shift = Self::lshift_from_end(second_byte_idx, offset);
                        if first_byte_idx == second_byte_idx {
                            (raw[first_byte_idx] >> shift) & mask as u8
                        } else {
                            let first_seg_mask = mask_from_width(Self::first_seg_width(offset));
                            let second_seg_width = Self::last_seg_width(offset);
                            let second_seg_mask = mask_from_width(second_seg_width);
                            ((raw[first_byte_idx] & first_seg_mask) << second_seg_width) |
                                ((raw[second_byte_idx] >> shift) & second_seg_mask)
                        }
                    }
                }
            }
            Width::U16 | Width::U32 | Width::U64 => {
                quote! {
                    let info = Self::start_end_info(offset);
                    let (first_byte_idx, end_byte_idx) = (info.start_idx, info.end_idx);
                    let first_seg_mask = mask_from_width(Self::first_seg_width(offset));
                    let mut val = (raw[first_byte_idx] & first_seg_mask) as Self::UTYPE;
                    for i in first_byte_idx + 1..end_byte_idx {
                        val = (val << 8) | raw[i] as Self::UTYPE;
                    }
                    if info.end_on_byte_boundary {
                        val
                    } else {
                        let shift = Self::lshift_from_end(end_byte_idx, offset);
                        let last_seg_width = Self::last_seg_width(offset);
                        let last_seg_mask = mask_from_width(last_seg_width) as Self::UTYPE;
                        val = (val << last_seg_width)
                            | ((raw[end_byte_idx] >> shift) as Self::UTYPE & last_seg_mask);
                        val
                    }
                }
            }
        };
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
                type UTYPE = #type_ident;

                fn write_to_bytes(val: Self::UTYPE, offset: usize, raw: &mut [u8]) {
                    #writer_impl
                }
                fn read_from_bytes(offset: usize, raw: &[u8]) -> Self::UTYPE {
                    #reader_impl
                }
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
                        #specifier_path::write_to_bytes(val, #scoped_offset_ident, self.raw_data.as_mut());
                    }
                });
                getters.extend(quote! {
                    pub fn #getter_name(&self) -> #specifier_path::UTYPE {
                        #specifier_path::read_from_bytes(#scoped_offset_ident, self.raw_data.as_ref())
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
pub fn derive(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
   proc_macro::TokenStream::new().into()
}
