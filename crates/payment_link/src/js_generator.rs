use error_stack::Result;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum PaymentLinkError {
    #[error("Failed to serialize payment link data")]
    SerializationFailed,
}

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

pub fn convert_custom_message_keys_to_camel(value: &mut Value) {
    if let Some(test_mode) = value
        .as_object_mut()
        .and_then(|map| map.remove("test_mode"))
    {
        if let Some(map) = value.as_object_mut() {
            map.insert("isTestMode".to_string(), test_mode);
        }
    }

    if let Some(mut preload_params) = value
        .as_object_mut()
        .and_then(|map| map.remove("preload_sdk_with_params"))
    {
        camel_case_json(&mut preload_params);
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

    convert_custom_message_keys_to_camel(&mut json);

    let payment_details_str =
        serde_json::to_string(&json).map_err(|_| PaymentLinkError::SerializationFailed)?;
    let url_encoded_str = urlencoding::encode(&payment_details_str);

    Ok(format!("window.__PAYMENT_DETAILS = '{}';", url_encoded_str))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_camel_case_key_conversion() {
        assert_eq!(camel_case_key("test_mode"), "testMode");
        assert_eq!(
            camel_case_key("preload_sdk_with_params"),
            "preloadSDKWithParams"
        );
        assert_eq!(camel_case_key("payment_methods_list"), "paymentMethodsList");
        assert_eq!(camel_case_key("session_tokens"), "sessionTokens");
        assert_eq!(camel_case_key("custom_message"), "customMessage");
    }

    #[test]
    fn test_test_mode_converted_to_is_test_mode() {
        let mut value = json!({
            "test_mode": true,
            "client_secret": "secret_123"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(value.get("isTestMode").is_some());
        assert!(value.get("test_mode").is_none());
        assert_eq!(value["isTestMode"], json!(true));
    }

    #[test]
    fn test_preload_sdk_with_params_converted_to_preload_sdk_with_params() {
        let mut value = json!({
            "preload_sdk_with_params": {
                "payment_methods_list": ["card", "wallet"]
            },
            "client_secret": "secret_123"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(value.get("preloadSDKWithParams").is_some());
        assert!(value.get("preload_sdk_with_params").is_none());
    }

    #[test]
    fn test_nested_keys_in_preload_sdk_with_params_are_converted() {
        let mut value = json!({
            "preload_sdk_with_params": {
                "payment_methods_list": ["card", "wallet"],
                "customer_methods_list": ["upi", "bank_transfer"],
                "session_tokens": ["token1", "token2"],
                "blocked_bins": ["411111", "555555"]
            }
        });

        convert_custom_message_keys_to_camel(&mut value);

        let preload = &value["preloadSDKWithParams"];
        assert!(preload.get("paymentMethodsList").is_some());
        assert!(preload.get("payment_methods_list").is_none());
        assert!(preload.get("customerMethodsList").is_some());
        assert!(preload.get("customer_methods_list").is_none());
        assert!(preload.get("sessionTokens").is_some());
        assert!(preload.get("session_tokens").is_none());
        assert!(preload.get("blockedBins").is_some());
        assert!(preload.get("blocked_bins").is_none());
    }

    #[test]
    fn test_custom_message_for_payment_method_types_still_works() {
        let mut value = json!({
            "custom_message_for_payment_method_types": {
                "card": {
                    "message_for_customer": "Please enter your card details",
                    "warning_message": "Test mode enabled"
                }
            }
        });

        convert_custom_message_keys_to_camel(&mut value);

        let custom_msg = &value["custom_message_for_payment_method_types"]["card"];
        assert!(custom_msg.get("messageForCustomer").is_some());
        assert!(custom_msg.get("message_for_customer").is_none());
        assert!(custom_msg.get("warningMessage").is_some());
        assert!(custom_msg.get("warning_message").is_none());
    }

    #[test]
    fn test_all_conversions_together() {
        let mut value = json!({
            "test_mode": false,
            "preload_sdk_with_params": {
                "payment_methods_list": ["card"]
            },
            "custom_message_for_payment_method_types": {
                "wallet": {
                    "warning_message": "Use wallet"
                }
            },
            "client_secret": "secret_xyz"
        });

        convert_custom_message_keys_to_camel(&mut value);

        // Check test_mode -> isTestMode
        assert!(value.get("isTestMode").is_some());
        assert!(value.get("test_mode").is_none());
        assert_eq!(value["isTestMode"], json!(false));

        // Check preload_sdk_with_params -> preloadSDKWithParams
        assert!(value.get("preloadSDKWithParams").is_some());
        assert!(value.get("preload_sdk_with_params").is_none());

        // Check nested key in preload
        assert!(value["preloadSDKWithParams"]
            .get("paymentMethodsList")
            .is_some());

        // Check custom_message_for_payment_method_types nested keys
        let wallet_msg = &value["custom_message_for_payment_method_types"]["wallet"];
        assert!(wallet_msg.get("warningMessage").is_some());
        assert!(wallet_msg.get("warning_message").is_none());

        // Check other fields remain unchanged
        assert!(value.get("client_secret").is_some());
    }
}
