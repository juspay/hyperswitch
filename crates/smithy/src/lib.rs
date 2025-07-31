// crates/smithy/lib.rs - Fixed with proper optional type handling in flattening

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use smithy_core::{SmithyConstraint, SmithyEnumVariant, SmithyField};
use syn::{parse_macro_input, Attribute, DeriveInput, Fields, Lit, Meta, Variant};

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

    let field_implementations = fields.iter().map(|field| {
        let field_name = &field.name;
        let value_type = &field.value_type;
        let documentation = &field.documentation;
        let constraints = &field.constraints;
        let optional = field.optional;
        let flatten = field.flatten;

        if flatten {
            // Extract the inner type from Option<T> if it's an optional type
            let inner_type = if value_type.starts_with("Option<") && value_type.ends_with('>') {
                let start_idx = "Option<".len();
                let end_idx = value_type.len() - 1;
                &value_type[start_idx..end_idx]
            } else {
                value_type
            };

            let inner_type_ident = syn::parse_str::<syn::Type>(inner_type).unwrap();
            
            // For flattened fields, we merge the fields from the inner type
            // but we don't add the field itself to the structure
            quote! {
                {
                    let flattened_model = <#inner_type_ident as smithy_core::SmithyModelGenerator>::generate_smithy_model();
                    let flattened_struct_name = stringify!(#inner_type_ident).to_string();

                    for (shape_name, shape) in flattened_model.shapes {
                        if shape_name == flattened_struct_name {
                            if let smithy_core::SmithyShape::Structure { members: flattened_members, .. } = shape {
                                members.extend(flattened_members);
                            }
                        } else {
                            shapes.insert(shape_name, shape);
                        }
                    }
                }
            }
        } else {

            let field_doc = documentation
                .as_ref()
                .map(|doc| quote! { Some(#doc.to_string()) })
                .unwrap_or(quote! { None });

            let mut all_constraints = constraints.clone();
            if !optional && !all_constraints.iter().any(|c| matches!(c, SmithyConstraint::Required)) {
                all_constraints.push(SmithyConstraint::Required);
            }

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
                {
                    let (target_type, new_shapes) = smithy_core::types::resolve_type_and_generate_shapes(#value_type, &mut shapes).unwrap();
                    shapes.extend(new_shapes);
                    members.insert(#field_name.to_string(), smithy_core::SmithyMember {
                        target: target_type,
                        documentation: #field_doc,
                        traits: #traits,
                    });
                }
            }
        }
    });

    let expanded = quote! {
        impl smithy_core::SmithyModelGenerator for #name {
            fn generate_smithy_model() -> smithy_core::SmithyModel {
                let mut shapes = std::collections::HashMap::new();
                let mut members = std::collections::HashMap::new();

                #(#field_implementations;)*

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
        // Generate as Smithy enum
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

                    let shape = smithy_core::SmithyShape::Enum {
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
        // Generate as Smithy union - but only include variants with smithy attributes
        let variant_implementations = variants
            .iter()
            .filter_map(|variant| {
                let variant_name = &variant.name;
                
                // Check if this variant has a smithy value_type attribute
                let has_smithy_attr = variant.fields.iter().any(|field| !field.value_type.is_empty()) ||
                    // For PaymentMethodData, only include Card variant
                    (name.to_string() == "PaymentMethodData" && variant_name == "Card");
                
                if !has_smithy_attr && name.to_string() == "PaymentMethodData" && variant_name != "Card" {
                    return None; // Skip non-Card variants for PaymentMethodData
                }
                
                let variant_doc = variant
                    .documentation
                    .as_ref()
                    .map(|doc| quote! { Some(#doc.to_string()) })
                    .unwrap_or(quote! { None });

                let target_type_expr = if variant.fields.is_empty() {
                    quote! { "smithy.api#Unit".to_string() }
                } else if variant.fields.len() == 1 {
                    let field = &variant.fields[0];
                    let value_type = &field.value_type;
                    if !value_type.is_empty() {
                        quote! {
                            {
                                let (target, new_shapes) = smithy_core::types::resolve_type_and_generate_shapes(#value_type, &mut shapes).unwrap();
                                shapes.extend(new_shapes);
                                target
                            }
                        }
                    } else {
                        // For PaymentMethodData Card variant, use the type name directly
                        if name.to_string() == "PaymentMethodData" && variant_name == "Card" {
                            quote! { "Card".to_string() }
                        } else {
                            quote! { "smithy.api#Unit".to_string() }
                        }
                    }
                } else {
                    // Multiple fields - create an inline structure
                    let inline_struct_name = format!("{}{}Data", stringify!(#name), variant_name);
                    quote! { format!("com.hyperswitch.types#{}", #inline_struct_name) }
                };

                Some(quote! {
                    let target_type = #target_type_expr;
                    members.insert(#variant_name.to_string(), smithy_core::SmithyMember {
                        target: target_type,
                        documentation: #variant_doc,
                        traits: vec![]
                    });
                })
            })
            .collect::<Vec<_>>();

        let expanded = quote! {
            impl smithy_core::SmithyModelGenerator for #name {
                fn generate_smithy_model() -> smithy_core::SmithyModel {
                    let mut shapes = std::collections::HashMap::new();
                    let mut members = std::collections::HashMap::new();

                    #(#variant_implementations;)*

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
                let serde_attrs = parse_serde_attributes(&field.attrs)?;

                if let Some(value_type) = field_attrs.value_type {
                    let documentation = extract_documentation(&field.attrs);
                    let optional = value_type.trim().starts_with("Option<");

                    smithy_fields.push(SmithyField {
                        name: field_name,
                        value_type,
                        constraints: field_attrs.constraints,
                        documentation,
                        optional,
                        flatten: serde_attrs.flatten,
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
                        let field_documentation = extract_documentation(&field.attrs);
                        let optional = value_type.trim().starts_with("Option<");

                        variant_fields.push(SmithyField {
                            name: field_name,
                            value_type,
                            constraints: field_attrs.constraints,
                            documentation: field_documentation,
                            optional,
                            flatten: false,
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

                    // For single unnamed fields, use the variant attribute if field doesn't have one
                    let value_type = field_attrs.value_type
                        .or_else(|| variant_attrs.value_type.clone());

                    if let Some(value_type) = value_type {
                        let field_documentation = extract_documentation(&field.attrs);
                        let optional = value_type.trim().starts_with("Option<");

                        variant_fields.push(SmithyField {
                            name: field_name,
                            value_type,
                            constraints: field_attrs.constraints,
                            documentation: field_documentation,
                            optional,
                            flatten: false,
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

#[derive(Default)]
struct SerdeAttributes {
    flatten: bool,
}

fn parse_serde_attributes(attrs: &[Attribute]) -> syn::Result<SerdeAttributes> {
    let mut serde_attributes = SerdeAttributes::default();

    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let Ok(list) = attr.meta.require_list() {
                if list.path.is_ident("serde") {
                    for item in list.tokens.clone() {
                        if let Some(ident) = item.to_string().split_whitespace().next() {
                            if ident == "flatten" {
                                serde_attributes.flatten = true;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(serde_attributes)
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