// crates/smithy/lib.rs

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use smithy_core::{SmithyConstraint, SmithyField};
use syn::{
    parse_macro_input, Attribute, DeriveInput, Fields, Lit, Meta, PathArguments, Type,
};

/// Derive macro for generating Smithy models from Rust structs
#[proc_macro_derive(SmithyModel, attributes(smithy))]
pub fn derive_smithy_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_smithy_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_smithy_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let namespace = extract_namespace(&input.attrs)?;

    let fields = match &input.data {
        syn::Data::Struct(data_struct) => extract_fields(&data_struct.fields),
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "SmithyModel can only be derived for structs",
            ))
        }
    }?;

    let struct_doc = extract_documentation(&input.attrs);
    let struct_doc_expr = struct_doc
        .as_ref()
        .map(|doc| quote! { Some(#doc.to_string()) })
        .unwrap_or(quote! { None });

    let field_implementations = fields
        .iter()
        .map(|field| {
            let field_name = &field.name;
            let smithy_type = &field.smithy_type;
            let field_doc = field
                .documentation
                .as_ref()
                .map(|doc| quote! { Some(#doc.to_string()) })
                .unwrap_or(quote! { None });

            // Automatically add Required trait for non-optional fields
            let mut all_constraints = field.constraints.clone();
            if !field.optional && !all_constraints.iter().any(|c| matches!(c, SmithyConstraint::Required)) {
                all_constraints.push(SmithyConstraint::Required);
            }

            // Handle traits properly to avoid empty vector with trailing comma
            let traits = if all_constraints.is_empty() {
                quote! { vec![] }
            } else {
                let trait_tokens = all_constraints
                    .iter()
                    .map(|constraint| match constraint {
                        SmithyConstraint::Pattern(pattern) => quote! {
                            smithy_core::SmithyTrait::Pattern { pattern: #pattern.to_string() }
                        },
                        SmithyConstraint::Range(min, max) => {
                            let min_expr = min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                            let max_expr = max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                            quote! {
                                smithy_core::SmithyTrait::Range {
                                    min: #min_expr,
                                    max: #max_expr
                                }
                            }
                        },
                        SmithyConstraint::Length(min, max) => {
                            let min_expr = min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                            let max_expr = max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                            quote! {
                                smithy_core::SmithyTrait::Length { 
                                    min: #min_expr, 
                                    max: #max_expr 
                                }
                            }
                        },
                        SmithyConstraint::Required => quote! {
                            smithy_core::SmithyTrait::Required
                        },
                    })
                    .collect::<Vec<_>>();

                quote! { vec![#(#trait_tokens),*] }
            };

            quote! {
                members.insert(#field_name.to_string(), smithy_core::SmithyMember {
                    target: #smithy_type.to_string(),
                    documentation: #field_doc,
                    traits: #traits
                });
            }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        impl smithy_core::SmithyModelGenerator for #name {
            fn generate_smithy_model() -> smithy_core::SmithyModel {
                let mut shapes = std::collections::HashMap::new();
                let mut members = std::collections::HashMap::new();

                #(#field_implementations)*

                let shape = smithy_core::SmithyShape::Structure {
                    members,
                    documentation: #struct_doc_expr,
                    traits: vec![]
                };

                shapes.insert(stringify!(#name).to_string(), shape);

                smithy_core::SmithyModel {
                    namespace: #namespace.to_string(),
                    shapes
                }
            }
        }
    };

    Ok(expanded)
}

fn extract_namespace(attrs: &[Attribute]) -> syn::Result<String> {
    for attr in attrs {
        if attr.path().is_ident("smithy") {
            let mut namespace = None;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("namespace") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                namespace = Some(lit_str.value());
                            }
                        }
                    }
                }
                Ok(())
            })?; // Propagate parsing errors
            
            if let Some(ns) = namespace {
                return Ok(ns);
            }
        }
    }
    Ok("com.hyperswitch.default".to_string())
}

fn extract_fields(fields: &Fields) -> syn::Result<Vec<SmithyField>> {
    let mut smithy_fields = Vec::new();

    match fields {
        Fields::Named(fields_named) => {
            for field in &fields_named.named {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let smithy_type = rust_type_to_smithy_type(&field.ty)?;
                let constraints = extract_constraints_from_attrs(&field.attrs)?;
                let documentation = extract_documentation(&field.attrs);
                let optional = is_optional_type(&field.ty);

                smithy_fields.push(SmithyField {
                    name: field_name,
                    smithy_type,
                    constraints,
                    documentation,
                    optional,
                });
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                fields,
                "Only named fields are supported",
            ))
        }
    }

    Ok(smithy_fields)
}

fn rust_type_to_smithy_type(ty: &Type) -> syn::Result<String> {
    fn unwrap_known_wrapper(ty: &Type) -> &Type {
        if let Type::Path(type_path) = ty {
            if let Some(last_segment) = type_path.path.segments.last() {
                let ident_str = last_segment.ident.to_string();
                if (ident_str == "Secret" || ident_str == "Box") && matches!(&last_segment.arguments, PathArguments::AngleBracketed(_)) {
                    if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return unwrap_known_wrapper(inner);
                        }
                    }
                }
            }
        }
        ty
    }

    let ty = unwrap_known_wrapper(ty); // <-- unwrapping Secret, Box, etc.

    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            if let Some(last_segment) = path.segments.last() {
                let type_name = last_segment.ident.to_string();

                match type_name.as_str() {
                    "String" | "str" => Ok("smithy.api#String".to_string()),
                    "i8" | "i16" | "i32" | "u8" | "u16" | "u32" => Ok("smithy.api#Integer".to_string()),
                    "i64" | "u64" | "isize" | "usize" => Ok("smithy.api#Long".to_string()),
                    "f32" => Ok("smithy.api#Float".to_string()),
                    "f64" => Ok("smithy.api#Double".to_string()),
                    "bool" => Ok("smithy.api#Boolean".to_string()),
                    "Amount" | "MinorUnit" => Ok("smithy.api#Long".to_string()),
                    "Vec" => {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                let inner_smithy_type = rust_type_to_smithy_type(inner_type)?;
                                Ok(format!("smithy.api#List<{}>", inner_smithy_type))
                            } else {
                                Err(syn::Error::new_spanned(
                                    ty,
                                    "Vec must have a type parameter",
                                ))
                            }
                        } else {
                            Err(syn::Error::new_spanned(ty, "Vec must have type parameters"))
                        }
                    }
                    "HashMap" | "BTreeMap" => {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            if args.args.len() >= 2 {
                                if let (
                                    Some(syn::GenericArgument::Type(key_type)),
                                    Some(syn::GenericArgument::Type(value_type)),
                                ) = (args.args.first(), args.args.get(1)) {
                                    let key_smithy_type = rust_type_to_smithy_type(key_type)?;
                                    let value_smithy_type = rust_type_to_smithy_type(value_type)?;
                                    Ok(format!("smithy.api#Map<{}, {}>", key_smithy_type, value_smithy_type))
                                } else {
                                    Err(syn::Error::new_spanned(
                                        ty,
                                        "Map types must have key and value type parameters",
                                    ))
                                }
                            } else {
                                Err(syn::Error::new_spanned(
                                    ty,
                                    "Map types must have key and value type parameters",
                                ))
                            }
                        } else {
                            Err(syn::Error::new_spanned(ty, "Map types must have type parameters"))
                        }
                    }
                    "Option" => {
                        if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                                rust_type_to_smithy_type(inner_type)
                            } else {
                                Err(syn::Error::new_spanned(
                                    ty,
                                    "Option must have a type parameter",
                                ))
                            }
                        } else {
                            Err(syn::Error::new_spanned(
                                ty,
                                "Option must have type parameters",
                            ))
                        }
                    }
                    "Value" => {
                        // Handle serde_json::Value specifically
                        if path.segments.len() >= 2 {
                            let full_path = path.segments.iter()
                                .map(|seg| seg.ident.to_string())
                                .collect::<Vec<_>>()
                                .join("::");
                            
                            // Check for serde_json::Value specifically
                            if full_path.ends_with("serde_json::Value") || full_path == "serde_json::Value" {
                                return Ok("smithy.api#Document".to_string());
                            }
                        }
                        
                        // If it's just "Value" without qualification, assume it's serde_json::Value
                        // This handles cases where it's imported as `use serde_json::Value;`
                        Ok("smithy.api#Document".to_string())
                    }
                    _ => {
                        // Handle fully qualified paths that might include serde_json::Value
                        if path.segments.len() > 1 {
                            let full_path = path.segments.iter()
                                .map(|seg| seg.ident.to_string())
                                .collect::<Vec<_>>()
                                .join("::");
                            
                            // Special handling for serde_json::Value
                            if full_path.contains("serde_json") && full_path.ends_with("Value") {
                                return Ok("smithy.api#Document".to_string());
                            }
                            
                            Ok(format!("com.hyperswitch.types#{}", full_path))
                        } else {
                            Ok(format!("com.hyperswitch.types#{}", type_name))
                        }
                    }
                }
            } else {
                Err(syn::Error::new_spanned(ty, "Empty path"))
            }
        }
        Type::Array(type_array) => {
            let inner_smithy_type = rust_type_to_smithy_type(&type_array.elem)?;
            Ok(format!("smithy.api#List<{}>", inner_smithy_type))
        }
        Type::Slice(type_slice) => {
            let inner_smithy_type = rust_type_to_smithy_type(&type_slice.elem)?;
            Ok(format!("smithy.api#List<{}>", inner_smithy_type))
        }
        _ => Err(syn::Error::new_spanned(ty, "Unsupported type")),
    }
}

fn extract_constraints_from_attrs(attrs: &[Attribute]) -> syn::Result<Vec<SmithyConstraint>> {
    let mut constraints = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("smithy") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("pattern") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                constraints.push(SmithyConstraint::Pattern(lit_str.value()));
                            }
                        }
                    }
                } else if meta.path.is_ident("range") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                let range_str = lit_str.value();
                                match parse_range(&range_str) {
                                    Ok((min, max)) => {
                                        constraints.push(SmithyConstraint::Range(min, max));
                                    }
                                    Err(e) => {
                                        return Err(syn::Error::new_spanned(attr, format!("Invalid range: {}", e)));
                                    }
                                }
                            }
                        }
                    }
                } else if meta.path.is_ident("length") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                let length_str = lit_str.value();
                                match parse_length(&length_str) {
                                    Ok((min, max)) => {
                                        constraints.push(SmithyConstraint::Length(min, max));
                                    }
                                    Err(e) => {
                                        return Err(syn::Error::new_spanned(attr, format!("Invalid length: {}", e)));
                                    }
                                }
                            }
                        }
                    }
                } else if meta.path.is_ident("required") {
                    constraints.push(SmithyConstraint::Required);
                }
                Ok(())
            })?; // Propagate parsing errors
        }
    }

    Ok(constraints)
}

fn extract_documentation(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    
    for attr in attrs {
        if attr.path().is_ident("doc") {
            match &attr.meta {
                Meta::NameValue(meta_name_value) => {
                    if let syn::Expr::Lit(expr_lit) = &meta_name_value.value {
                        if let Lit::Str(lit_str) = &expr_lit.lit {
                            docs.push(lit_str.value().trim().to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}

fn is_optional_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            return last_segment.ident == "Option";
        }
    }
    false
}

fn parse_range(range_str: &str) -> Result<(Option<i64>, Option<i64>), String> {
    if range_str.contains("..=") {
        let parts: Vec<&str> = range_str.split("..=").collect();
        if parts.len() != 2 {
            return Err("Invalid range format: must be 'min..=max', '..=max', or 'min..='".to_string());
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse().map_err(|_| format!("Invalid range min: '{}'", parts[0]))?)
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(parts[1].parse().map_err(|_| format!("Invalid range max: '{}'", parts[1]))?)
        };
        Ok((min, max))
    } else if range_str.contains("..") {
        let parts: Vec<&str> = range_str.split("..").collect();
        if parts.len() != 2 {
            return Err("Invalid range format: must be 'min..max', '..max', or 'min..'".to_string());
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse().map_err(|_| format!("Invalid range min: '{}'", parts[0]))?)
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(parts[1].parse::<i64>().map_err(|_| format!("Invalid range max: '{}'", parts[1]))? - 1)
        };
        Ok((min, max))
    } else {
        Err("Invalid range format: must contain '..' or '..='".to_string())
    }
}

fn parse_length(length_str: &str) -> Result<(Option<u64>, Option<u64>), String> {
    if length_str.contains("..=") {
        let parts: Vec<&str> = length_str.split("..=").collect();
        if parts.len() != 2 {
            return Err("Invalid length format: must be 'min..=max', '..=max', or 'min..='".to_string());
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse().map_err(|_| format!("Invalid length min: '{}'", parts[0]))?)
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(parts[1].parse().map_err(|_| format!("Invalid length max: '{}'", parts[1]))?)
        };
        Ok((min, max))
    } else if length_str.contains("..") {
        let parts: Vec<&str> = length_str.split("..").collect();
        if parts.len() != 2 {
            return Err("Invalid length format: must be 'min..max', '..max', or 'min..'".to_string());
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(parts[0].parse().map_err(|_| format!("Invalid length min: '{}'", parts[0]))?)
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(parts[1].parse::<u64>().map_err(|_| format!("Invalid length max: '{}'", parts[1]))? - 1)
        };
        Ok((min, max))
    } else {
        Err("Invalid length format: must contain '..' or '..='".to_string())
    }
}