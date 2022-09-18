extern crate core;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Data, DeriveInput, Fields, FieldsNamed,
    GenericArgument, PathArguments, Type, TypePath,
};

#[allow(dead_code)]
fn dbg_derive_input(input: &DeriveInput) {
    dbg!("{:?}", input.clone());
}

#[derive(Default)]
struct OutputInfo {
    struct_field_names: Vec<Ident>,
    struct_field_definitions: Vec<TokenStream>,
    field_setters: Vec<TokenStream>,
    // Key: Name of optional field. Value: Type wrapped in Option
    opt_fields: HashSet<Ident>,
}

fn handle_named_struct_fields(fields: FieldsNamed, out_info: &mut OutputInfo) {
    for field in fields.named {
        if let Some(ref field_ident) = field.ident {
            out_info.struct_field_names.push(field_ident.clone());
            if let Type::Path(p) = field.ty {
                handle_named_field_type(p, field_ident, out_info);
            }
        }
    }
}

fn handle_named_field_type(p: TypePath, field_ident: &Ident, out_info: &mut OutputInfo) {
    let mut is_optional_field = false;
    for (idx, path_seg) in p.path.segments.iter().enumerate() {
        let mut full_type_token = None;
        let type_ident = &path_seg.ident;
        if type_ident == "Option" && idx == 0 {
            is_optional_field = true;
            // Need to insert this somewhere else after wrapped type is known..
            out_info.opt_fields.insert(field_ident.clone());
        }
        match path_seg.arguments {
            PathArguments::None => {
                // Is that even possible? Just continue here..
                if is_optional_field {
                    continue;
                }
                out_info.struct_field_definitions.push(quote! {
                    #field_ident: Option<#type_ident>
                });
                full_type_token = Some(type_ident.to_token_stream());
            }
            PathArguments::AngleBracketed(ref generics) => {
                let generics_ident = collect_generic_arguments(generics);
                if is_optional_field {
                    // There is not explicit option wrapping necessary because the type identier
                    // will be an option.
                    out_info.struct_field_definitions.push(quote! {
                        #field_ident: #type_ident<#(#generics_ident),*>
                    });
                    // Do not include the type identifier used for the setter function,
                    // which is Option. If someone calls the setter function for an optional field,
                    // we still want the API to expect the actual type, not being wrapped inside an
                    // option.
                    full_type_token = Some(quote! { #(#generics_ident),* });
                } else {
                    out_info.struct_field_definitions.push(quote! {
                        #field_ident: Option<#type_ident<#(#generics_ident),*>>
                    });
                    full_type_token = Some(quote! { #type_ident<#(#generics_ident),*> });
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

/// Collect generic arguments of a type in a recursive fashion
fn collect_generic_arguments(generic_args: &AngleBracketedGenericArguments) -> Vec<TokenStream> {
    let mut generic_idents: Vec<TokenStream> = Vec::new();
    for arg in &generic_args.args {
        match arg {
            GenericArgument::Lifetime(_) => {}
            GenericArgument::Type(generic_ty) => {
                if let Type::Path(gen_p) = generic_ty {
                    for path_seg in &gen_p.path.segments {
                        let ident = &path_seg.ident;
                        match path_seg.arguments {
                            PathArguments::None => {
                                generic_idents.push(ident.to_token_stream());
                            }
                            PathArguments::AngleBracketed(ref generic_args) => {
                                let wrapped_generic_args = collect_generic_arguments(generic_args);
                                generic_idents.push(quote! { #(#wrapped_generic_args),* });
                            }
                            PathArguments::Parenthesized(_) => {}
                        }
                    }
                }
            }
            GenericArgument::Binding(_) => {}
            GenericArgument::Constraint(_) => {}
            GenericArgument::Const(_) => {}
        }
    }
    generic_idents
}

fn build_build_command(struct_name: &Ident, out_info: &OutputInfo) -> TokenStream {
    let struct_field_names = &out_info.struct_field_names;
    let mut field_assignments: Vec<TokenStream> = Vec::new();
    let mut struct_field_iter = struct_field_names.iter().peekable();
    let mut check_conditions = Vec::new();

    while let Some(field_ident) = struct_field_iter.next() {
        if out_info.opt_fields.contains(field_ident) {
            field_assignments.push(quote! {
                #field_ident: self.#field_ident.to_owned()
            });
            continue;
        }
        field_assignments.push(quote! {
            #field_ident: self.#field_ident.to_owned().unwrap()
        });
        if let Some(&next_ident) = struct_field_iter.peek() {
            if out_info.opt_fields.contains(next_ident) {
                check_conditions.push(quote! { self.#field_ident.is_none() });
            } else {
                check_conditions.push(quote! { self.#field_ident.is_none() || });
            }
        } else {
            check_conditions.push(quote! { self.#field_ident.is_none() });
        }
    }
    let mut check_all_fields_set = None;
    if !check_conditions.is_empty() {
        check_all_fields_set = Some(quote! {
            if #(#check_conditions)* {
                return Err(String::from("Please call all setter methods").into())
            }
        });
    }

    quote! {
        pub fn build(&mut self) -> Result<#struct_name, Box<dyn Error>> {
            #check_all_fields_set

            Ok(#struct_name {
                #(#field_assignments),*
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
    // dbg_derive_input(&input);
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

    let build_command = build_build_command(struct_name, &out_info);

    let OutputInfo {
        struct_field_names,
        struct_field_definitions,
        field_setters,
        opt_fields: _,
    } = &out_info;
    // dbg!("{}", opt_fields);

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
