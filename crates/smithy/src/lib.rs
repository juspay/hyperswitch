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
    let (namespace, is_mixin) = extract_namespace_and_mixin(&input.attrs)?;

    match &input.data {
        syn::Data::Struct(data_struct) => {
            generate_struct_impl(name, &namespace, data_struct, &input.attrs, is_mixin)
        }
        syn::Data::Enum(data_enum) => generate_enum_impl(name, &namespace, data_enum, &input.attrs),
        _ => Err(syn::Error::new_spanned(
            input,
            "SmithyModel can only be derived for structs and enums",
        )),
    }
}

fn generate_struct_impl(
    name: &syn::Ident,
    namespace: &str,
    data_struct: &syn::DataStruct,
    attrs: &[Attribute],
    is_mixin: bool,
) -> syn::Result<TokenStream2> {
    let fields = extract_fields(&data_struct.fields)?;

    let struct_doc = extract_documentation(attrs);
    let struct_doc_expr = struct_doc
        .as_ref()
        .map(|doc| quote! { Some(#doc.to_string()) })
        .unwrap_or(quote! { None });

    // Count flattened vs non-flattened fields to determine shape type
    let flattened_fields: Vec<_> = fields.iter().filter(|f| f.flatten).collect();
    let non_flattened_fields: Vec<_> = fields.iter().filter(|f| !f.flatten).collect();

    // Use smart runtime inspection for structs with only a single flattened field
    let should_use_smart_generation =
        non_flattened_fields.is_empty() && flattened_fields.len() == 1;

    if should_use_smart_generation {
        // Generate smart logic that determines union vs structure at runtime based on the flattened type
        return generate_union_from_flattened_struct(
            name,
            namespace,
            flattened_fields[0],
            &struct_doc_expr,
        );
    }

    // Otherwise, generate Structure (existing logic)
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
                            match shape {
                                smithy_core::SmithyShape::Structure { members: flattened_members, .. } |
                                smithy_core::SmithyShape::Union { members: flattened_members, .. } => {
                                    members.extend(flattened_members);
                                }
                                _ => {
                                    // Potentially handle other shapes or log a warning
                                }
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
                        SmithyConstraint::HttpLabel => quote! {
                            smithy_core::SmithyTrait::HttpLabel
                        },
                        SmithyConstraint::HttpQuery(name) => quote! {
                            smithy_core::SmithyTrait::HttpQuery { name: #name.to_string() }
                        },
                        SmithyConstraint::JsonName(name) => quote! {
                            smithy_core::SmithyTrait::JsonName { name: #name.to_string() }
                        },
                        SmithyConstraint::EnumValue(value) => quote! {
                            smithy_core::SmithyTrait::EnumValue { value: #value.to_string() }
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

    let traits_expr = if is_mixin {
        quote! { vec![smithy_core::SmithyTrait::Mixin] }
    } else {
        quote! { vec![] }
    };

    let expanded = quote! {
        impl smithy_core::SmithyModelGenerator for #name {
            fn generate_smithy_model() -> smithy_core::SmithyModel {
                let mut shapes = std::collections::HashMap::new();
                let mut members = std::collections::HashMap::new();

                #(#field_implementations;)*

                let shape = smithy_core::SmithyShape::Structure {
                    members,
                    documentation: #struct_doc_expr,
                    traits: #traits_expr
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

fn generate_union_from_flattened_struct(
    name: &syn::Ident,
    namespace: &str,
    flattened_field: &SmithyField,
    struct_doc_expr: &proc_macro2::TokenStream,
) -> syn::Result<TokenStream2> {
    let value_type = &flattened_field.value_type;

    // Extract the inner type from Option<T> if it's an optional type
    let inner_type = if value_type.starts_with("Option<") && value_type.ends_with('>') {
        let start_idx = "Option<".len();
        let end_idx = value_type.len() - 1;
        &value_type[start_idx..end_idx]
    } else {
        value_type
    };

    let inner_type_ident = syn::parse_str::<syn::Type>(inner_type).unwrap();

    let expanded = quote! {
        impl smithy_core::SmithyModelGenerator for #name {
            fn generate_smithy_model() -> smithy_core::SmithyModel {
                let mut shapes = std::collections::HashMap::new();
                let mut members = std::collections::HashMap::new();

                // Get the flattened model and determine if it's actually an enum/union
                let flattened_model = <#inner_type_ident as smithy_core::SmithyModelGenerator>::generate_smithy_model();
                let flattened_struct_name = stringify!(#inner_type_ident).to_string();

                // Check if the flattened type is actually an enum or union
                let mut is_flattened_enum_or_union = false;

                // Find the target shape in the flattened model
                for (shape_name, shape) in flattened_model.shapes.clone() {
                    if shape_name == flattened_struct_name {
                        match &shape {
                            smithy_core::SmithyShape::Union { .. } |
                            smithy_core::SmithyShape::Enum { .. } => {
                                is_flattened_enum_or_union = true;
                            },
                            smithy_core::SmithyShape::Structure { .. } => {
                                is_flattened_enum_or_union = false;
                            },
                            _ => {
                                is_flattened_enum_or_union = false;
                            }
                        }
                        break;
                    }
                }

                if is_flattened_enum_or_union {
                    // Generate as Union: flattened type is enum/union
                    for (shape_name, shape) in flattened_model.shapes {
                        if shape_name == flattened_struct_name {
                            match shape {
                                smithy_core::SmithyShape::Union { members: flattened_members, .. } => {
                                    // If the flattened type is already a union, use its members
                                    members.extend(flattened_members);
                                },
                                smithy_core::SmithyShape::Enum { values, .. } => {
                                    // If the flattened type is an enum, convert enum values to union members
                                    for (enum_name, enum_value) in values {
                                        members.insert(enum_name, smithy_core::SmithyMember {
                                            target: "smithy.api#Unit".to_string(),
                                            documentation: enum_value.documentation,
                                            traits: vec![],
                                        });
                                    }
                                },
                                _ => {
                                    // Fallback case
                                    members.insert("value".to_string(), smithy_core::SmithyMember {
                                        target: flattened_struct_name.clone(),
                                        documentation: None,
                                        traits: vec![],
                                    });
                                }
                            }
                        } else {
                            // Add all other shapes from the flattened model
                            shapes.insert(shape_name, shape);
                        }
                    }

                    // Create the union shape
                    let shape = smithy_core::SmithyShape::Union {
                        members,
                        documentation: #struct_doc_expr,
                        traits: vec![]
                    };

                    shapes.insert(stringify!(#name).to_string(), shape);
                } else {
                    // Generate as Structure: flattened type is struct, merge fields
                    for (shape_name, shape) in flattened_model.shapes {
                        if shape_name == flattened_struct_name {
                            match shape {
                                smithy_core::SmithyShape::Structure { members: flattened_members, .. } => {
                                    members.extend(flattened_members);
                                }
                                _ => {
                                    // Fallback - add as single field
                                    members.insert("value".to_string(), smithy_core::SmithyMember {
                                        target: flattened_struct_name.clone(),
                                        documentation: None,
                                        traits: vec![],
                                    });
                                }
                            }
                        } else {
                            shapes.insert(shape_name, shape);
                        }
                    }

                    // Create the structure shape
                    let shape = smithy_core::SmithyShape::Structure {
                        members,
                        documentation: #struct_doc_expr,
                        traits: vec![]
                    };

                    shapes.insert(stringify!(#name).to_string(), shape);
                }

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
    let serde_enum_attrs = parse_serde_enum_attributes(attrs)?;

    let enum_doc = extract_documentation(attrs);
    let enum_doc_expr = enum_doc
        .as_ref()
        .map(|doc| quote! { Some(#doc.to_string()) })
        .unwrap_or(quote! { None });

    // Check if this is a tagged enum, string enum, or union
    let is_string_enum = variants.iter().all(|v| v.fields.is_empty());
    let has_nested_value_type = variants.iter().any(|v| v.nested_value_type);
    let is_tagged_enum = serde_enum_attrs.tag.is_some() && !is_string_enum;

    if is_tagged_enum {
        // Generate tagged enum as a structure with tag field + all variant fields as optional
        // Plus a separate enum for the variants
        generate_tagged_enum_impl(
            name,
            namespace,
            &variants,
            &serde_enum_attrs,
            &enum_doc_expr,
        )
    } else if is_string_enum && !has_nested_value_type {
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

                // Apply serde rename transformation if specified
                let rename_all = serde_enum_attrs.rename_all.as_deref();
                let transformed_name = if let Some(rename_pattern) = rename_all {
                    // Generate the transformation at compile time
                    let transformed = transform_variant_name(variant_name, Some(rename_pattern));
                    quote! { #transformed.to_string() }
                } else {
                    quote! { #variant_name.to_string() }
                };

                // Generate traits for enum value
                let traits = if variant.constraints.is_empty() {
                    quote! { vec![] }
                } else {
                    let trait_tokens = variant
                        .constraints
                        .iter()
                        .map(|constraint| match constraint {
                            SmithyConstraint::Pattern(pattern) => quote! {
                                smithy_core::SmithyTrait::Pattern { pattern: #pattern.to_string() }
                            },
                            SmithyConstraint::Range(min, max) => {
                                let min_expr =
                                    min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                                let max_expr =
                                    max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                                quote! {
                                    smithy_core::SmithyTrait::Range {
                                        min: #min_expr,
                                        max: #max_expr
                                    }
                                }
                            }
                            SmithyConstraint::Length(min, max) => {
                                let min_expr =
                                    min.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                                let max_expr =
                                    max.map(|v| quote! { Some(#v) }).unwrap_or(quote! { None });
                                quote! {
                                    smithy_core::SmithyTrait::Length {
                                        min: #min_expr,
                                        max: #max_expr
                                    }
                                }
                            }
                            SmithyConstraint::Required => quote! {
                                smithy_core::SmithyTrait::Required
                            },
                            SmithyConstraint::HttpLabel => quote! {
                                smithy_core::SmithyTrait::HttpLabel
                            },
                            SmithyConstraint::HttpQuery(name) => quote! {
                                smithy_core::SmithyTrait::HttpQuery { name: #name.to_string() }
                            },
                            SmithyConstraint::JsonName(name) => quote! {
                                smithy_core::SmithyTrait::JsonName { name: #name.to_string() }
                            },
                            SmithyConstraint::EnumValue(value) => quote! {
                                smithy_core::SmithyTrait::EnumValue { value: #value.to_string() }
                            },
                        })
                        .collect::<Vec<_>>();

                    quote! { vec![#(#trait_tokens),*] }
                };

                quote! {
                    enum_values.insert(#transformed_name, smithy_core::SmithyEnumValue {
                        name: #transformed_name,
                        documentation: #variant_doc,
                        is_default: false,
                        traits: #traits,
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
        // Generate as Smithy union
        let variant_implementations = variants
            .iter()
            .filter_map(|variant| {
                let variant_name = &variant.name;
                let variant_doc = variant
                    .documentation
                    .as_ref()
                    .map(|doc| quote! { Some(#doc.to_string()) })
                    .unwrap_or(quote! { None });

                let target_type_expr = if variant.nested_value_type {
                    // Force nested structure creation when nested_value_type is specified
                    // This works for both empty and non-empty variants
                    let nested_struct_members = variant.fields.iter().map(|field| {
                        let field_name = &field.name;
                        let field_value_type = &field.value_type;
                        let field_doc = field
                            .documentation
                            .as_ref()
                            .map(|doc| quote! { Some(#doc.to_string()) })
                            .unwrap_or(quote! { None });

                        let mut field_constraints = field.constraints.clone();
                        if !field.optional && !field_constraints.iter().any(|c| matches!(c, SmithyConstraint::Required)) {
                            field_constraints.push(SmithyConstraint::Required);
                        }

                        let field_traits = if field_constraints.is_empty() {
                            quote! { vec![] }
                        } else {
                            let trait_tokens = field_constraints
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
                                    SmithyConstraint::HttpLabel => quote! {
                                        smithy_core::SmithyTrait::HttpLabel
                                    },
                                    SmithyConstraint::HttpQuery(name) => quote! {
                                        smithy_core::SmithyTrait::HttpQuery { name: #name.to_string() }
                                    },
                                    SmithyConstraint::JsonName(name) => quote! {
                                        smithy_core::SmithyTrait::JsonName { name: #name.to_string() }
                                    },
                                    SmithyConstraint::EnumValue(value) => quote! {
                                        smithy_core::SmithyTrait::EnumValue { value: #value.to_string() }
                                    },
                                })
                                .collect::<Vec<_>>();

                            quote! { vec![#(#trait_tokens),*] }
                        };

                        quote! {
                            {
                                let (field_target, field_shapes) = smithy_core::types::resolve_type_and_generate_shapes(#field_value_type, &mut shapes).unwrap();
                                shapes.extend(field_shapes);
                                nested_members.insert(#field_name.to_string(), smithy_core::SmithyMember {
                                    target: field_target,
                                    documentation: #field_doc,
                                    traits: #field_traits,
                                });
                            }
                        }
                    });

                    quote! {
                        {
                            let nested_struct_name = format!("{}NestedType", #variant_name);
                            let mut nested_members = std::collections::HashMap::new();
                            #(#nested_struct_members)*
                            let nested_shape = smithy_core::SmithyShape::Structure {
                                members: nested_members,
                                documentation: None,
                                traits: vec![],
                            };
                            shapes.insert(nested_struct_name.clone(), nested_shape);
                            nested_struct_name
                        }
                    }
                } else if variant.fields.is_empty() {
                    // If there are no fields but variant has a value_type, use that
                    if let Some(variant_value_type) = &variant.value_type {
                        quote! { #variant_value_type.to_string() }
                    } else {
                        // If there are no fields and no variant value_type, this variant should be skipped
                        return None;
                    }
                } else if variant.fields.len() == 1 {
                    // Single field - reference the type directly instead of creating a wrapper
                    let field = &variant.fields[0];
                    let field_value_type = &field.value_type;
                    if field_value_type.is_empty() {
                        return None;
                    }

                    quote! {
                        {
                            let (target_type, new_shapes) = smithy_core::types::resolve_type_and_generate_shapes(#field_value_type, &mut shapes).unwrap();
                            shapes.extend(new_shapes);
                            target_type
                        }
                    }
                } else {
                    // Multiple fields - create an inline structure
                    let inline_struct_members = variant.fields.iter().map(|field| {
                        let field_name = &field.name;
                        let field_value_type = &field.value_type;
                        let field_doc = field
                            .documentation
                            .as_ref()
                            .map(|doc| quote! { Some(#doc.to_string()) })
                            .unwrap_or(quote! { None });

                        let mut field_constraints = field.constraints.clone();
                        if !field.optional && !field_constraints.iter().any(|c| matches!(c, SmithyConstraint::Required)) {
                            field_constraints.push(SmithyConstraint::Required);
                        }

                        let field_traits = if field_constraints.is_empty() {
                            quote! { vec![] }
                        } else {
                            let trait_tokens = field_constraints
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
                                    SmithyConstraint::HttpLabel => quote! {
                                        smithy_core::SmithyTrait::HttpLabel
                                    },
                                    SmithyConstraint::HttpQuery(name) => quote! {
                                        smithy_core::SmithyTrait::HttpQuery { name: #name.to_string() }
                                    },
                                    SmithyConstraint::JsonName(name) => quote! {
                                        smithy_core::SmithyTrait::JsonName { name: #name.to_string() }
                                    },
                                    SmithyConstraint::EnumValue(value) => quote! {
                                        smithy_core::SmithyTrait::EnumValue { value: #value.to_string() }
                                    },
                                })
                                .collect::<Vec<_>>();

                            quote! { vec![#(#trait_tokens),*] }
                        };

                        quote! {
                            {
                                let (field_target, field_shapes) = smithy_core::types::resolve_type_and_generate_shapes(#field_value_type, &mut shapes).unwrap();
                                shapes.extend(field_shapes);
                                inline_members.insert(#field_name.to_string(), smithy_core::SmithyMember {
                                    target: field_target,
                                    documentation: #field_doc,
                                    traits: #field_traits,
                                });
                            }
                        }
                    });

                    quote! {
                        {
                            let inline_struct_name = format!("{}{}Data", stringify!(#name), #variant_name);
                            let mut inline_members = std::collections::HashMap::new();
                            #(#inline_struct_members)*
                            let inline_shape = smithy_core::SmithyShape::Structure {
                                members: inline_members,
                                documentation: None,
                                traits: vec![],
                            };
                            shapes.insert(inline_struct_name.clone(), inline_shape);
                            inline_struct_name
                        }
                    }
                };

                // Apply serde rename transformation if specified
                let rename_all = serde_enum_attrs.rename_all.as_deref();
                let transformed_name = if let Some(rename_pattern) = rename_all {
                    // Generate the transformation at compile time
                    let transformed = transform_variant_name(variant_name, Some(rename_pattern));
                    quote! { #transformed.to_string() }
                } else {
                    quote! { #variant_name.to_string() }
                };

                Some(quote! {
                    let target_type = #target_type_expr;
                    members.insert(#transformed_name, smithy_core::SmithyMember {
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

fn generate_tagged_enum_impl(
    name: &syn::Ident,
    namespace: &str,
    variants: &[SmithyEnumVariant],
    serde_enum_attrs: &SerdeEnumAttributes,
    enum_doc_expr: &proc_macro2::TokenStream,
) -> syn::Result<TokenStream2> {
    let tag_field_name = serde_enum_attrs.tag.as_ref().unwrap();
    let variants_enum_name = format!("{}EnumVariants", name);

    // Collect all unique fields from all variants
    let mut all_fields = std::collections::HashMap::new();

    for variant in variants {
        for field in &variant.fields {
            // Make all variant fields optional by wrapping in Option<> if not already
            let optional_type = if field.optional {
                field.value_type.clone()
            } else {
                format!("Option<{}>", field.value_type)
            };

            all_fields.insert(
                field.name.clone(),
                (
                    optional_type,
                    field.documentation.clone(),
                    field.constraints.clone(),
                ),
            );
        }
    }

    // Generate field implementations for the main structure
    let field_implementations = all_fields.iter().map(|(field_name, (value_type, documentation, constraints))| {
        let field_doc = documentation
            .as_ref()
            .map(|doc| quote! { Some(#doc.to_string()) })
            .unwrap_or(quote! { None });

        let traits = if constraints.is_empty() {
            quote! { vec![] }
        } else {
            let trait_tokens = constraints
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
                    SmithyConstraint::HttpLabel => quote! {
                        smithy_core::SmithyTrait::HttpLabel
                    },
                    SmithyConstraint::HttpQuery(name) => quote! {
                        smithy_core::SmithyTrait::HttpQuery { name: #name.to_string() }
                    },
                    SmithyConstraint::JsonName(name) => quote! {
                        smithy_core::SmithyTrait::JsonName { name: #name.to_string() }
                    },
                    SmithyConstraint::EnumValue(value) => quote! {
                        smithy_core::SmithyTrait::EnumValue { value: #value.to_string() }
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
    });

    // Generate variant enum values
    let variant_implementations = variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.name;
            let variant_doc = variant
                .documentation
                .as_ref()
                .map(|doc| quote! { Some(#doc.to_string()) })
                .unwrap_or(quote! { None });

            // Apply serde rename transformation if specified
            let rename_all = serde_enum_attrs.rename_all.as_deref();
            let transformed_name = if let Some(rename_pattern) = rename_all {
                let transformed = transform_variant_name(variant_name, Some(rename_pattern));
                quote! { #transformed.to_string() }
            } else {
                quote! { #variant_name.to_string() }
            };

            quote! {
                enum_values.insert(#transformed_name, smithy_core::SmithyEnumValue {
                    name: #transformed_name,
                    documentation: #variant_doc,
                    is_default: false,
                    traits: vec![],
                });
            }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        impl smithy_core::SmithyModelGenerator for #name {
            fn generate_smithy_model() -> smithy_core::SmithyModel {
                let mut shapes = std::collections::HashMap::new();
                let mut members = std::collections::HashMap::new();

                // Add all variant fields as optional members
                #(#field_implementations;)*

                // Add the tag field - required and references the variants enum
                members.insert(#tag_field_name.to_string(), smithy_core::SmithyMember {
                    target: #variants_enum_name.to_string(),
                    documentation: Some("Discriminator field for the tagged enum".to_string()),
                    traits: vec![smithy_core::SmithyTrait::Required],
                });

                // Create the main structure
                let main_shape = smithy_core::SmithyShape::Structure {
                    members,
                    documentation: #enum_doc_expr,
                    traits: vec![]
                };

                shapes.insert(stringify!(#name).to_string(), main_shape);

                // Create the variants enum
                let mut enum_values = std::collections::HashMap::new();
                #(#variant_implementations)*

                let variants_shape = smithy_core::SmithyShape::Enum {
                    values: enum_values,
                    documentation: Some(format!("Enum variants for {}", stringify!(#name))),
                    traits: vec![]
                };

                shapes.insert(#variants_enum_name.to_string(), variants_shape);

                smithy_core::SmithyModel {
                    namespace: #namespace.to_string(),
                    shapes
                }
            }
        }
    };

    Ok(expanded)
}

fn extract_namespace_and_mixin(attrs: &[Attribute]) -> syn::Result<(String, bool)> {
    for attr in attrs {
        if attr.path().is_ident("smithy") {
            let mut namespace = None;
            let mut mixin = false;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("namespace") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            namespace = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("mixin") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Bool(lit_bool)) = value.parse::<Lit>() {
                            mixin = lit_bool.value;
                        }
                    }
                }
                Ok(())
            })?; // Propagate parsing errors

            return Ok((
                namespace.unwrap_or_else(|| "com.hyperswitch.default".to_string()),
                mixin,
            ));
        }
    }
    Ok(("com.hyperswitch.default".to_string(), false))
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

                    // Apply JsonName trait if field has serde rename
                    let mut constraints = field_attrs.constraints;
                    if let Some(rename_value) = &serde_attrs.rename {
                        // Add JsonName constraint for serde rename
                        constraints.push(SmithyConstraint::JsonName(rename_value.clone()));
                    }

                    smithy_fields.push(SmithyField {
                        name: field_name,
                        value_type,
                        constraints,
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
        let variant_serde_attrs = parse_serde_attributes(&variant.attrs)?;

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
                    let value_type = field_attrs
                        .value_type
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

        // Apply EnumValue trait if variant has serde rename
        let mut constraints = variant_attrs.constraints;
        if let Some(rename_value) = &variant_serde_attrs.rename {
            // Add EnumValue constraint for serde rename
            constraints.push(SmithyConstraint::EnumValue(rename_value.clone()));
        }

        smithy_variants.push(SmithyEnumVariant {
            name: variant_name,
            fields,
            constraints,
            documentation,
            nested_value_type: variant_attrs.nested_value_type,
            value_type: variant_attrs.value_type,
        });
    }

    Ok(smithy_variants)
}

#[derive(Default)]
struct SmithyFieldAttributes {
    value_type: Option<String>,
    constraints: Vec<SmithyConstraint>,
    nested_value_type: bool,
}

#[derive(Default)]
struct SerdeAttributes {
    flatten: bool,
    tag: Option<String>,
    rename: Option<String>,
}

#[derive(Default)]
struct SerdeEnumAttributes {
    rename_all: Option<String>,
    tag: Option<String>,
    rename: Option<String>,
}

fn parse_serde_attributes(attrs: &[Attribute]) -> syn::Result<SerdeAttributes> {
    let mut serde_attributes = SerdeAttributes::default();

    for attr in attrs {
        if attr.path().is_ident("serde") {
            // Use more robust parsing that handles all serde attributes
            let parse_result = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("flatten") {
                    serde_attributes.flatten = true;
                } else if meta.path.is_ident("tag") {
                    // Parse and capture the tag attribute
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            serde_attributes.tag = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("rename_all") {
                    // Parse and ignore the rename_all attribute for structs
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("content") {
                    // Parse and ignore the content attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("rename") {
                    // Parse and capture the rename attribute
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            serde_attributes.rename = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("deny_unknown_fields") {
                    // Handle deny_unknown_fields (no value needed)
                } else if meta.path.is_ident("skip_serializing") {
                    // Handle skip_serializing
                } else if meta.path.is_ident("skip_deserializing") {
                    // Handle skip_deserializing
                } else if meta.path.is_ident("skip_serializing_if") {
                    // Handle skip_serializing_if
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<syn::Expr>();
                    }
                } else if meta.path.is_ident("default") {
                    // Handle default attribute
                    if meta.value().is_ok() {
                        let _ = meta.value().and_then(|v| v.parse::<syn::Expr>());
                    }
                } else if meta.path.is_ident("untagged") {
                    // Handle untagged (flag attribute)
                } else if meta.path.is_ident("bound") {
                    // Handle bound attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("with") {
                    // Handle with attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("serialize_with") {
                    // Handle serialize_with attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("deserialize_with") {
                    // Handle deserialize_with attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("alias") {
                    // Handle alias attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                }
                // Silently ignore any other serde attributes to prevent parsing errors
                Ok(())
            });

            // If parsing failed, provide a more helpful error message
            if let Err(e) = parse_result {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("Failed to parse serde attribute: {}. This may be due to multiple serde attributes on separate lines. Consider consolidating them into a single #[serde(...)] attribute.", e)
                ));
            }
        }
    }

    Ok(serde_attributes)
}

fn parse_serde_enum_attributes(attrs: &[Attribute]) -> syn::Result<SerdeEnumAttributes> {
    let mut serde_enum_attributes = SerdeEnumAttributes::default();

    for attr in attrs {
        if attr.path().is_ident("serde") {
            // Use more robust parsing that handles all serde attributes
            let parse_result = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename_all") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            serde_enum_attributes.rename_all = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("tag") {
                    // Parse and capture the tag attribute
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            serde_enum_attributes.tag = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("content") {
                    // Parse and ignore the content attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                } else if meta.path.is_ident("rename") {
                    // Parse and capture the rename attribute (used for enum renaming)
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            serde_enum_attributes.rename = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("deny_unknown_fields") {
                    // Handle deny_unknown_fields (no value needed)
                    // This is a flag attribute with no value
                } else if meta.path.is_ident("skip_serializing") {
                    // Handle skip_serializing
                } else if meta.path.is_ident("skip_deserializing") {
                    // Handle skip_deserializing
                } else if meta.path.is_ident("skip_serializing_if") {
                    // Handle skip_serializing_if
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<syn::Expr>();
                    }
                } else if meta.path.is_ident("default") {
                    // Handle default attribute
                    // Could have a value or be a flag
                    if meta.value().is_ok() {
                        let _ = meta.value().and_then(|v| v.parse::<syn::Expr>());
                    }
                } else if meta.path.is_ident("flatten") {
                    // Handle flatten (flag attribute)
                } else if meta.path.is_ident("untagged") {
                    // Handle untagged (flag attribute)
                } else if meta.path.is_ident("bound") {
                    // Handle bound attribute
                    if let Ok(value) = meta.value() {
                        let _ = value.parse::<Lit>();
                    }
                }
                // Silently ignore any other serde attributes to prevent parsing errors
                Ok(())
            });

            // If parsing failed, provide a more helpful error message
            if let Err(e) = parse_result {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("Failed to parse serde attribute: {}. This may be due to multiple serde attributes on separate lines. Consider consolidating them into a single #[serde(...)] attribute.", e)
                ));
            }
        }
    }

    Ok(serde_enum_attributes)
}

fn transform_variant_name(name: &str, rename_all: Option<&str>) -> String {
    match rename_all {
        Some("snake_case") => to_snake_case(name),
        Some("camelCase") => to_camel_case(name),
        Some("kebab-case") => to_kebab_case(name),
        Some("PascalCase") => name.to_string(), // No change for PascalCase
        Some("SCREAMING_SNAKE_CASE") => to_screaming_snake_case(name),
        Some("lowercase") => name.to_lowercase(),
        Some("UPPERCASE") => name.to_uppercase(),
        _ => name.to_string(), // No transformation if no rename_all or unknown pattern
    }
}

fn to_snake_case(input: &str) -> String {
    let mut result = String::new();
    let chars = input.chars();

    for ch in chars {
        if ch.is_uppercase() && !result.is_empty() {
            // Add underscore before uppercase letters (except the first character)
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result
}

fn to_camel_case(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    // First character should be lowercase
    if let Some(ch) = chars.next() {
        result.push(ch.to_lowercase().next().unwrap());
    }

    // Rest of the characters remain the same
    for ch in chars {
        result.push(ch);
    }

    result
}

fn to_kebab_case(input: &str) -> String {
    let mut result = String::new();

    for ch in input.chars() {
        if ch.is_uppercase() && !result.is_empty() {
            // Add hyphen before uppercase letters (except the first character)
            result.push('-');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result
}

fn to_screaming_snake_case(input: &str) -> String {
    let mut result = String::new();

    for ch in input.chars() {
        if ch.is_uppercase() && !result.is_empty() {
            // Add underscore before uppercase letters (except the first character)
            result.push('_');
        }
        result.push(ch.to_uppercase().next().unwrap());
    }

    result
}

fn parse_smithy_field_attributes(attrs: &[Attribute]) -> syn::Result<SmithyFieldAttributes> {
    let mut field_attributes = SmithyFieldAttributes::default();

    for attr in attrs {
        if attr.path().is_ident("smithy") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("value_type") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            field_attributes.value_type = Some(lit_str.value());
                        }
                    }
                } else if meta.path.is_ident("pattern") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            field_attributes
                                .constraints
                                .push(SmithyConstraint::Pattern(lit_str.value()));
                        }
                    }
                } else if meta.path.is_ident("range") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
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
                } else if meta.path.is_ident("length") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
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
                } else if meta.path.is_ident("required") {
                    field_attributes
                        .constraints
                        .push(SmithyConstraint::Required);
                } else if meta.path.is_ident("http_label") {
                    field_attributes
                        .constraints
                        .push(SmithyConstraint::HttpLabel);
                } else if meta.path.is_ident("http_query") {
                    if let Ok(value) = meta.value() {
                        if let Ok(Lit::Str(lit_str)) = value.parse::<Lit>() {
                            field_attributes
                                .constraints
                                .push(SmithyConstraint::HttpQuery(lit_str.value()));
                        }
                    }
                } else if meta.path.is_ident("nested_value_type") {
                    field_attributes.nested_value_type = true;
                }
                Ok(())
            })?;
        }
    }

    // Automatically add Required for http_label fields
    if field_attributes
        .constraints
        .iter()
        .any(|c| matches!(c, SmithyConstraint::HttpLabel))
        && !field_attributes
            .constraints
            .iter()
            .any(|c| matches!(c, SmithyConstraint::Required))
    {
        field_attributes
            .constraints
            .push(SmithyConstraint::Required);
    }

    Ok(field_attributes)
}

fn extract_documentation(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(meta_name_value) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta_name_value.value {
                    if let Lit::Str(lit_str) = &expr_lit.lit {
                        docs.push(lit_str.value().trim().to_string());
                    }
                }
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
            return Err(
                "Invalid range format: must be 'min..=max', '..=max', or 'min..='".to_string(),
            );
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid range min: '{}'", parts[0]))?,
            )
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse()
                    .map_err(|_| format!("Invalid range max: '{}'", parts[1]))?,
            )
        };
        Ok((min, max))
    } else if range_str.contains("..") {
        let parts: Vec<&str> = range_str.split("..").collect();
        if parts.len() != 2 {
            return Err(
                "Invalid range format: must be 'min..max', '..max', or 'min..'".to_string(),
            );
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid range min: '{}'", parts[0]))?,
            )
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse::<i64>()
                    .map_err(|_| format!("Invalid range max: '{}'", parts[1]))?
                    - 1,
            )
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
            return Err(
                "Invalid length format: must be 'min..=max', '..=max', or 'min..='".to_string(),
            );
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid length min: '{}'", parts[0]))?,
            )
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse()
                    .map_err(|_| format!("Invalid length max: '{}'", parts[1]))?,
            )
        };
        Ok((min, max))
    } else if length_str.contains("..") {
        let parts: Vec<&str> = length_str.split("..").collect();
        if parts.len() != 2 {
            return Err(
                "Invalid length format: must be 'min..max', '..max', or 'min..'".to_string(),
            );
        }
        let min = if parts[0].is_empty() {
            None
        } else {
            Some(
                parts[0]
                    .parse()
                    .map_err(|_| format!("Invalid length min: '{}'", parts[0]))?,
            )
        };
        let max = if parts[1].is_empty() {
            None
        } else {
            Some(
                parts[1]
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid length max: '{}'", parts[1]))?
                    - 1,
            )
        };
        Ok((min, max))
    } else {
        Err("Invalid length format: must contain '..' or '..='".to_string())
    }
}
