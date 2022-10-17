use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{braced, parse_macro_input, Data, Field, Ident, Token, Type, Visibility};

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

fn bits_type_ident(bits: usize) -> TokenStream {
    match bits {
        0..=8 => quote! { u8 },
        9..=16 => quote! { u16 },
        17..=32 => quote! { u32 },
        33..=63 => quote! { u64 },
        _ => panic!("Invalid number of bits {}", bits),
    }
}

#[proc_macro]
pub fn make_bitwidth_markers(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output = TokenStream::new();
    for ref i in 1..MAX_BIT_WIDTH {
        let bits_type_ident = bits_type_ident(*i);
        let bitwidth_ident = format_ident!("B{}", i);
        output.extend(quote! {
            pub enum #bitwidth_ident {}

            impl Specifier for #bitwidth_ident {
                const BITS: usize = #i;
                type UTYPE = #bits_type_ident;

                fn from_u64(val: u64) -> Self::UTYPE {
                    val as Self::UTYPE
                }
            }
        })
    }
    output.extend(quote! {
        impl Specifier for bool {
            const BITS: usize = 1usize;
            type UTYPE = bool;

            fn from_u64(val: u64) -> Self::UTYPE {
                val == 1
            }
        }
    });
    output.into()
}

#[proc_macro_attribute]
pub fn bitfield(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let _ = args;
    let input = parse_macro_input!(input as StructInfo);
    let mut attribute_code_generator = BitfieldAttributeCodeGenerator::default();
    attribute_code_generator
        .gen_output(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(Default)]
struct BitfieldAttributeCodeGenerator {
    setters: TokenStream,
    getters: TokenStream,
    const_offsets: TokenStream,
    bit_attr_checks: TokenStream,
    path_vec: Vec<syn::Path>,
}

impl BitfieldAttributeCodeGenerator {
    fn gen_output(&mut self, input: StructInfo) -> syn::Result<TokenStream> {
        let out_ident = &input.ident;
        let out_vis = input.vis;
        let mut previous_const = None;
        let mut previous_specifier = None;
        for field in &input.fields {
            self.handle_field(field, &mut previous_const, &mut previous_specifier)?;
        }

        let path_vec = &self.path_vec;
        let full_len_bits = quote! { (#(<#path_vec as bitfield::Specifier>::BITS)+*) };
        let full_len_bytes = quote! {
           #full_len_bits / 8
        };
        let setters = &self.setters;
        let getters = &self.getters;
        let const_offsets = &self.const_offsets;
        let bit_attr_checks = &self.bit_attr_checks;
        Ok(quote! {
            #[repr(C)]
            #out_vis struct #out_ident {
                raw_data: [u8; #full_len_bytes]
            }


            impl #out_ident {
                #const_offsets
                #bit_attr_checks

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
                // which is vastly superior to what I hacked together
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
        })
    }

    fn handle_field(
        &mut self,
        field: &Field,
        previous_const: &mut Option<Ident>,
        previous_specifier: &mut Option<TokenStream>,
    ) -> syn::Result<()> {
        if let Some(ident) = &field.ident {
            let mut bit_from_bits_attr = None;
            let mut span_attr = None;
            if let Some(attr) = field.attrs.first() {
                if let syn::Meta::NameValue(meta_nv) = attr.parse_meta()? {
                    span_attr = Some(meta_nv.lit.span());
                    if let Some(path) = meta_nv.path.segments.first() {
                        if path.ident == "bits" {
                            if let syn::Lit::Int(bits_num) = meta_nv.lit {
                                bit_from_bits_attr = Some(bits_num.base10_parse::<usize>()?);
                            } else {
                                return Err(syn::Error::new(
                                    meta_nv.lit.span(),
                                    "Only integer literals are allowed for the bit specifier",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(
                                meta_nv.span(),
                                "Only the bits field attribute is supported",
                            ));
                        }
                    }
                }
            }

            let ident_upper_case = ident.clone().to_string().to_uppercase();
            let offset_ident = format_ident!("OFFSET_{}", ident_upper_case);
            let scoped_offset_ident = quote! { Self::#offset_ident };
            if let Type::Path(p) = &field.ty {
                let path = p.path.clone();
                let specifier_path = quote! { <#path as bitfield::Specifier> };

                if let Some(previous_const) = previous_const {
                    self.const_offsets.extend(quote! {
                        const #offset_ident: usize = Self::#previous_const + #previous_specifier::BITS;
                    });
                } else {
                    self.const_offsets.extend(quote! {
                        const #offset_ident: usize = 0;
                    });
                }
                *previous_const = Some(offset_ident);
                *previous_specifier = Some(specifier_path.clone());
                self.path_vec.push(path);
                let setter_name = format_ident!("set_{}", ident);
                let getter_name = format_ident!("get_{}", ident);
                self.setters.extend(quote! {
                    pub fn #setter_name(&mut self, val: #specifier_path::UTYPE) {
                        self.set(val as u64, #scoped_offset_ident, #specifier_path::BITS);
                    }
                });
                self.getters.extend(quote! {
                    pub fn #getter_name(&self) -> #specifier_path::UTYPE {
                        let val = self.get(#scoped_offset_ident, #specifier_path::BITS);
                        #specifier_path::from_u64(val)
                    }
                });
                if let Some(bit_attr) = bit_from_bits_attr {
                    let span = span_attr.expect("No span attribute found");
                    let check_ident = format_ident!("__CHECK_{}", ident_upper_case);
                    self.bit_attr_checks.extend(quote_spanned! {span=>
                        const #check_ident: [(); #bit_attr] = [(); #specifier_path::BITS];
                    })
                }
            }
        }
        Ok(())
    }
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    BitfieldDeriver::gen_derive(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

struct BitfieldDeriver {}

impl BitfieldDeriver {
    pub fn gen_derive(input: syn::DeriveInput) -> syn::Result<TokenStream> {
        if let Data::Enum(enumeration) = &input.data {
            let variants_count = enumeration.variants.iter().count();
            let mut divisible_by_two;
            let mut div_by_two = variants_count;
            let mut bits: usize = 0;
            loop {
                divisible_by_two = div_by_two % 2 == 0;
                if !divisible_by_two {
                    return Err(syn::Error::new(
                        input.attrs.first().span(),
                        "BitfieldSpecifier expected a number of variants which is a power of 2",
                    ));
                }
                div_by_two /= 2;
                bits += 1;
                if div_by_two == 1 {
                    break;
                }
            }
            let ident = input.ident;
            let mut variant_match_arms = TokenStream::new();
            let mut discriminant_checks = TokenStream::new();
            for variant in &enumeration.variants {
                let vident = &variant.ident;
                variant_match_arms.extend(quote! {
                    x if x == Self::#vident as u64 => Self::#vident,
                });
                let vspan = variant.span();
                discriminant_checks.extend(quote_spanned! {vspan=>
                    let _: bitfield::checks::DiscriminantCheck<
                        bitfield::checks::Assert<
                        { (Self::#vident as usize) < 2usize.pow(Self::BITS as u32)}
                    >>;
                });
            }
            let ident_str = ident.to_string();
            variant_match_arms.extend(quote! {
                _ => panic!("Received unexpected value {} for enum {}", val, #ident_str)
            });
            Ok(quote! {
                impl bitfield::Specifier for #ident {
                    const BITS: usize = #bits;
                    type UTYPE = Self;

                    fn from_u64(val: u64) -> Self::UTYPE {
                        #discriminant_checks
                        match val {
                            #variant_match_arms
                        }
                    }
                }
            })
        } else {
            Err(syn::Error::new(
                input.span(),
                "Macro can only be applied to enums",
            ))
        }
    }
}
