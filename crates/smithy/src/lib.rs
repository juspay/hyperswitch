// crates/smithy/lib.rs

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use smithy_core::{SmithyConstraint, SmithyField, SmithyEnumVariant};
use syn::{
    parse_macro_input, Attribute, DeriveInput, Fields, Lit, Meta, Type, Variant,
};

/// Derive macro for generating Smithy models from Rust structs and enums
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

    match &input.data {
        syn::Data::Struct(data_struct) => {
            generate_struct_impl(name, &namespace, data_struct, &input.attrs)
        }
        syn::Data::Enum(data_enum) => {
            generate_enum_impl(name, &namespace, data_enum, &input.attrs)
        }
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "SmithyModel can only be derived for structs and enums",
            ))
        }
    }
}

fn generate_struct_impl(
    name: &syn::Ident,
    namespace: &str,
    data_struct: &syn::DataStruct,
    attrs: &[Attribute],
) -> syn::Result<TokenStream2> {
    let fields = extract_fields(&data_struct.fields)?;

    let struct_doc = extract_documentation(attrs);
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

fn generate_enum_impl(
    name: &syn::Ident,
    namespace: &str,
    data_enum: &syn::DataEnum,
    attrs: &[Attribute],
) -> syn::Result<TokenStream2> {
    let variants = extract_enum_variants(&data_enum.variants)?;

    let enum_doc = extract_documentation(attrs);
    let enum_doc_expr = enum_doc
        .as_ref()
        .map(|doc| quote! { Some(#doc.to_string()) })
        .unwrap_or(quote! { None });

    // Check if this is a string enum (all variants are unit variants) or a union
    let is_string_enum = variants.iter().all(|v| v.fields.is_empty());

    if is_string_enum {
        // Generate as Smithy string enum
        let variant_implementations = variants
            .iter()
            .map(|variant| {
                let variant_name = &variant.name;
                let variant_doc = variant
                    .documentation
                    .as_ref()
                    .map(|doc| quote! { Some(#doc.to_string()) })
                    .unwrap_or(quote! { None });

                quote! {
                    enum_values.insert(#variant_name.to_string(), smithy_core::SmithyEnumValue {
                        name: #variant_name.to_string(),
                        documentation: #variant_doc,
                    });
                }
            })
            .collect::<Vec<_>>();

        let expanded = quote! {
            impl smithy_core::SmithyModelGenerator for #name {
                fn generate_smithy_model() -> smithy_core::SmithyModel {
                    let mut shapes = std::collections::HashMap::new();
                    let mut enum_values = std::collections::HashMap::new();

                    #(#variant_implementations)*

                    let shape = smithy_core::SmithyShape::StringEnum {
                        values: enum_values,
                        documentation: #enum_doc_expr,
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
    } else {
        // Generate as Smithy union
        let variant_implementations = variants
            .iter()
            .map(|variant| {
                let variant_name = &variant.name;
                let variant_doc = variant
                    .documentation
                    .as_ref()
                    .map(|doc| quote! { Some(#doc.to_string()) })
                    .unwrap_or(quote! { None });

                // Determine the target type for this variant
                let target_type = if variant.fields.is_empty() {
                    // Unit variant - use a special unit type
                    quote! { "smithy.api#Unit".to_string() }
                } else if variant.fields.len() == 1 {
                    // Single field - use its type
                    let field = &variant.fields[0];
                    let smithy_type = &field.smithy_type;
                    quote! { #smithy_type.to_string() }
                } else {
                    // Multiple fields - create an inline structure
                    let inline_struct_name = format!("{}{}Data", stringify!(#name), variant_name);
                    quote! { format!("com.hyperswitch.types#{}", #inline_struct_name) }
                };

                quote! {
                    members.insert(#variant_name.to_string(), smithy_core::SmithyMember {
                        target: #target_type,
                        documentation: #variant_doc,
                        traits: vec![]
                    });
                }
            })
            .collect::<Vec<_>>();

        let expanded = quote! {
            impl smithy_core::SmithyModelGenerator for #name {
                fn generate_smithy_model() -> smithy_core::SmithyModel {
                    let mut shapes = std::collections::HashMap::new();
                    let mut members = std::collections::HashMap::new();

                    #(#variant_implementations)*

                    let shape = smithy_core::SmithyShape::Union {
                        members,
                        documentation: #enum_doc_expr,
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
                let field_attrs = parse_smithy_field_attributes(&field.attrs)?;

                if let Some(value_type) = field_attrs.value_type {
                    let smithy_type = convert_value_type_to_smithy_type(&value_type)?;
                    let documentation = extract_documentation(&field.attrs);
                    let optional = is_optional_type(&field.ty);

                    smithy_fields.push(SmithyField {
                        name: field_name,
                        smithy_type,
                        constraints: field_attrs.constraints,
                        documentation,
                        optional,
                    });
                }
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

fn extract_enum_variants(
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<Vec<SmithyEnumVariant>> {
    let mut smithy_variants = Vec::new();

    for variant in variants {
        let variant_name = variant.ident.to_string();
        let documentation = extract_documentation(&variant.attrs);
        let variant_attrs = parse_smithy_field_attributes(&variant.attrs)?;

        // Extract fields from the variant
        let fields = match &variant.fields {
            Fields::Unit => Vec::new(),
            Fields::Named(fields_named) => {
                let mut variant_fields = Vec::new();
                for field in &fields_named.named {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let field_attrs = parse_smithy_field_attributes(&field.attrs)?;

                    if let Some(value_type) = field_attrs.value_type {
                        let smithy_type = convert_value_type_to_smithy_type(&value_type)?;
                        let field_documentation = extract_documentation(&field.attrs);
                        let optional = is_optional_type(&field.ty);

                        variant_fields.push(SmithyField {
                            name: field_name,
                            smithy_type,
                            constraints: field_attrs.constraints,
                            documentation: field_documentation,
                            optional,
                        });
                    }
                }
                variant_fields
            }
            Fields::Unnamed(fields_unnamed) => {
                let mut variant_fields = Vec::new();
                for (index, field) in fields_unnamed.unnamed.iter().enumerate() {
                    let field_name = format!("field_{}", index);
                    let field_attrs = parse_smithy_field_attributes(&field.attrs)?;

                    if let Some(value_type) = field_attrs.value_type {
                        let smithy_type = convert_value_type_to_smithy_type(&value_type)?;
                        let field_documentation = extract_documentation(&field.attrs);
                        let optional = is_optional_type(&field.ty);

                        variant_fields.push(SmithyField {
                            name: field_name,
                            smithy_type,
                            constraints: field_attrs.constraints,
                            documentation: field_documentation,
                            optional,
                        });
                    }
                }
                variant_fields
            }
        };

        smithy_variants.push(SmithyEnumVariant {
            name: variant_name,
            fields,
            constraints: variant_attrs.constraints,
            documentation,
        });
    }

    Ok(smithy_variants)
}

#[derive(Default)]
struct SmithyFieldAttributes {
    value_type: Option<String>,
    constraints: Vec<SmithyConstraint>,
}

fn parse_smithy_field_attributes(attrs: &[Attribute]) -> syn::Result<SmithyFieldAttributes> {
    let mut field_attributes = SmithyFieldAttributes::default();

    for attr in attrs {
        if attr.path().is_ident("smithy") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value_type") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                field_attributes.value_type = Some(lit_str.value());
                            }
                        }
                    }
                } else if meta.path.is_ident("pattern") {
                    if let Ok(value) = meta.value() {
                        if let Ok(lit) = value.parse::<Lit>() {
                            if let Lit::Str(lit_str) = lit {
                                field_attributes
                                    .constraints
                                    .push(SmithyConstraint::Pattern(lit_str.value()));
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
                                        field_attributes
                                            .constraints
                                            .push(SmithyConstraint::Range(min, max));
                                    }
                                    Err(e) => {
                                        return Err(syn::Error::new_spanned(
                                            &meta.path,
                                            format!("Invalid range: {}", e),
                                        ));
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
                                        field_attributes
                                            .constraints
                                            .push(SmithyConstraint::Length(min, max));
                                    }
                                    Err(e) => {
                                        return Err(syn::Error::new_spanned(
                                            &meta.path,
                                            format!("Invalid length: {}", e),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                } else if meta.path.is_ident("required") {
                    field_attributes.constraints.push(SmithyConstraint::Required);
                }
                Ok(())
            })?;
        }
    }

    Ok(field_attributes)
}

fn convert_value_type_to_smithy_type(value_type: &str) -> syn::Result<String> {
    // Handle the value_type string and convert it to appropriate Smithy type
    convert_value_type_to_smithy_type_recursive(value_type.trim())
}

fn convert_value_type_to_smithy_type_recursive(value_type: &str) -> syn::Result<String> {
    let value_type_span = proc_macro2::Span::call_site();
    match value_type {
        // Basic primitive types
        "String" | "str" => Ok("smithy.api#String".to_string()),
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" => Ok("smithy.api#Integer".to_string()),
        "i64" | "u64" | "isize" | "usize" => Ok("smithy.api#Long".to_string()),
        "f32" => Ok("smithy.api#Float".to_string()),
        "f64" => Ok("smithy.api#Double".to_string()),
        "bool" => Ok("smithy.api#Boolean".to_string()),

        // Special hyperswitch types
        "Amount" | "MinorUnit" => Ok("smithy.api#Long".to_string()),

        // Handle serde_json::Value
        "serde_json::Value" | "Value" => Ok("smithy.api#Document".to_string()),

        // Handle generic types with recursion
        vt if vt.starts_with("Option<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Option")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            convert_value_type_to_smithy_type_recursive(inner_type)
        }

        vt if vt.starts_with("Vec<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Vec")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let inner_smithy_type = convert_value_type_to_smithy_type_recursive(inner_type)?;
            Ok(format!("smithy.api#List<{}>", inner_smithy_type))
        }

        vt if vt.starts_with("Box<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Box")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            convert_value_type_to_smithy_type_recursive(inner_type)
        }

        vt if vt.starts_with("Secret<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Secret")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            convert_value_type_to_smithy_type_recursive(inner_type)
        }

        // Handle HashMap and BTreeMap
        vt if vt.starts_with("HashMap<") && vt.ends_with('>') => {
            let inner_types = extract_generic_inner_type(vt, "HashMap")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_type, value_type) =
                parse_map_types(inner_types).map_err(|e| syn::Error::new(value_type_span, e))?;
            let key_smithy_type = convert_value_type_to_smithy_type_recursive(key_type)?;
            let value_smithy_type = convert_value_type_to_smithy_type_recursive(value_type)?;
            Ok(format!(
                "smithy.api#Map<{}, {}>",
                key_smithy_type, value_smithy_type
            ))
        }

        vt if vt.starts_with("BTreeMap<") && vt.ends_with('>') => {
            let inner_types = extract_generic_inner_type(vt, "BTreeMap")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_type, value_type) =
                parse_map_types(inner_types).map_err(|e| syn::Error::new(value_type_span, e))?;
            let key_smithy_type = convert_value_type_to_smithy_type_recursive(key_type)?;
            let value_smithy_type = convert_value_type_to_smithy_type_recursive(value_type)?;
            Ok(format!(
                "smithy.api#Map<{}, {}>",
                key_smithy_type, value_smithy_type
            ))
        }

        // Custom types - check if it contains module path separators
        _ => {
            // Handle fully qualified paths (e.g., "api_enums::Currency", "payments::CaptureMethod")
            if value_type.contains("::") {
                // For qualified paths, use the full path but replace :: with .
                let smithy_path = value_type.replace("::", ".");
                Ok(format!("com.hyperswitch.types#{}", smithy_path))
            } else {
                // For simple custom types (e.g., "CaptureMethod", "Currency")
                Ok(format!("com.hyperswitch.types#{}", value_type))
            }
        }
    }
}

/// Extract the inner type from a generic type like Option<T>, Vec<T>, etc.
fn extract_generic_inner_type<'a>(full_type: &'a str, wrapper: &str) -> Result<&'a str, String> {
    let expected_start = format!("{}<", wrapper);

    if !full_type.starts_with(&expected_start) || !full_type.ends_with('>') {
        return Err(format!("Invalid {} type format: {}", wrapper, full_type));
    }

    let start_idx = expected_start.len();
    let end_idx = full_type.len() - 1;

    if start_idx >= end_idx {
        return Err(format!("Empty {} type: {}", wrapper, full_type));
    }

    Ok(full_type[start_idx..end_idx].trim())
}

/// Parse map types like "String, i32" into ("String", "i32")
fn parse_map_types(inner_types: &str) -> Result<(&str, &str), String> {
    // Handle nested generics by counting angle brackets
    let mut bracket_count = 0;
    let mut comma_pos = None;

    for (i, ch) in inner_types.char_indices() {
        match ch {
            '<' => bracket_count += 1,
            '>' => bracket_count -= 1,
            ',' if bracket_count == 0 => {
                comma_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = comma_pos {
        let key_type = inner_types[..pos].trim();
        let value_type = inner_types[pos + 1..].trim();

        if key_type.is_empty() || value_type.is_empty() {
            return Err(format!("Invalid map type format: {}", inner_types));
        }

        Ok((key_type, value_type))
    } else {
        Err(format!(
            "Invalid map type format, missing comma: {}",
            inner_types
        ))
    }
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
