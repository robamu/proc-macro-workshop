extern crate core;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
use syn::{
    parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, GenericArgument, PathArguments,
    Type, TypePath,
};

#[allow(dead_code)]
fn dbg_derive_input(input: &DeriveInput) {
    dbg!("{:?}", input.clone());
}

#[derive(Default)]
struct OutputInfo {
    struct_field_names: Vec<TokenStream>,
    struct_field_definitions: Vec<TokenStream>,
    field_setters: Vec<TokenStream>,
    opt_fields: HashSet<Ident>,
}

fn handle_named_struct_fields(fields: FieldsNamed, out_info: &mut OutputInfo) {
    for field in fields.named {
        let field_ident = field.ident.unwrap();
        out_info.struct_field_names.push(quote! { #field_ident });
        if let Type::Path(p) = field.ty {
            handle_named_field_path_type(p, &field_ident, out_info);
        }
    }
}

fn handle_named_field_path_type(p: TypePath, field_ident: &Ident, out_info: &mut OutputInfo) {
    let mut type_ident;
    for path_seg in p.path.segments {
        let mut full_type_token = None;
        type_ident = path_seg.ident;
        match path_seg.arguments {
            PathArguments::None => {
                out_info.struct_field_definitions.push(quote! {
                    #field_ident: Option<#type_ident>
                });
                full_type_token = Some(type_ident.to_token_stream());
            }
            PathArguments::AngleBracketed(generics) => {
                for arg in generics.args {
                    match arg {
                        GenericArgument::Lifetime(_) => {}
                        GenericArgument::Type(generic_ty) => {
                            if let Type::Path(gen_p) = generic_ty {
                                let mut generic_idents = Vec::new();
                                for generic_segment in gen_p.path.segments {
                                    generic_idents.push(generic_segment.ident);
                                }
                                out_info.struct_field_definitions.push(quote! {
                                    #field_ident: Option<#type_ident<#(#generic_idents),*>>
                                });
                                full_type_token =
                                    Some(quote! { #type_ident<#(#generic_idents),*> });
                            }
                        }
                        GenericArgument::Binding(_) => {}
                        GenericArgument::Constraint(_) => {}
                        GenericArgument::Const(_) => {}
                    }
                }
            }
            PathArguments::Parenthesized(_) => {}
        }
        if let Some(full_type) = full_type_token {
            out_info.field_setters.push(quote! {
                fn #field_ident(&mut self, #field_ident: #full_type) -> &mut Self {
                    self.#field_ident = Some(#field_ident);
                    self
                }
            });
        }
    }
}

fn build_build_command(struct_name: &Ident, struct_field_names: &Vec<TokenStream>) -> TokenStream {
    let mut struct_field_iter = struct_field_names.iter().peekable();
    let mut check_vec = Vec::new();

    while let Some(field_ident) = struct_field_iter.next() {
        if struct_field_iter.peek().is_none() {
            check_vec.push(quote! { self.#field_ident.is_none() });
        } else {
            check_vec.push(quote! { self.#field_ident.is_none() || });
        }
    }
    quote! {
        pub fn build(&mut self) -> Result<#struct_name, Box<dyn Error>> {
            if #(#check_vec)* {
                return Err(String::from("Please call all setter methods").into())
            }
            Ok(#struct_name {
                #(#struct_field_names: self.#struct_field_names.to_owned().unwrap()),*
            })
        }
    }
}

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let builder_name = format_ident!("{}Builder", input.ident);
    let mut out_info = OutputInfo::default();

    match input.data {
        Data::Struct(structure) => match structure.fields {
            Fields::Named(named_fields) => {
                handle_named_struct_fields(named_fields, &mut out_info);
            }
            Fields::Unnamed(_) => {}
            Fields::Unit => {}
        },
        Data::Enum(_) => {}
        Data::Union(_) => {}
    }

    let build_command = build_build_command(struct_name, &out_info.struct_field_names);

    let OutputInfo {
        struct_field_names,
        struct_field_definitions,
        field_setters,
        opt_fields: _,
    } = &out_info;

    let output = quote! {
        use std::error::Error;

        pub struct #builder_name {
            #(#struct_field_definitions),*
        }

        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#struct_field_names: None),*
                }
            }
        }

        impl #builder_name {
            #(#field_setters)*

            #build_command
        }
    };
    proc_macro::TokenStream::from(output)
}
