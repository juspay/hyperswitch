#![allow(clippy::expect_used, clippy::indexing_slicing)]

use common_utils::pii::SecretSerdeValue;
use hyperswitch_masking::PeekInterface;
use serde_json::json;

use super::payment_response::upsert_profile_preference;
use crate::core::payments::preferred_connector_for_profile;

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
    assert_eq!(
        value.peek()["interac"]
            .as_array()
            .expect("profile list")
            .len(),
        10
    );
    assert_eq!(
        value.peek()["interac"][0],
        json!({"pro_10": "loonio:mca_one"})
    );
    assert_eq!(
        preferred_connector_for_profile(value.peek(), "interac", "pro_0"),
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
