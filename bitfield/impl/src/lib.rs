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
        let mask_val = 2_usize.pow(i.clone() as u32) - 1;
        // ((Self::BITS - first_seg_width - last_seg_width) / 8) as u8
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
                const MASK: usize = #mask_val;
                type UTYPE = #type_ident;

                fn middle_segments(&self, first_seg_width: u8, last_seg_width: u8) -> u8 {
                    0
                }
            }
        })
    }
    output.into()
}

/*
This  generic setter approach is a bit overkill for many common cases, but should work for all
special cases.

0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
            1 0   0 0 1 0 1 1 1 0   1 1

Value: 100010111011
Offset: 6
Width: 12
First Byte Index: Offset / 8
First Seg Width: 8 - (Offset % 8) = 2
Last Byte Index: (Offset + Width) / 8
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

0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
                          1 0 1 0

Offset = 15
FirstByteIndex = Offset / 8  => 1
LastByteIndex = (Offset + Width) / 8 => 1
Shift = ( 2 * 8 ) - 1 - 15 = 0
SecondByte &= !(Mask << Shift)
SecondByte |= (Value & Mask) << Shift

0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
                      1 0 1 0
Offset = 10
FirstByteIndex = 1
LastByteIndex = 1
Shift = ((Index + 1) * 8 ) - (Offset + Width) = 2
SecondByte &= !(Mask << Shift)
SecondByte |= (Value & Mask) << Shift

0 0 0 0 0 0 0 0 | 0 0 0 0 0 0 0 0
          1 0 0   1 1

Offset: 5
FirstByteIndex = 0
SecondByteIndex = 1
First Seg Width: 8 - (Offset % 8) = 3
Second Seg Width: (Offset + Width) % 8 = 2

LastSeg = Value & SecondSegWidth
FirstSeg = (Value >> FirstSegWidth) & FirstSegWidth
FirstByte &= !(FirstSegWidthMask)
FirstByte |= FirstSeg

SecondByteShift = ((SecondByteIndex + 1) * 8) - (Offset + Width) = 6
SecondSeg = Value & SecondSegWidth
SecondByte &= !(SecondSegWidth << SecondByteShift)
SecondByte |= (SecondSeg << SecondByteShift)
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
            let offset_ident = format_ident!("OFFSET_{}", ident.to_string().to_uppercase());
            let scoped_offset_ident = quote! { Self::#offset_ident };
            if let Type::Path(p) = &field.ty {
                let path = p.path.clone();
                let specifier_path = quote! { <#path as Specifier> };
                //let specifier_path = quote! { #path };

                if let Some(previous_const) = preceeding_const {
                    const_offsets.extend(quote! {
                        const #offset_ident: usize = Self::#previous_const + #specifier_path::BITS;
                    });
                    preceeding_const = Some(offset_ident.clone());
                } else {
                    const_offsets.extend(quote! {
                        const #offset_ident: usize = #specifier_path::BITS;
                    });
                    preceeding_const = Some(offset_ident.clone());
                }
                path_vec.push(path);
                let setter_name = format_ident!("set_{}", ident);
                let getter_name = format_ident!("get_{}", ident);
                setters.extend(quote! {
                    pub fn #setter_name(&mut self, val: #specifier_path::UTYPE) {

                        let first_seg_width = #specifier_path::first_seg_width(#scoped_offset_ident);
                        let last_seg_width = #specifier_path::last_seg_width(#scoped_offset_ident);
                        //let segs = #specifier_path::middle_segments(first_seg_width, last_seg_width);
                        //let last_seg = val & #specifier_path::MASK as #specifier_path::UTYPE;
                        //self.#ident = 0;
                    }
                });
                getters.extend(quote! {
                    pub fn #getter_name(&self) -> #specifier_path::UTYPE {
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
