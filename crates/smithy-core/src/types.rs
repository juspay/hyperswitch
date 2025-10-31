// crates/smithy-core/types.rs
use api_models::customers::{CustomerUpdateRequest, CustomerResponse};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyModel {
    pub namespace: String,
    pub shapes: HashMap<String, SmithyShape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SmithyShape {
    #[serde(rename = "structure")]
    Structure {
        members: HashMap<String, SmithyMember>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "string")]
    String {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "integer")]
    Integer {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "long")]
    Long {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "boolean")]
    Boolean {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "list")]
    List {
        member: Box<SmithyMember>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "union")]
    Union {
        members: HashMap<String, SmithyMember>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
    #[serde(rename = "enum")]
    Enum {
        values: HashMap<String, SmithyEnumValue>,
        #[serde(skip_serializing_if = "Option::is_none")]
        documentation: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        traits: Vec<SmithyTrait>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyMember {
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<SmithyTrait>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmithyEnumValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "trait")]
pub enum SmithyTrait {
    #[serde(rename = "smithy.api#pattern")]
    Pattern { pattern: String },
    #[serde(rename = "smithy.api#range")]
    Range { min: Option<i64>, max: Option<i64> },
    #[serde(rename = "smithy.api#required")]
    Required,
    #[serde(rename = "smithy.api#documentation")]
    Documentation { documentation: String },
    #[serde(rename = "smithy.api#length")]
    Length { min: Option<u64>, max: Option<u64> },
    #[serde(rename = "smithy.api#httpLabel")]
    HttpLabel,
    #[serde(rename = "smithy.api#httpQuery")]
    HttpQuery { name: String },
    #[serde(rename = "smithy.api#mixin")]
    Mixin,
}

#[derive(Debug, Clone)]
pub struct SmithyField {
    pub name: String,
    pub value_type: String,
    pub constraints: Vec<SmithyConstraint>,
    pub documentation: Option<String>,
    pub optional: bool,
    pub flatten: bool,
}

#[derive(Debug, Clone)]
pub struct SmithyEnumVariant {
    pub name: String,
    pub fields: Vec<SmithyField>,
    pub constraints: Vec<SmithyConstraint>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SmithyConstraint {
    Pattern(String),
    Range(Option<i64>, Option<i64>),
    Length(Option<u64>, Option<u64>),
    Required,
    HttpLabel,
    HttpQuery(String),
}


#[derive(SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CustomerUpdateRequest {
    /// The customer's name
    #[smithy(max_length = 255, value_type = String, example = "Jon Test")]
    pub name: Option<Secret<String>>,
    /// The customer's email address
    #[smithy(value_type = String, max_length = 255, example = "JonTest@test.com")]
    pub email: Option<pii::Email>,
    /// The customer's phone number
    #[smithy(value_type = Option<String>, max_length = 255, example = "9123456789")]
    pub phone: Option<Secret<String>>,
    /// An arbitrary string that you can attach to a customer object.
    #[smithy(max_length = 255, example = "First Customer", value_type = Option<String>)]
    pub description: Option<Description>,
    /// The country code for the customer phone number
    #[smithy(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// The default billing address for the customer
    #[smithy(value_type = Option<AddressDetails>)]
    pub default_billing_address: Option<payments::AddressDetails>,
    /// The default shipping address for the customer
    #[smithy(value_type = Option<AddressDetails>)]
    pub default_shipping_address: Option<payments::AddressDetails>,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[smithy(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The unique identifier of the payment method
    #[smithy(value_type = Option<String>, example = "12345_pm_01926c58bc6e77c09e809964e72af8c8")]
    pub default_payment_method_id: Option<id_type::GlobalPaymentMethodId>,
    /// The customer's tax registration number.
    #[smithy(max_length = 255, value_type = Option<String>, example = "123456789")]
    pub tax_registration_id: Option<Secret<String>>,
}

#[derive(SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct CustomerResponse {
    /// Unique identifier for the customer
    #[smithy(
        min_length = 32,
        max_length = 64,
        example = "12345_cus_01926c58bc6e77c09e809964e72af8c8",
        value_type = String
    )]
    pub id: id_type::GlobalCustomerId,
    /// The identifier for the customer object
    #[smithy(value_type = String, max_length = 64, min_length = 1, example = "cus_y3oqhf46pyzuxjbcn2giaqnb44")]
    pub merchant_reference_id: Option<id_type::CustomerId>,
    /// Connector specific customer reference ids
    #[smithy(value_type = Option<Object>, example = json!({"mca_hwySG2NtpzX0qr7toOy8": "cus_Rnm2pDKGyQi506"}))]
    pub connector_customer_ids: Option<common_types::customers::ConnectorCustomerMap>,
    /// The customer's name
    #[smithy(max_length = 255, value_type = Option<String>, example = "Jon Test")]
    pub name: crypto::OptionalEncryptableName,
    /// The customer's email address
    #[smithy(value_type = Option<String> ,max_length = 255, example = "JonTest@test.com")]
    pub email: crypto::OptionalEncryptableEmail,
    /// The customer's phone number
    #[smithy(value_type = Option<String>,max_length = 255, example = "9123456789")]
    pub phone: crypto::OptionalEncryptablePhone,
    /// The country code for the customer phone number
    #[smithy(max_length = 255, example = "+65")]
    pub phone_country_code: Option<String>,
    /// An arbitrary string that you can attach to a customer object.
    #[smithy(max_length = 255, example = "First Customer", value_type = Option<String>)]
    pub description: Option<Description>,
    /// The default billing address for the customer
    #[schema(value_type = Option<AddressDetails>)]
    pub default_billing_address: Option<payments::AddressDetails>,
    /// The default shipping address for the customer
    #[smithy(value_type = Option<AddressDetails>)]
    pub default_shipping_address: Option<payments::AddressDetails>,
    ///  A timestamp (ISO 8601 code) that determines when the customer was created
    #[smithy(value_type = PrimitiveDateTime,example = "2023-01-18T11:04:09.922Z")]
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500
    /// characters long. Metadata is useful for storing additional, structured information on an
    /// object.
    #[smithy(value_type = Option<Object>,example = json!({ "city": "NY", "unit": "245" }))]
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The identifier for the default payment method.
    #[smithy(value_type = Option<String>, max_length = 64, example = "12345_pm_01926c58bc6e77c09e809964e72af8c8")]
    pub default_payment_method_id: Option<id_type::GlobalPaymentMethodId>,
    /// The customer's tax registration number.
    #[smithy(max_length = 255, value_type = Option<String>, example = "123456789")]
    pub tax_registration_id: crypto::OptionalEncryptableSecretString,
}

#[derive(SmithyModel)]
#[smithy(namespace = "com.hyperswitch.smithy.types")]
pub struct AddressDetails {
    #[smithy(value_type = "Option<String>", length = "1..=50")]
    pub city: Option<String>,
    
    #[smithy(value_type = "Option<String>")]
    pub country: Option<String>,  // This could be enum `CountryAlpha2`
    
    #[smithy(value_type = "Option<String>", length = "1..=200")]
    pub line1: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=50")]
    pub line2: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=50")]
    pub line3: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=50")]
    pub zip: Option<String>,
    
    #[smithy(value_type = "Option<String>")]
    pub state: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=255")]
    pub first_name: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=255")]
    pub last_name: Option<String>,
    
    #[smithy(value_type = "Option<String>", length = "1..=50")]
    pub origin_zip: Option<String>,
}

pub trait SmithyModelGenerator {
    fn generate_smithy_model() -> SmithyModel;
}
// Helper functions moved from the proc-macro crate to be accessible by it.

pub fn resolve_type_and_generate_shapes(
    value_type: &str,
    shapes: &mut HashMap<String, SmithyShape>,
) -> Result<(String, HashMap<String, SmithyShape>), syn::Error> {
    let value_type = value_type.trim();
    let value_type_span = proc_macro2::Span::call_site();
    let mut generated_shapes = HashMap::new();

    let target_type = match value_type {
        "String" | "str" => "smithy.api#String".to_string(),
        "i8" | "i16" | "i32" | "u8" | "u16" | "u32" => "smithy.api#Integer".to_string(),
        "i64" | "u64" | "isize" | "usize" => "smithy.api#Long".to_string(),
        "f32" => "smithy.api#Float".to_string(),
        "f64" => "smithy.api#Double".to_string(),
        "bool" => "smithy.api#Boolean".to_string(),
        "PrimitiveDateTime" | "time::PrimitiveDateTime" => "smithy.api#Timestamp".to_string(),
        "Amount" | "MinorUnit" => "smithy.api#Long".to_string(),
        "serde_json::Value" | "Value" | "Object" => "smithy.api#Document".to_string(),
        "Url" | "url::Url" => "smithy.api#String".to_string(),

        vt if vt.starts_with("Option<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Option")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (resolved_type, new_shapes) = resolve_type_and_generate_shapes(inner_type, shapes)?;
            generated_shapes.extend(new_shapes);
            resolved_type
        }

        vt if vt.starts_with("Vec<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Vec")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (inner_smithy_type, new_shapes) =
                resolve_type_and_generate_shapes(inner_type, shapes)?;
            generated_shapes.extend(new_shapes);

            let list_shape_name = format!(
                "{}List",
                inner_smithy_type
                    .split("::")
                    .last()
                    .unwrap_or(&inner_smithy_type)
                    .split('#')
                    .next_back()
                    .unwrap_or(&inner_smithy_type)
            );
            if !shapes.contains_key(&list_shape_name)
                && !generated_shapes.contains_key(&list_shape_name)
            {
                let list_shape = SmithyShape::List {
                    member: Box::new(SmithyMember {
                        target: inner_smithy_type,
                        documentation: None,
                        traits: vec![],
                    }),
                    traits: vec![],
                };
                generated_shapes.insert(list_shape_name.clone(), list_shape);
            }
            list_shape_name
        }

        vt if vt.starts_with("Box<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Box")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (resolved_type, new_shapes) = resolve_type_and_generate_shapes(inner_type, shapes)?;
            generated_shapes.extend(new_shapes);
            resolved_type
        }

        vt if vt.starts_with("Secret<") && vt.ends_with('>') => {
            let inner_type = extract_generic_inner_type(vt, "Secret")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (resolved_type, new_shapes) = resolve_type_and_generate_shapes(inner_type, shapes)?;
            generated_shapes.extend(new_shapes);
            resolved_type
        }

        vt if vt.starts_with("HashMap<") && vt.ends_with('>') => {
            let inner_types = extract_generic_inner_type(vt, "HashMap")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_type, value_type) =
                parse_map_types(inner_types).map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_smithy_type, key_shapes) = resolve_type_and_generate_shapes(key_type, shapes)?;
            generated_shapes.extend(key_shapes);
            let (value_smithy_type, value_shapes) =
                resolve_type_and_generate_shapes(value_type, shapes)?;
            generated_shapes.extend(value_shapes);
            format!(
                "smithy.api#Map<key: {}, value: {}>",
                key_smithy_type, value_smithy_type
            )
        }

        vt if vt.starts_with("BTreeMap<") && vt.ends_with('>') => {
            let inner_types = extract_generic_inner_type(vt, "BTreeMap")
                .map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_type, value_type) =
                parse_map_types(inner_types).map_err(|e| syn::Error::new(value_type_span, e))?;
            let (key_smithy_type, key_shapes) = resolve_type_and_generate_shapes(key_type, shapes)?;
            generated_shapes.extend(key_shapes);
            let (value_smithy_type, value_shapes) =
                resolve_type_and_generate_shapes(value_type, shapes)?;
            generated_shapes.extend(value_shapes);
            format!(
                "smithy.api#Map<key: {}, value: {}>",
                key_smithy_type, value_smithy_type
            )
        }

        _ => {
            if value_type.contains("::") {
                value_type.replace("::", ".")
            } else {
                value_type.to_string()
            }
        }
    };

    Ok((target_type, generated_shapes))
}

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

    if start_idx >= full_type.len() || end_idx > full_type.len() {
        return Err(format!(
            "Invalid index bounds for {} type: {}",
            wrapper, full_type
        ));
    }

    Ok(full_type
        .get(start_idx..end_idx)
        .ok_or_else(|| {
            format!(
                "Failed to extract inner type from {}: {}",
                wrapper, full_type
            )
        })?
        .trim())
}

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
        let key_type = inner_types
            .get(..pos)
            .ok_or_else(|| format!("Invalid key type bounds in map: {}", inner_types))?
            .trim();
        let value_type = inner_types
            .get(pos + 1..)
            .ok_or_else(|| format!("Invalid value type bounds in map: {}", inner_types))?
            .trim();

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
