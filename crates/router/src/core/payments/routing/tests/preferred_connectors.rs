use api_models::routing::{RoutableChoiceKind, RoutableConnectorChoice};
use common_utils::{id_type, pii::SecretSerdeValue};
use euclid::enums::RoutableConnectors;
use hyperswitch_masking::PeekInterface;
use serde_json::json;

use super::super::HybridRoutingStage;
use crate::core::payments::{
    operations::payment_response::upsert_profile_preference, preferred_connector_for_profile,
};

fn connector_choice(
    connector: RoutableConnectors,
    mca_id: &str,
) -> Option<RoutableConnectorChoice> {
    id_type::MerchantConnectorAccountId::wrap(mca_id.to_string())
        .ok()
        .map(|merchant_connector_id| RoutableConnectorChoice {
            choice_kind: RoutableChoiceKind::FullStruct,
            connector,
            merchant_connector_id: Some(merchant_connector_id),
        })
}

#[test]
fn preference_updates_preserve_other_profiles_and_payment_method_types() {
    let existing = SecretSerdeValue::new(json!({
        "interac": [{"pro_one": "loonio:mca_old"}, {"pro_two": "payper:mca_two"}],
        "ideal": [{"pro_one": "adyen:mca_three"}]
    }));
    let updated =
        upsert_profile_preference(Some(&existing), "interac", "pro_one", "loonio:mca_new")
            .expect("changed account must update the preference");
    assert_eq!(
        updated.peek(),
        &json!({
            "interac": [{"pro_one": "loonio:mca_new"}, {"pro_two": "payper:mca_two"}],
            "ideal": [{"pro_one": "adyen:mca_three"}]
        })
    );
    assert_eq!(
        preferred_connector_for_profile(updated.peek(), "interac", "pro_one").as_deref(),
        Some("loonio:mca_new")
    );
    assert_eq!(
        preferred_connector_for_profile(updated.peek(), "interac", "pro_two").as_deref(),
        Some("payper:mca_two")
    );
    assert_eq!(
        preferred_connector_for_profile(updated.peek(), "ideal", "pro_one").as_deref(),
        Some("adyen:mca_three")
    );
    assert_eq!(
        preferred_connector_for_profile(updated.peek(), "interac", "pro_missing"),
        None
    );
    assert_eq!(
        preferred_connector_for_profile(updated.peek(), "card", "pro_one"),
        None
    );
    assert_eq!(
        upsert_profile_preference(Some(&updated), "interac", "pro_one", "loonio:mca_new"),
        None
    );
}

#[test]
fn preference_history_is_bounded_and_newest_profile_is_first() {
    let mut value = None;
    for index in 0..11 {
        value = upsert_profile_preference(
            value.as_ref(),
            "interac",
            &format!("pro_{index}"),
            "loonio:mca_one",
        );
    }
    let value = value.expect("preferences were recorded");
    let preferences = value
        .peek()
        .get("interac")
        .and_then(|entry| entry.as_array())
        .expect("profile list");
    assert_eq!(preferences.len(), 10);
    assert_eq!(
        preferences.first(),
        Some(&json!({"pro_10": "loonio:mca_one"}))
    );
    assert_eq!(
        preferred_connector_for_profile(value.peek(), "interac", "pro_0"),
        None
    );
}

#[test]
fn repeated_success_refreshes_profile_recency_before_eviction() {
    let mut value = None;
    for index in 0..10 {
        value = upsert_profile_preference(
            value.as_ref(),
            "interac",
            &format!("pro_{index}"),
            "loonio:mca_one",
        );
    }
    let value = upsert_profile_preference(value.as_ref(), "interac", "pro_0", "loonio:mca_one")
        .expect("a repeated success refreshes an old profile");
    let value = upsert_profile_preference(Some(&value), "interac", "pro_10", "loonio:mca_one")
        .expect("a new profile is recorded");

    assert_eq!(
        preferred_connector_for_profile(value.peek(), "interac", "pro_0").as_deref(),
        Some("loonio:mca_one")
    );
    assert_eq!(
        preferred_connector_for_profile(value.peek(), "interac", "pro_1"),
        None
    );
}

#[test]
fn malformed_preferences_are_ignored_and_repaired_on_success() {
    for existing in [
        json!(null),
        json!("invalid"),
        json!({"interac": false}),
        json!({"interac": [null, {"pro_one": 42}]}),
    ] {
        assert_eq!(
            preferred_connector_for_profile(&existing, "interac", "pro_one"),
            None
        );
        let existing = SecretSerdeValue::new(existing);
        let updated =
            upsert_profile_preference(Some(&existing), "interac", "pro_one", "loonio:mca_one")
                .expect("replace invalid preference");
        assert_eq!(
            preferred_connector_for_profile(updated.peek(), "interac", "pro_one").as_deref(),
            Some("loonio:mca_one")
        );
    }
}

#[test]
fn malformed_and_duplicate_profile_entries_are_canonicalized() {
    let existing = SecretSerdeValue::new(json!({
        "interac": [
            null,
            {"pro_one": "loonio:mca_old", "pro_two": "payper:mca_two"},
            {"pro_one": "loonio:mca_new"},
            {"pro_invalid": 42}
        ],
        "ideal": [{"pro_one": "adyen:mca_three"}]
    }));
    let updated =
        upsert_profile_preference(Some(&existing), "interac", "pro_one", "loonio:mca_new")
            .expect("duplicates and malformed entries require canonicalization");

    assert_eq!(
        updated.peek(),
        &json!({
            "interac": [
                {"pro_one": "loonio:mca_new"},
                {"pro_two": "payper:mca_two"}
            ],
            "ideal": [{"pro_one": "adyen:mca_three"}]
        })
    );
}

#[test]
fn exact_preferred_account_match_keeps_mca_id_case_sensitive() {
    let connectors = [
        connector_choice(RoutableConnectors::Stripe, "mca_CaseSensitive"),
        connector_choice(RoutableConnectors::Stripe, "mca_casesensitive"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    assert_eq!(connectors.len(), 2);

    assert_eq!(
        HybridRoutingStage::resolve_preferred_connector("stripe:mca_casesensitive", &connectors,)
            .as_deref(),
        Some("stripe:mca_casesensitive")
    );
}
