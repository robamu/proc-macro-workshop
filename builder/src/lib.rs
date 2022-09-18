use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::{HashMap, HashSet};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Attribute, Data, DeriveInput, Field, Fields,
    FieldsNamed, GenericArgument, Lit, Meta, NestedMeta, PathArguments, PathSegment, Type,
    TypePath,
};

#[allow(dead_code)]
fn dbg_derive_input(input: &DeriveInput) {
    dbg!("{:?}", input.clone());
}

#[derive(Default, Debug)]
struct VecFieldInfo {
    create_builder_for_vec_field: bool,
    builder_ident: Option<Ident>,
    wrapped_type: TokenStream,
}

#[derive(Default)]
struct OutputInfo {
    struct_field_names: Vec<Ident>,
    struct_field_definitions: Vec<TokenStream>,
    field_setters: Vec<TokenStream>,
    /// Hash map of field which are Vectors
    vec_fields: HashMap<Ident, VecFieldInfo>,
    opt_fields: HashSet<Ident>,
}

fn handle_named_struct_fields(fields: FieldsNamed, out_info: &mut OutputInfo) {
    for field in &fields.named {
        if let Some(ref field_ident) = field.ident {
            out_info.struct_field_names.push(field_ident.clone());
            if let Type::Path(tpath) = &field.ty {
                handle_named_field_type(field, field_ident, tpath, out_info);
            }
        }
    }
}

fn handle_named_field_type(
    field: &Field,
    field_ident: &Ident,
    tpath: &TypePath,
    out_info: &mut OutputInfo,
) {
    let mut is_opt_field = false;
    let mut is_vec_field = false;
    for attr in &field.attrs {
        process_field_attrs(field_ident, attr, out_info).expect("Processing field attr failed");
    }
    for (idx, path_seg) in tpath.path.segments.iter().enumerate() {
        if idx == 0 {
            if path_seg.ident == "Option" {
                is_opt_field = true;
                // Need to insert this somewhere else after wrapped type is known..
                out_info.opt_fields.insert(field_ident.clone());
            }
            if path_seg.ident == "Vec" {
                is_vec_field = true;
                if !out_info.vec_fields.contains_key(field_ident) {
                    out_info
                        .vec_fields
                        .insert(field_ident.clone(), VecFieldInfo::default());
                }
            }
        }

        let full_type_token =
            process_type_arguments(field_ident, path_seg, out_info, is_opt_field, is_vec_field);
        generate_field_setters(field_ident, full_type_token, out_info, is_vec_field);
    }
}

fn process_field_attrs(
    field_ident: &Ident,
    attr: &Attribute,
    out_info: &mut OutputInfo,
) -> syn::Result<()> {
    match attr.parse_meta()? {
        Meta::Path(_) => {}
        Meta::List(meta_list) => {
            let mut try_process_nested_meta = false;
            if let Some(seg) = meta_list.path.segments.first() {
                if seg.ident == "builder" {
                    try_process_nested_meta = true;
                }
            }
            if !try_process_nested_meta {
                return Ok(());
            }
            let mut each_attr = false;
            for nested in &meta_list.nested {
                match nested {
                    NestedMeta::Meta(Meta::NameValue(meta_name_value)) => {
                        if let Some(seg) = meta_name_value.path.segments.first() {
                            if seg.ident == "each" {
                                each_attr = true;
                            }
                        }
                        match &meta_name_value.lit {
                            Lit::Str(str) => {
                                if each_attr {
                                    let mut vec_info = VecFieldInfo::default();
                                    let ident: Ident = str.parse()?;
                                    vec_info.builder_ident = Some(ident);
                                    out_info.vec_fields.insert(field_ident.clone(), vec_info);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Meta::NameValue(_) => {}
    }
    Ok(())
}

fn process_type_arguments(
    field_ident: &Ident,
    path_seg: &PathSegment,
    out_info: &mut OutputInfo,
    is_opt_field: bool,
    is_vec_field: bool,
) -> Option<TokenStream> {
    let type_ident = &path_seg.ident;
    let mut full_type_token = None;
    match path_seg.arguments {
        PathArguments::None => {
            // Is that even possible? Just continue here..
            if is_opt_field || is_vec_field {
                panic!("No generic arguments for opt field or vec field");
            }
            out_info.struct_field_definitions.push(quote! {
                #field_ident: Option<#type_ident>
            });
            full_type_token = Some(type_ident.to_token_stream());
        }
        PathArguments::AngleBracketed(ref generics) => {
            let generics_ident = collect_generic_arguments(generics);
            if is_vec_field {
                if let Some(vec_info) = out_info.vec_fields.get_mut(&field_ident) {
                    vec_info.wrapped_type = quote! { #(#generics_ident),* };
                    if vec_info.builder_ident.is_some() {
                        vec_info.create_builder_for_vec_field = true;
                    }
                }
                out_info.struct_field_definitions.push(quote! {
                    #field_ident: #type_ident<#(#generics_ident),*>
                });
                full_type_token = Some(quote! { #type_ident<#(#generics_ident),*> });
            }
            if is_opt_field {
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
            }
            if !is_vec_field && !is_opt_field {
                out_info.struct_field_definitions.push(quote! {
                    #field_ident: Option<#type_ident<#(#generics_ident),*>>
                });
                full_type_token = Some(quote! { #type_ident<#(#generics_ident),*> });
            }
        }
        PathArguments::Parenthesized(_) => {}
    }
    full_type_token
}

fn generate_field_setters(
    field_ident: &Ident,
    full_type_token: Option<TokenStream>,
    out_info: &mut OutputInfo,
    is_vec_field: bool,
) {
    let mut gen_all_at_once_builder = true;
    if let Some(vec_info) = out_info.vec_fields.get_mut(&field_ident) {
        if vec_info.create_builder_for_vec_field {
            let vec_builder_ident = vec_info.builder_ident.as_ref().unwrap();
            let wrapped_type = &vec_info.wrapped_type;
            out_info.field_setters.push(quote! {
                fn #vec_builder_ident(&mut self, #vec_builder_ident: #wrapped_type) -> &mut Self {
                    self.#field_ident.push(#vec_builder_ident);
                    self
                }
            });
            if field_ident == vec_builder_ident {
                gen_all_at_once_builder = false;
            }
        }
    }
    if let Some(full_type) = full_type_token {
        if gen_all_at_once_builder {
            let init_val;
            if is_vec_field {
                init_val = quote! { #field_ident };
            } else {
                init_val = quote! { Some(#field_ident) };
            }
            out_info.field_setters.push(quote! {
                fn #field_ident(&mut self, #field_ident: #full_type) -> &mut Self {
                    self.#field_ident = #init_val;
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
    let mut checked_idents = Vec::new();

    while let Some(field_ident) = struct_field_iter.next() {
        let vec_field = out_info.vec_fields.contains_key(field_ident);
        let opt_field = out_info.opt_fields.contains(field_ident);
        if opt_field {
            field_assignments.push(quote! {
                #field_ident: self.#field_ident.to_owned()
            });
        } else if vec_field {
            field_assignments.push(quote! {
                #field_ident: self.#field_ident.to_owned()
            });
        } else {
            field_assignments.push(quote! {
                #field_ident: self.#field_ident.to_owned().unwrap()
            });
        }
        if !vec_field && !opt_field {
            checked_idents.push(field_ident);
        }
    }
    let mut check_all_fields_set = None;
    if !checked_idents.is_empty() {
        check_all_fields_set = Some(quote! {
            if #(self.#checked_idents.is_none())||* {
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

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let builder_name = format_ident!("{}Builder", input.ident);
    //dbg_derive_input(&input);
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
        vec_fields,
    } = &out_info;

    let mut field_init_list = Vec::new();
    for field_def in struct_field_names {
        if vec_fields.contains_key(field_def) {
            field_init_list.push(quote! { #field_def: Vec::new() });
        } else {
            field_init_list.push(quote! { #field_def: None });
        }
    }

    let output = quote! {
        use std::error::Error;

        pub struct #builder_name {
            #(#struct_field_definitions),*
        }

        impl #struct_name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#field_init_list),*
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
