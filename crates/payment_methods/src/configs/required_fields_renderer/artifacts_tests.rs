#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;

fn name(value: &str) -> Name {
    value.to_owned().try_into().unwrap()
}
fn digest(value: &impl Serialize) -> Digest {
    canonical_hash(HashDomain::Bundle, value).unwrap()
}

const POLICY: &str = r#"
schema_version = 1
contract_id = "synthetic-policy-v1"
bindings = []
[connectors.synthetic]
status = "refused"
reason = "owner-review-pending"
[payment_methods.card]
status = "approved"
value = "Card"
review = "synthetic-review"
[payment_method_types.debit]
status = "approved"
value = "Debit"
review = "synthetic-review"
"#;

const CATALOG: &str = r#"
schema_version = 1
contract_id = "synthetic-catalog-v1"
[sdk]
contract_id = "synthetic-sdk-v1"
web_revision = "1111111111111111111111111111111111111111"
shared_revision = "2222222222222222222222222222222222222222"
evaluator_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
[sdk.dimensions]
payment_method = ["Card"]
[fields.synthetic]
status = "refused"
reason = "owner-review-pending"
[unrestricted_countries]
status = "refused"
reason = "catalog-review-pending"
"#;

#[test]
fn canonical_json_has_independent_expected_bytes() {
    let value = serde_json::json!({"z":true,"a":["B","A","A"]});
    // Independently calculated from the specified v1 envelope using Python hashlib.
    assert_eq!(
        serde_json::to_value(digest(&value)).unwrap(),
        "sha256:8d6fcca2c5b9a4377570b298d490afc360cb5249e938b55b0692124276e6c53f"
    );
    assert_eq!(
        canonical_bytes(&value).unwrap(),
        br#"{"a":["B","A","A"],"z":true}"#
    );
    assert_ne!(
        digest(&value),
        digest(&serde_json::json!({"z":true,"a":["A","B"]}))
    );
}

#[test]
fn nested_shuffled_maps_have_identical_bytes_and_hashes() {
    for reverse in [false, true] {
        let entries = if reverse {
            vec![("z", 2), ("a", 1)]
        } else {
            vec![("a", 1), ("z", 2)]
        };
        let map: std::collections::HashMap<_, _> = entries.into_iter().collect();
        let nested = std::collections::HashMap::from([("outer", map)]);
        assert_eq!(
            canonical_bytes(&nested).unwrap(),
            br#"{"outer":{"a":1,"z":2}}"#
        );
        assert_eq!(
            digest(&nested),
            digest(&serde_json::json!({"outer":{"a":1,"z":2}}))
        );
    }
}

#[test]
fn floats_are_not_canonical_inputs() {
    assert!(canonical_bytes(&serde_json::json!(1.5)).is_err());
}

#[test]
fn domain_separates_identical_values() {
    assert_ne!(
        canonical_hash(HashDomain::MappingPolicy, &POLICY).unwrap(),
        canonical_hash(HashDomain::FieldCatalog, &POLICY).unwrap()
    );
}

#[test]
fn strict_policy_retains_refused_choices_without_approving_them() {
    let policy: MappingPolicy = parse_contract(POLICY).unwrap();
    assert!(matches!(
        policy.connectors[&name("synthetic")]
            .approved()
            .unwrap_err()
            .current_context(),
        InputError::MappingUnresolved
    ));
    assert_eq!(
        policy.payment_methods[&name("card")].approved().unwrap(),
        &name("Card")
    );
}

#[test]
fn unknown_versions_fields_and_enum_values_refuse() {
    for input in [
        POLICY.replace("schema_version = 1", "schema_version = 2"),
        format!("unknown = true\n{POLICY}"),
        POLICY.replace(
            "reason = \"owner-review-pending\"",
            "reason = \"owner-review-pending\"\nforce = true",
        ),
        POLICY.replace("status = \"refused\"", "status = \"assumed\""),
        POLICY.replace(
            "review = \"synthetic-review\"",
            "unexpected = \"synthetic-review\"",
        ),
        POLICY.replace(
            "contract_id = \"synthetic-policy-v1\"",
            "contract_id = \" \"",
        ),
    ] {
        assert!(parse_contract::<MappingPolicy>(&input).is_err(), "{input}");
    }
}

#[test]
fn malformed_duplicates_missing_values_and_oversize_refuse() {
    assert!(parse_contract::<MappingPolicy>(&format!("schema_version = 1\n{POLICY}")).is_err());
    assert!(parse_contract::<MappingPolicy>(&POLICY.replace("bindings = []", "")).is_err());
    assert!(parse_contract::<MappingPolicy>(&" ".repeat(MAX_CONTRACT_BYTES + 1)).is_err());
    let mut policy: MappingPolicy = parse_contract(POLICY).unwrap();
    let source = SourceIdentity {
        payment_method: name("card"),
        payment_method_type: name("debit"),
        connector: name("synthetic"),
        group: RuleGroup::Common,
        path: name("outer.path"),
    };
    let binding = Binding {
        source,
        field: Decision::Refused {
            reason: name("pending"),
        },
    };
    policy.bindings = vec![binding.clone(), binding];
    assert!(policy.validate().is_err());
}

#[test]
fn catalog_is_strict_at_every_nested_boundary() {
    let catalog: FieldContractCatalog = parse_contract(CATALOG).unwrap();
    assert!(catalog.unrestricted_countries.approved().is_err());
    for input in [
        CATALOG.replace("[sdk]", "[sdk]\nunknown = true"),
        CATALOG.replace("[fields.synthetic]", "[fields.synthetic]\nunknown = true"),
        CATALOG.replace(
            "[unrestricted_countries]",
            "[unrestricted_countries]\nunknown = true",
        ),
        CATALOG.replace("schema_version = 1", "schema_version = 0"),
        CATALOG.replace(
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "tag:latest",
        ),
        CATALOG.replace("1111111111111111111111111111111111111111", "main"),
    ] {
        assert!(parse_contract::<FieldContractCatalog>(&input).is_err());
    }
    let mut value = toml::Value::try_from(&catalog).unwrap();
    value["fields"]["synthetic"] = toml::Value::try_from(Decision::Approved {
        review: name("review"),
        value: FieldContract {
            render_kind: RenderKind::Text,
            write_path: name("a.b"),
            read_path: name("a.b"),
            review: name("review"),
        },
    })
    .unwrap();
    let text = toml::to_string(&value).unwrap();
    assert!(parse_contract::<FieldContractCatalog>(&text).is_ok());
    value["fields"]["synthetic"]["value"]
        .as_table_mut()
        .unwrap()
        .insert("unknown".into(), toml::Value::Boolean(true));
    assert!(parse_contract::<FieldContractCatalog>(&toml::to_string(&value).unwrap()).is_err());
}

#[test]
fn country_catalog_is_ordered_and_never_splits_or_expands_values() {
    let mut catalog: FieldContractCatalog = parse_contract(CATALOG).unwrap();
    catalog.unrestricted_countries = Decision::Approved {
        value: vec!["GB".into(), "AT".into()],
        review: name("synthetic-review"),
    };
    catalog.validate().unwrap();
    let original = digest(&catalog);
    catalog.unrestricted_countries = Decision::Approved {
        value: vec!["AT".into(), "GB".into()],
        review: name("synthetic-review"),
    };
    assert_ne!(original, digest(&catalog));
    for values in [
        vec![],
        vec!["ALL"],
        vec!["DK,FI,NO,SE"],
        vec!["GB", "GB"],
        vec!["gb"],
    ] {
        catalog.unrestricted_countries = Decision::Approved {
            value: values.into_iter().map(str::to_owned).collect(),
            review: name("synthetic-review"),
        };
        assert!(catalog.validate().is_err());
    }
}

#[test]
fn exclusions_are_exact_scopes_and_duplicate_scopes_refuse() {
    let empty = "schema_version = 1\ncontract_id = 'synthetic-exclusions'\nsources = []";
    let exclusions: ExclusionPolicy = parse_contract(empty).unwrap();
    assert!(exclusions.sources.is_empty());
    assert!(parse_contract::<ExclusionPolicy>(&format!("force = true\n{empty}")).is_err());
    let mut policy = exclusions;
    let source = SourceIdentity {
        payment_method: name("card"),
        payment_method_type: name("debit"),
        connector: name("synthetic"),
        group: RuleGroup::Common,
        path: name("a.b"),
    };
    let exclusion = Exclusion {
        source,
        reason: name("pending"),
        review: name("review"),
    };
    policy.sources = vec![exclusion.clone(), exclusion];
    assert!(policy.validate().is_err());
}

fn release() -> ReleaseIdentity {
    ReleaseIdentity {
        application_revision: "1".repeat(40).try_into().unwrap(),
        source_tree_hash: digest(&"synthetic-tree"),
        artifact: ArtifactIdentity::Offline,
        renderer_implementation: name("foundation-v1"),
    }
}

#[test]
fn build_identity_sorts_feature_sets_and_binds_availability() {
    let a = BuildIdentity::from_feature_keys(
        release(),
        "V1,REQUIRED_FIELDS_SYNC,DUMMY_CONNECTOR",
        "V1,REQUIRED_FIELDS_SYNC",
    )
    .unwrap();
    let b = BuildIdentity::from_feature_keys(
        release(),
        "DUMMY_CONNECTOR,REQUIRED_FIELDS_SYNC,V1",
        "REQUIRED_FIELDS_SYNC,V1",
    )
    .unwrap();
    assert_eq!(digest(&a), digest(&b));
    let c = BuildIdentity::from_feature_keys(
        release(),
        "V1,REQUIRED_FIELDS_SYNC",
        "V1,REQUIRED_FIELDS_SYNC",
    )
    .unwrap();
    assert_ne!(digest(&a), digest(&c));
    let mut changed = release();
    changed.source_tree_hash = digest(&"different-tree");
    assert_ne!(
        digest(&c),
        digest(
            &BuildIdentity::from_feature_keys(
                changed,
                "V1,REQUIRED_FIELDS_SYNC",
                "V1,REQUIRED_FIELDS_SYNC"
            )
            .unwrap()
        )
    );
}

#[test]
fn unsupported_builds_refuse() {
    for features in [
        "V2,REQUIRED_FIELDS_SYNC",
        "V1,V2,REQUIRED_FIELDS_SYNC",
        "REQUIRED_FIELDS_SYNC",
        "V1",
    ] {
        assert!(
            BuildIdentity::from_feature_keys(release(), features, "V1,REQUIRED_FIELDS_SYNC")
                .is_err()
        );
        assert!(
            BuildIdentity::from_feature_keys(release(), "V1,REQUIRED_FIELDS_SYNC", features)
                .is_err()
        );
    }
}

#[test]
fn managed_values_preserve_false_empty_and_ordered_duplicates() {
    let values = [
        ManagedValue::Required(false),
        ManagedValue::Options(vec![]),
        ManagedValue::Options(vec!["B".into(), "A".into(), "A".into()]),
    ];
    assert_eq!(canonical_bytes(&values).unwrap(), br#"[{"kind":"required","value":false},{"kind":"options","value":[]},{"kind":"options","value":["B","A","A"]}]"#);
}

#[test]
fn parse_errors_do_not_echo_untrusted_input() {
    let error =
        parse_contract::<MappingPolicy>("synthetic_secret = 'never-echo-this'").unwrap_err();
    assert!(!format!("{error:?}").contains("never-echo-this"));
}

#[test]
fn managed_keys_reject_presentation_and_malformed_paths() {
    for key in [
        "dynamic_fields.card.number._is_required",
        "dynamic_fields.billing.country._options",
    ] {
        assert!(ManagedKey::try_from(key.to_owned()).is_ok());
    }
    for key in [
        "dynamic_fields.card.number._label",
        "dynamic_fields.._options",
        "dynamic_fields.card..number._options",
        "card.number._options",
        "dynamic_fields.card number._options",
    ] {
        assert!(ManagedKey::try_from(key.to_owned()).is_err());
    }
}

#[test]
fn input_contracts_and_sdk_revision_changes_invalidate_identity() {
    let policy: MappingPolicy = parse_contract(POLICY).unwrap();
    let mut changed = policy.clone();
    changed.connectors.insert(
        name("synthetic"),
        Decision::Approved {
            value: name("synthetic"),
            review: name("synthetic-review"),
        },
    );
    assert_ne!(digest(&policy), digest(&changed));
    let mut catalog: FieldContractCatalog = parse_contract(CATALOG).unwrap();
    let original = digest(&catalog);
    catalog.sdk.web_revision = "3".repeat(40).try_into().unwrap();
    assert_ne!(original, digest(&catalog));
    let mut exclusions = ExclusionPolicy {
        schema_version: SCHEMA_VERSION,
        contract_id: name("synthetic-exclusions"),
        sources: vec![],
    };
    let original = digest(&exclusions);
    exclusions.sources.push(Exclusion {
        source: SourceIdentity {
            payment_method: name("card"),
            payment_method_type: name("debit"),
            connector: name("synthetic"),
            group: RuleGroup::Common,
            path: name("a.b"),
        },
        reason: name("pending"),
        review: name("review"),
    });
    assert_ne!(original, digest(&exclusions));
}

#[test]
fn compound_context_keys_serialize_and_ordered_values_remain_in_bundle() {
    let identity = digest(&"synthetic");
    let manifest = InputManifest {
        schema_version: SCHEMA_VERSION,
        canonical_version: CANONICAL_VERSION,
        build: BuildIdentity::from_feature_keys(
            release(),
            "V1,REQUIRED_FIELDS_SYNC",
            "V1,REQUIRED_FIELDS_SYNC",
        )
        .unwrap(),
        settings_provenance: SettingsProvenance::EffectiveValuesOnly,
        effective_settings_hash: identity.clone(),
        retained_rules_hash: identity.clone(),
        mapping_hash: identity.clone(),
        catalog_hash: identity.clone(),
        exclusions_hash: identity,
    };
    let context = ContextKey {
        connector: name("synthetic"),
        payment_method: name("Card"),
        payment_method_type: name("Debit"),
        mandate_type: None,
    };
    let desired = DesiredState {
        schema_version: SCHEMA_VERSION,
        contexts: BTreeMap::from([(
            context,
            BTreeMap::from([(
                "dynamic_fields.card.number._options"
                    .to_owned()
                    .try_into()
                    .unwrap(),
                ManagedValue::Options(vec!["B".into(), "A".into(), "A".into()]),
            )]),
        )]),
        prerequisites: PrerequisiteRequirements {
            dimension_labels: BTreeMap::new(),
            field_contracts: BTreeSet::new(),
        },
        input_manifest: manifest,
        diagnostics: vec![],
    };
    let bytes = canonical_bytes(&desired).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        parsed["contexts"][0][1]["dynamic_fields.card.number._options"]["value"],
        serde_json::json!(["B", "A", "A"])
    );
    assert_eq!(
        parsed["contexts"][0][0]["mandate_type"],
        serde_json::Value::Null
    );
    assert!(!String::from_utf8(bytes).unwrap().contains("timestamp"));
}
