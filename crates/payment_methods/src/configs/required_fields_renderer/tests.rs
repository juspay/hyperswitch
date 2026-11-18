#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::collections::{HashMap, HashSet};

use api_models::{
    enums::{BankNames, Connector, PaymentMethod, PaymentMethodType},
    payment_methods::RequiredFieldInfo,
};

use super::*;
use crate::configs::settings::{
    BanksVector, ConnectorBankNames, ConnectorFields, RequiredFieldFinal,
};

fn banks(values: &[BankNames]) -> BankRedirectConfig {
    BankRedirectConfig(HashMap::from([(
        PaymentMethodType::Eps,
        ConnectorBankNames(HashMap::from([(
            "stripe".to_owned(),
            BanksVector {
                banks: values.iter().copied().collect::<HashSet<_>>(),
            },
        )])),
    )]))
}

fn input(config: &BankRedirectConfig) -> EffectiveInputs {
    EffectiveInputs::capture(&RequiredFields::new(config), config).unwrap()
}

fn synthetic(field_type: FieldType) -> RequiredFields {
    RequiredFields(HashMap::from([(
        PaymentMethod::Card,
        crate::configs::settings::PaymentMethodType(HashMap::from([(
            PaymentMethodType::Debit,
            ConnectorFields {
                fields: HashMap::from([(
                    Connector::Stripe,
                    RequiredFieldFinal {
                        common: HashMap::from([(
                            "outer.synthetic".to_owned(),
                            RequiredFieldInfo {
                                required_field: "inner.synthetic".to_owned(),
                                display_name: "Synthetic label".to_owned(),
                                field_type,
                                value: None,
                            },
                        )]),
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                    },
                )]),
            },
        )])),
    )]))
}

#[test]
fn repeated_real_constructor_maps_and_hashsets_normalize_retained_inputs() {
    let expected = input(&banks(&[BankNames::BankAustria, BankNames::AbnAmro]));
    for round in 0..20 {
        let values = if round % 2 == 0 {
            [BankNames::AbnAmro, BankNames::BankAustria]
        } else {
            [BankNames::BankAustria, BankNames::AbnAmro]
        };
        let actual = input(&banks(&values));
        assert_eq!(
            canonical_bytes(&expected).unwrap(),
            canonical_bytes(&actual).unwrap()
        );
        assert_eq!(
            expected.retained_rules_hash().unwrap(),
            actual.retained_rules_hash().unwrap()
        );
    }
}

#[test]
fn only_effective_constructor_inputs_change_settings_and_expected_rule() {
    let base = banks(&[BankNames::BankAustria]);
    let a = input(&base);
    let b = input(&banks(&[BankNames::AbnAmro]));
    assert_ne!(a.settings_hash().unwrap(), b.settings_hash().unwrap());
    assert_ne!(
        a.retained_rules_hash().unwrap(),
        b.retained_rules_hash().unwrap()
    );
    let changed: Vec<_> = a
        .rules()
        .iter()
        .zip(b.rules())
        .filter(|(a, b)| canonical_bytes(a).unwrap() != canonical_bytes(b).unwrap())
        .collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0.source.path.as_str(), EPS_PATH);
    let mut unrelated = base;
    unrelated.0.insert(
        PaymentMethodType::Ideal,
        ConnectorBankNames(HashMap::from([(
            "stripe".to_owned(),
            BanksVector {
                banks: HashSet::from([BankNames::AbnAmro]),
            },
        )])),
    );
    assert_eq!(
        canonical_bytes(&a).unwrap(),
        canonical_bytes(&input(&unrelated)).unwrap()
    );
}

#[test]
fn absent_empty_and_populated_settings_are_distinct() {
    let absent = input(&BankRedirectConfig::default());
    let empty = input(&banks(&[]));
    let populated = input(&banks(&[BankNames::BankAustria]));
    assert_ne!(
        absent.settings_hash().unwrap(),
        empty.settings_hash().unwrap()
    );
    assert_ne!(
        empty.settings_hash().unwrap(),
        populated.settings_hash().unwrap()
    );
    assert_eq!(
        absent.retained_rules_hash().unwrap(),
        empty.retained_rules_hash().unwrap()
    );
    // Empty required-select refusal is a renderer gate (Step 4), not input capture success.
}

#[test]
fn ordered_bank_vectors_outside_exact_eps_provenance_keep_order_and_duplicates() {
    let fields = synthetic(FieldType::UserBankOptions {
        options: vec!["B".into(), "A".into(), "A".into()],
    });
    let captured = EffectiveInputs::capture(&fields, &BankRedirectConfig::default()).unwrap();
    assert_eq!(
        field_options(&captured.rules()[0].field_type).unwrap(),
        ["B", "A", "A"]
    );
    let reordered = synthetic(FieldType::UserBankOptions {
        options: vec!["A".into(), "B".into(), "A".into()],
    });
    assert_ne!(
        captured.retained_rules_hash().unwrap(),
        EffectiveInputs::capture(&reordered, &BankRedirectConfig::default())
            .unwrap()
            .retained_rules_hash()
            .unwrap()
    );
}

#[test]
fn prefill_is_never_retained_and_outer_inner_paths_remain_distinct() {
    let mut fields = synthetic(FieldType::Text);
    let before = EffectiveInputs::capture(&fields, &BankRedirectConfig::default()).unwrap();
    let field = fields
        .0
        .get_mut(&PaymentMethod::Card)
        .unwrap()
        .0
        .get_mut(&PaymentMethodType::Debit)
        .unwrap()
        .fields
        .get_mut(&Connector::Stripe)
        .unwrap()
        .common
        .get_mut("outer.synthetic")
        .unwrap();
    field.value = Some(hyperswitch_masking::Secret::new(
        "synthetic-sensitive-prefill".to_owned(),
    ));
    let after = EffectiveInputs::capture(&fields, &BankRedirectConfig::default()).unwrap();
    assert_eq!(
        canonical_bytes(&before).unwrap(),
        canonical_bytes(&after).unwrap()
    );
    let text = String::from_utf8(canonical_bytes(&after).unwrap()).unwrap();
    assert!(!text.contains("prefill") && !text.contains("\"value\""));
    assert_eq!(after.rules()[0].source.path.as_str(), "outer.synthetic");
    assert_eq!(after.rules()[0].required_field.as_str(), "inner.synthetic");
}

#[test]
fn custom_field_equality_cannot_hide_changed_country_options() {
    let a = FieldType::UserAddressCountry {
        options: vec!["AT".into()],
    };
    let b = FieldType::UserAddressCountry {
        options: vec!["GB".into()],
    };
    assert_eq!(a, b); // Existing API equality deliberately ignores these options.
    let a = EffectiveInputs::capture(&synthetic(a), &BankRedirectConfig::default()).unwrap();
    let b = EffectiveInputs::capture(&synthetic(b), &BankRedirectConfig::default()).unwrap();
    assert_ne!(
        a.retained_rules_hash().unwrap(),
        b.retained_rules_hash().unwrap()
    );
}

#[test]
fn stale_eps_settings_and_duplicate_options_refuse_normalization() {
    let settings = banks(&[BankNames::BankAustria]);
    let mut fields = RequiredFields::new(&settings);
    assert!(EffectiveInputs::capture(&fields, &banks(&[BankNames::AbnAmro])).is_err());
    let field = fields
        .0
        .get_mut(&PaymentMethod::BankRedirect)
        .unwrap()
        .0
        .get_mut(&PaymentMethodType::Eps)
        .unwrap()
        .fields
        .get_mut(&Connector::Stripe)
        .unwrap()
        .non_mandate
        .get_mut(EPS_PATH)
        .unwrap();
    field.field_type = FieldType::UserBankOptions {
        options: vec!["bank_austria".into(), "bank_austria".into()],
    };
    assert!(EffectiveInputs::capture(&fields, &settings).is_err());
}

#[test]
fn all_option_variants_retain_exact_vectors() {
    let options = vec!["B".into(), "A".into(), "A".into()];
    for field in [
        FieldType::UserCountry {
            options: options.clone(),
        },
        FieldType::UserCurrency {
            options: options.clone(),
        },
        FieldType::UserAddressCountry {
            options: options.clone(),
        },
        FieldType::UserShippingAddressCountry {
            options: options.clone(),
        },
        FieldType::UserDocumentType {
            options: options.clone(),
        },
        FieldType::UserBankOptions {
            options: options.clone(),
        },
        FieldType::DropDown {
            options: options.clone(),
        },
        FieldType::LanguagePreference {
            options: options.clone(),
        },
        FieldType::UserBankType {
            options: options.clone(),
        },
    ] {
        assert_eq!(field_options(&field).unwrap(), options);
    }
    assert!(field_options(&FieldType::Text).is_none());
}

#[test]
fn source_vocabulary_is_the_actual_compiled_enum_contract() {
    let text = "schema_version = 1\ncontract_id = 'synthetic'\nbindings = []\n[connectors.stripe]\nstatus = 'refused'\nreason = 'pending'\n[payment_methods.card]\nstatus = 'refused'\nreason = 'pending'\n[payment_method_types.debit]\nstatus = 'refused'\nreason = 'pending'";
    assert!(parse_mapping_policy(text).is_ok());
    for wrong in [
        text.replace("connectors.stripe", "connectors.unknown_connector"),
        text.replace("methods.card", "methods.Card"),
        text.replace("types.debit", "types.unknown_type"),
    ] {
        assert!(parse_mapping_policy(&wrong).is_err());
    }
}

#[test]
fn embedded_features_match_this_compilation() {
    let keys: std::collections::BTreeSet<_> = compiled_feature_keys().split(',').collect();
    assert!(keys.contains("V1") && keys.contains("REQUIRED_FIELDS_SYNC"));
    assert!(!keys.contains("V2"));
    assert_eq!(
        keys.contains("DUMMY_CONNECTOR"),
        cfg!(feature = "dummy_connector")
    );
}
