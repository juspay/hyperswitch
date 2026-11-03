//! The ACI settings a merchant connector account carries in its metadata.
//!
//! Every setting is optional and read leniently: a value that is missing, mistyped or
//! unrecognised leaves the setting at its default rather than failing the payment, so a typo in
//! account configuration degrades to the connector's default behaviour instead of an outage.
//! (`collect_cvv` is shared with core and read through `should_collect_cvv_from_mca_metadata`.)

use hyperswitch_masking::{PeekInterface, Secret};
use serde_json::Value;

use super::{
    acquirer::AcquirerKind,
    request::{AciTestMode, AciTransactionCategory},
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(super) struct AciMetadata {
    /// `transactionCategory`: the channel the entity is boarded for. Outranks the request's
    /// `payment_channel`, see [`super::payment::Channel`].
    pub(super) transaction_category: Option<AciTransactionCategory>,
    /// `test_mode`: `EXTERNAL` routes to the acquirer's test system and requires entity-side
    /// configuration; sending it to a plain test entity is rejected with 900.100.100. Absent lets
    /// ACI apply its own default (INTERNAL) for test entities.
    pub(super) test_mode: Option<AciTestMode>,
    /// `trim_merchant_transaction_id`: opt in to the 16-character `merchantTransactionId` limit
    /// some acquirers enforce. Not universal, so never applied by default.
    pub(super) trim_merchant_transaction_id: bool,
    /// `acquirer`: the downstream acquirer, declared at onboarding. The only acquirer signal a
    /// request has, and the most trusted one a response has.
    pub(super) acquirer: Option<AcquirerKind>,
}

impl AciMetadata {
    pub(super) fn from_connector_meta(connector_meta_data: Option<&Secret<Value>>) -> Self {
        connector_meta_data
            .map(|meta| Self::from_value(meta.peek()))
            .unwrap_or_default()
    }

    /// Only the declared acquirer, read without the other settings — a response needs nothing
    /// else, and must not re-run their validation (and its warnings) on every payment.
    pub(super) fn declared_acquirer(
        connector_meta_data: Option<&Secret<Value>>,
    ) -> Option<AcquirerKind> {
        connector_meta_data
            .and_then(|meta| meta.peek().get("acquirer")?.as_str())
            .and_then(AcquirerKind::from_name)
    }

    fn from_value(meta: &Value) -> Self {
        let text = |key: &str| meta.get(key).and_then(Value::as_str);
        Self {
            transaction_category: text("transactionCategory").and_then(transaction_category),
            test_mode: text("test_mode").and_then(test_mode),
            trim_merchant_transaction_id: meta
                .get("trim_merchant_transaction_id")
                .is_some_and(is_enabled),
            acquirer: text("acquirer").and_then(AcquirerKind::from_name),
        }
    }
}

/// The documented OPPWA codes (`EC`/`MO`/`TO`/`PO`/`PM`/`MOTO`), case-insensitive. Anything else
/// is ignored with a warning, so a typo omits the field rather than failing ACI validation.
fn transaction_category(raw: &str) -> Option<AciTransactionCategory> {
    let code = raw.trim().to_ascii_uppercase();
    match code.as_str() {
        "EC" => Some(AciTransactionCategory::Ecommerce),
        "MO" => Some(AciTransactionCategory::MailOrder),
        "TO" => Some(AciTransactionCategory::TelephoneOrder),
        "PO" => Some(AciTransactionCategory::PointOfSale),
        "PM" => Some(AciTransactionCategory::MobilePointOfSale),
        "MOTO" => Some(AciTransactionCategory::MailOrTelephoneOrder),
        other => {
            router_env::logger::warn!(
                "Ignoring unrecognised transactionCategory {other:?} in ACI MCA metadata"
            );
            None
        }
    }
}

fn test_mode(raw: &str) -> Option<AciTestMode> {
    match raw.to_uppercase().as_str() {
        "EXTERNAL" => Some(AciTestMode::External),
        "INTERNAL" => Some(AciTestMode::Internal),
        _ => None,
    }
}

/// A JSON `true`, or the string `"true"` in any case.
fn is_enabled(value: &Value) -> bool {
    value
        .as_bool()
        .or_else(|| value.as_str().map(|s| s.eq_ignore_ascii_case("true")))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(value: Value) -> AciMetadata {
        AciMetadata::from_connector_meta(Some(&Secret::new(value)))
    }

    /// A response reads only the declared acquirer, and reads it exactly as the full parse does.
    #[test]
    fn declared_acquirer_agrees_with_the_full_parse() {
        for value in [
            serde_json::json!({ "acquirer": "Nedbank", "transactionCategory": "bogus" }),
            serde_json::json!({ "acquirer": "  ABSA bank " }),
            serde_json::json!({ "acquirer": 7 }),
            serde_json::json!({}),
            serde_json::json!("not an object"),
        ] {
            let meta = Secret::new(value);
            assert_eq!(
                AciMetadata::declared_acquirer(Some(&meta)),
                AciMetadata::from_connector_meta(Some(&meta)).acquirer
            );
        }
        assert_eq!(AciMetadata::declared_acquirer(None), None);
    }

    #[test]
    fn absent_metadata_is_all_defaults() {
        assert_eq!(
            AciMetadata::from_connector_meta(None),
            AciMetadata::default()
        );
        assert_eq!(metadata(serde_json::json!({})), AciMetadata::default());
        assert_eq!(
            metadata(serde_json::json!("not an object")),
            AciMetadata::default()
        );
    }

    /// Accepts the documented codes in any case and with surrounding whitespace; an unknown code
    /// is dropped rather than forwarded for ACI to reject.
    #[test]
    fn transaction_category_is_read_from_mca_metadata() {
        let category = |value: Value| {
            metadata(serde_json::json!({ "transactionCategory": value })).transaction_category
        };

        assert_eq!(
            category(serde_json::json!("EC")),
            Some(AciTransactionCategory::Ecommerce)
        );
        assert_eq!(
            category(serde_json::json!(" moto ")),
            Some(AciTransactionCategory::MailOrTelephoneOrder)
        );
        assert_eq!(
            category(serde_json::json!("mo")),
            Some(AciTransactionCategory::MailOrder)
        );
        assert_eq!(
            category(serde_json::json!("To")),
            Some(AciTransactionCategory::TelephoneOrder)
        );
        assert_eq!(
            category(serde_json::json!("po")),
            Some(AciTransactionCategory::PointOfSale)
        );
        assert_eq!(
            category(serde_json::json!("PM")),
            Some(AciTransactionCategory::MobilePointOfSale)
        );
        assert_eq!(category(serde_json::json!("XX")), None);
        assert_eq!(category(serde_json::json!(7)), None);
    }

    #[test]
    fn test_mode_is_case_insensitive_and_closed() {
        let mode = |value: Value| metadata(serde_json::json!({ "test_mode": value })).test_mode;

        assert!(matches!(
            mode(serde_json::json!("external")),
            Some(AciTestMode::External)
        ));
        assert!(matches!(
            mode(serde_json::json!("INTERNAL")),
            Some(AciTestMode::Internal)
        ));
        assert!(mode(serde_json::json!(" EXTERNAL")).is_none());
        assert!(mode(serde_json::json!(true)).is_none());
    }

    #[test]
    fn merchant_transaction_id_trim_accepts_a_bool_or_the_string_true() {
        let trim = |value: Value| {
            metadata(serde_json::json!({ "trim_merchant_transaction_id": value }))
                .trim_merchant_transaction_id
        };

        assert!(trim(serde_json::json!(true)));
        assert!(trim(serde_json::json!("TRUE")));
        assert!(!trim(serde_json::json!(false)));
        assert!(!trim(serde_json::json!("yes")));
        assert!(!trim(serde_json::json!(1)));
    }

    #[test]
    fn acquirer_is_matched_by_name() {
        let acquirer = |value: &str| metadata(serde_json::json!({ "acquirer": value })).acquirer;

        assert_eq!(acquirer("Nedbank"), Some(AcquirerKind::Nedbank));
        assert_eq!(acquirer("EMC_FNB"), Some(AcquirerKind::Cybersource));
        assert_eq!(acquirer("somebank"), None);
    }
}
