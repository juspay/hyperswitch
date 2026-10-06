use error_stack::Result;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Failed to serialize payment link data")]
    SerializationFailed,
}

/// Convert snake_case to camelCase
fn camel_case_key(key: &str) -> String {
    let mut out = String::new();
    let mut uppercase = false;

    for c in key.chars() {
        if c == '_' {
            uppercase = true;
        } else if uppercase {
            out.push(c.to_ascii_uppercase());
            uppercase = false;
        } else {
            out.push(c);
        }
    }

    out
}

/// Convert JSON keys to camelCase
fn camel_case_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                if let Some(mut v) = map.remove(&k) {
                    camel_case_json(&mut v);
                    map.insert(camel_case_key(&k), v);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                camel_case_json(v);
            }
        }
        _ => {}
    }
}

/// Only convert the `custom_message_for_payment_method_types` field to camelCase
pub fn convert_custom_message_keys_to_camel(value: &mut Value) {
    if let Some(test_mode) = value
        .as_object_mut()
        .and_then(|map| map.remove("test_mode"))
    {
        if let Some(map) = value.as_object_mut() {
            map.insert("isTestMode".to_string(), test_mode);
        }
    }

    // Rename the outer key to `preloadSDKWithParams` and camelCase all of its
    // direct children. Their values are left untouched, since the SDK expects
    // those nested contents verbatim (snake_case).
    if let Some(mut preload_params) = value
        .as_object_mut()
        .and_then(|map| map.remove("preload_sdk_with_params"))
    {
        if let Some(inner) = preload_params.as_object_mut() {
            let keys: Vec<String> = inner.keys().cloned().collect();
            for k in keys {
                if let Some(v) = inner.remove(&k) {
                    inner.insert(camel_case_key(&k), v);
                }
            }
        }
        if let Some(map) = value.as_object_mut() {
            map.insert("preloadSDKWithParams".to_string(), preload_params);
        }
    }

    if let Some(custom_msg) = value.get_mut("custom_message_for_payment_method_types") {
        camel_case_json(custom_msg);
    }
}

pub fn get_js_script<T>(payment_details: &T) -> Result<String, PaymentLinkError>
where
    T: serde::Serialize,
{
    let mut json =
        serde_json::to_value(payment_details).map_err(|_| PaymentLinkError::SerializationFailed)?;

    // Apply camelCase only on the custom_message_for_payment_method_types field
    convert_custom_message_keys_to_camel(&mut json);

    let payment_details_str =
        serde_json::to_string(&json).map_err(|_| PaymentLinkError::SerializationFailed)?;
    let url_encoded_str = urlencoding::encode(&payment_details_str);

    Ok(format!("window.__PAYMENT_DETAILS = '{}';", url_encoded_str))
}
