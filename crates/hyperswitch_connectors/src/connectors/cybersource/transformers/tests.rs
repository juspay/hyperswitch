use hyperswitch_masking::ExposeInterface;

use super::{CybersourceConsumerAuthValidateResponse, UcsAuthenticationData};
use crate::utils::ForeignTryFrom;

#[test]
fn test_cybersource_3ds_validate_visa_deserialization() {
    let json_data = serde_json::json!({
        "eci": "05",
        "eciRaw": "05",
        "ecommerceIndicator": "vbv",
        "cavv": "AAABBCbGYQAAAABmNkVmAAAAAAA=",
        "xid": "MDEyMzQ1Njc4OTA=",
        "directoryServerTransactionId": "80678c3b-7414-41d3-a417-1845112cb9dd",
        "specificationVersion": "2.2.0"
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize Visa 3DS payload");

    assert_eq!(response.ecommerce_indicator.as_deref(), Some("vbv"));
    assert_eq!(response.eci.as_deref(), Some("05"));
    assert_eq!(response.eci_raw.as_deref(), Some("05"));
    assert_eq!(response.get_commerce_indicator(), "vbv");
    assert_eq!(response.get_eci(), Some("05".to_string()));
    assert_eq!(
        response
            .specification_version
            .as_ref()
            .map(|v| v.to_string()),
        Some("2.2.0".to_string())
    );

    let ucs_auth_data = UcsAuthenticationData::foreign_try_from(&response)
        .expect("ForeignTryFrom into UcsAuthenticationData failed");
    assert_eq!(ucs_auth_data.eci, Some("05".to_string()));
}

#[test]
fn test_cybersource_3ds_validate_mastercard_deserialization() {
    let json_data = serde_json::json!({
        "eci": "02",
        "eciRaw": "02",
        "cavv": "mH4yBAAAAABwMRkUAAAAAAA=",
        "directoryServerTransactionId": "4c940c83-ec14-48bc-b40b-789a7fb3c706",
        "specificationVersion": "2.1.0"
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize Mastercard 3DS payload");

    assert_eq!(response.ecommerce_indicator, None);
    assert_eq!(response.eci.as_deref(), Some("02"));
    assert_eq!(response.eci_raw.as_deref(), Some("02"));
    assert_eq!(response.get_commerce_indicator(), "02");
    assert_eq!(response.get_eci(), Some("02".to_string()));

    let ucs_auth_data = UcsAuthenticationData::foreign_try_from(&response)
        .expect("ForeignTryFrom into UcsAuthenticationData failed");
    assert_eq!(ucs_auth_data.eci, Some("02".to_string()));
}

#[test]
fn test_cybersource_3ds_validate_raw_eci_only() {
    let json_data = serde_json::json!({
        "eciRaw": "05",
        "cavv": "dGVzdF9jYXZ2"
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize raw ECI payload");

    assert_eq!(response.ecommerce_indicator, None);
    assert_eq!(response.eci, None);
    assert_eq!(response.eci_raw.as_deref(), Some("05"));
    assert_eq!(response.get_commerce_indicator(), "05");
    assert_eq!(response.get_eci(), Some("05".to_string()));

    let ucs_auth_data = UcsAuthenticationData::foreign_try_from(&response)
        .expect("ForeignTryFrom into UcsAuthenticationData failed");
    assert_eq!(ucs_auth_data.eci, Some("05".to_string()));
}

#[test]
fn test_cybersource_3ds_validate_legacy_indicator_fallback() {
    let json_data = serde_json::json!({
        "indicator": "spa",
        "cavv": "bGVnYWN5X2NhdnY="
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize legacy indicator payload");

    assert_eq!(response.ecommerce_indicator, None);
    assert_eq!(response.eci, None);
    assert_eq!(response.eci_raw, None);
    assert_eq!(response.indicator.as_deref(), Some("spa"));
    assert_eq!(response.get_commerce_indicator(), "spa");
    assert_eq!(response.get_eci(), Some("spa".to_string()));

    let ucs_auth_data = UcsAuthenticationData::foreign_try_from(&response)
        .expect("ForeignTryFrom into UcsAuthenticationData failed");
    assert_eq!(ucs_auth_data.eci, Some("spa".to_string()));
}

#[test]
fn test_cybersource_3ds_validate_empty_missing_fallback_to_internet() {
    let json_data = serde_json::json!({
        "cavv": null,
        "xid": null
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize empty payload");

    assert_eq!(response.get_commerce_indicator(), "internet");
    assert_eq!(response.get_eci(), None);

    let ucs_auth_data = UcsAuthenticationData::foreign_try_from(&response)
        .expect("ForeignTryFrom into UcsAuthenticationData failed");
    assert_eq!(ucs_auth_data.eci, None);
}

#[test]
fn test_cybersource_3ds_secret_wrapping_and_masking() {
    let json_data = serde_json::json!({
        "eci": "05",
        "ecommerceIndicator": "vbv",
        "cavv": "AAABBCbGYQAAAABmNkVmAAAAAAA=",
        "directoryServerTransactionId": "80678c3b-7414-41d3-a417-1845112cb9dd"
    });

    let response: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_data).expect("Failed to deserialize 3DS payload");

    let debug_repr = format!("{response:?}");
    assert!(
        !debug_repr.contains("AAABBCbGYQAAAABmNkVmAAAAAAA="),
        "CAVV leaked in Debug format!"
    );
    assert!(
        !debug_repr.contains("80678c3b-7414-41d3-a417-1845112cb9dd"),
        "DS Transaction ID leaked in Debug format!"
    );
    assert!(
        debug_repr.contains("***"),
        "Debug representation must show masked characters"
    );

    assert_eq!(
        response.cavv.as_ref().map(|c| c.clone().expose()),
        Some("AAABBCbGYQAAAABmNkVmAAAAAAA=".to_string())
    );
    assert_eq!(
        response
            .directory_server_transaction_id
            .as_ref()
            .map(|d| d.clone().expose()),
        Some("80678c3b-7414-41d3-a417-1845112cb9dd".to_string())
    );

    let serialized = serde_json::to_string(&response).expect("Serialization failed");
    let deserialized: CybersourceConsumerAuthValidateResponse =
        serde_json::from_str(&serialized).expect("Deserialization roundtrip failed");
    assert_eq!(deserialized.get_commerce_indicator(), "vbv");
    assert_eq!(deserialized.get_eci(), Some("05".to_string()));
}

#[test]
fn test_cybersource_3ds_empty_string_filtering_and_priority_cascade() {
    // Scenario A: empty string on ecommerce_indicator falls through to eci
    let json_a = serde_json::json!({
        "ecommerceIndicator": "",
        "eci": "05",
        "eciRaw": "05"
    });
    let resp_a: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_a).expect("Deserialization failed");
    assert_eq!(resp_a.get_commerce_indicator(), "05");
    assert_eq!(resp_a.get_eci(), Some("05".to_string()));

    // Scenario B: empty strings on ecommerce_indicator and eci fall through to eci_raw
    let json_b = serde_json::json!({
        "ecommerceIndicator": "",
        "eci": "",
        "eciRaw": "02"
    });
    let resp_b: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_b).expect("Deserialization failed");
    assert_eq!(resp_b.get_commerce_indicator(), "02");
    assert_eq!(resp_b.get_eci(), Some("02".to_string()));

    // Scenario C: empty strings on all indicators fall through to legacy indicator
    let json_c = serde_json::json!({
        "ecommerceIndicator": "",
        "eci": "",
        "eciRaw": "",
        "indicator": "spa"
    });
    let resp_c: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_c).expect("Deserialization failed");
    assert_eq!(resp_c.get_commerce_indicator(), "spa");
    assert_eq!(resp_c.get_eci(), Some("spa".to_string()));

    // Scenario D: empty strings on everything fall through to "internet"
    let json_d = serde_json::json!({
        "ecommerceIndicator": "",
        "eci": "",
        "eciRaw": "",
        "indicator": ""
    });
    let resp_d: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_d).expect("Deserialization failed");
    assert_eq!(resp_d.get_commerce_indicator(), "internet");
    assert_eq!(resp_d.get_eci(), None);

    // Scenario E: Snake case and uppercase ECI aliases
    let json_e = serde_json::json!({
        "ecommerce_indicator": "vbv",
        "eci_raw": "05",
        "ECI": "05"
    });
    let resp_e: CybersourceConsumerAuthValidateResponse =
        serde_json::from_value(json_e).expect("Deserialization failed");
    assert_eq!(resp_e.ecommerce_indicator.as_deref(), Some("vbv"));
    assert_eq!(resp_e.eci_raw.as_deref(), Some("05"));
    assert_eq!(resp_e.eci.as_deref(), Some("05"));
    assert_eq!(resp_e.get_commerce_indicator(), "vbv");
}
