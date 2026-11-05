use super::*;

#[test]
fn missing_callback_config_is_rejected() {
    let error = PaymentCallbackConfig::parse(None).expect_err("missing config");
    assert!(format!("{error:?}").contains("Required config payment_callback is missing"));
}

#[test]
fn callback_config_supports_each_rollout_mode() {
    for (value, mode) in [
        ("legacy", PaymentCallbackMode::Legacy),
        ("migration", PaymentCallbackMode::Migration),
        ("enforced", PaymentCallbackMode::Enforced),
    ] {
        let value = serde_json::json!({"mode": value});
        let config =
            PaymentCallbackConfig::parse(Some(&value.to_string())).expect("valid callback config");
        assert_eq!(config.mode, mode);
    }
}

#[test]
fn invalid_callback_config_is_rejected() {
    for value in [
        "",
        "invalid JSON",
        "null",
        "{}",
        r#"{"unrelated":"migration"}"#,
        r#"{"mode":"unknown"}"#,
        r#"{"mode":null}"#,
        r#"{"mode":1}"#,
    ] {
        assert!(
            PaymentCallbackConfig::parse(Some(value)).is_err(),
            "{value}"
        );
    }
}
