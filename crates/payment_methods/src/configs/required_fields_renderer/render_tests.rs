#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::too_many_arguments
)]

use std::collections::{BTreeMap, HashMap};

use api_models::{
    enums::{BankNames, Connector, PaymentMethod, PaymentMethodType},
    payment_methods::RequiredFieldInfo,
};
use serde_json::json;

use super::*;
use crate::configs::settings::{
    BanksVector, ConnectorBankNames, ConnectorFields, RequiredFieldFinal,
};

fn name(s: &str) -> Name {
    s.to_owned().try_into().unwrap()
}
fn approved<T>(value: T) -> Decision<T> {
    Decision::Approved {
        value,
        review: name("synthetic-test-only"),
    }
}
fn values(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| (*s).to_owned()).collect()
}

fn build() -> BuildIdentity {
    BuildIdentity::for_offline_renderer(
        ReleaseIdentity {
            application_revision: "1".repeat(40).try_into().unwrap(),
            source_tree_hash: canonical_hash(HashDomain::Bundle, &"synthetic-render-tests")
                .unwrap(),
            artifact: ArtifactIdentity::Offline,
            renderer_implementation: name("renderer-v2-synthetic"),
        },
        compiled_feature_keys(),
    )
    .unwrap()
}

/// Explicit all-excluded fixtures exercise CLI plumbing, never business-profile acceptance.
/// Writes only below a separately registered disposable root; normal tests do not run this.
#[test]
#[ignore = "requires RF_CLI_FIXTURE_ROOT ownership registration"]
fn export_router_cli_profiles() {
    use std::io::Write as _;
    let root = std::path::PathBuf::from(std::env::var("RF_CLI_FIXTURE_ROOT").unwrap());
    let owner: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert!(root.is_absolute());
    assert_eq!(owner["root"], json!(root));
    assert_eq!(owner["cleanup_scope"], json!(root));
    assert_eq!(owner["purpose"], "Step 7 offline CLI checks");
    let banks: BankRedirectConfig =
        serde_json::from_value(json!({"eps":{"stripe":{"banks":"abn_amro"}}})).unwrap();
    let inputs = EffectiveInputs::capture(&RequiredFields::new(&banks), &banks).unwrap();
    let mut fixture = Fixture::new();
    fixture.exclusions.sources = inputs
        .rules()
        .iter()
        .map(|rule| Exclusion {
            source: rule.source.clone(),
            reason: name("synthetic-cli-all-excluded-not-adoption"),
            review: name("synthetic-cli-only"),
        })
        .collect();
    let release = serde_json::to_value(build()).unwrap()["release"].clone();
    let seed = json!({"schema_version":1,"profile_id":"synthetic-cli-seed-all-excluded","inputs":{"kind":"seed","stripe_eps_banks":["abn_amro"]},"release":release,"policy":fixture.policy,"catalog":fixture.catalog,"exclusions":fixture.exclusions});
    let mut effective = seed.clone();
    effective["profile_id"] = json!("synthetic-cli-effective-all-excluded");
    effective["inputs"] = json!({"kind":"effective_settings","settings_file":"unavailable-services.toml","expected_settings_hash":inputs.settings_hash().unwrap()});
    let mut blocked = seed.clone();
    blocked["exclusions"]["sources"] = json!([]);
    for (name, value) in [
        ("seed-profile.json", seed),
        ("effective-profile.json", effective),
        ("blocked-profile.json", blocked),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(root.join(name))
            .unwrap();
        file.write_all(&serde_json::to_vec(&value).unwrap())
            .unwrap();
    }
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join("unavailable-services.toml"))
        .unwrap();
    file.write_all(b"[bank_config.eps.stripe]\nbanks = 'abn_amro'\n[master_database]\nhost = '127.0.0.1'\nport = 'intentionally-invalid-unrelated-setting'\n[redis]\nhost = '127.0.0.1'\nport = 1\n[secrets]\njwt_secret = 'synthetic-sensitive-never-log'\n").unwrap();
    assert!(inputs.rules().len() > 1000);
}

struct Fixture {
    rules: RequiredFields,
    banks: BankRedirectConfig,
    policy: MappingPolicy,
    catalog: FieldContractCatalog,
    exclusions: ExclusionPolicy,
}

impl Fixture {
    fn new() -> Self {
        // These are independent literal SDK expectations, not calls to the renderer's vocabulary.
        let methods = [
            ("card", "Card"),
            ("bank_redirect", "BankRedirect"),
            ("bank_debit", "BankDebit"),
            ("wallet", "Wallet"),
            ("gift_card", "GiftCard"),
        ];
        let kinds = [
            ("debit", "Debit"),
            ("credit", "Credit"),
            ("ach", "Ach"),
            ("eps", "Eps"),
            ("paypal", "Paypal"),
            ("givex", "Givex"),
            (
                "online_banking_czech_republic",
                "OnlineBankingCzechRepublic",
            ),
            ("online_banking_slovakia", "OnlineBankingSlovakia"),
            ("online_banking_fpx", "OnlineBankingFpx"),
            ("online_banking_thailand", "OnlineBankingThailand"),
            ("open_banking_uk", "OpenBankingUk"),
        ];
        Self {
            rules: RequiredFields(HashMap::new()),
            banks: BankRedirectConfig::default(),
            policy: MappingPolicy {
                schema_version: SCHEMA_VERSION,
                contract_id: name("synthetic-policy"),
                connectors: ["adyen", "stripe", "getnet"]
                    .map(|v| (name(v), approved(name(v))))
                    .into(),
                payment_methods: methods.map(|(k, v)| (name(k), approved(name(v)))).into(),
                payment_method_types: kinds.map(|(k, v)| (name(k), approved(name(v)))).into(),
                bindings: vec![],
            },
            catalog: FieldContractCatalog {
                schema_version: SCHEMA_VERSION,
                contract_id: name("synthetic-catalog"),
                sdk: SdkContract {
                    contract_id: name("pinned-local-sdk-synthetic-cases"),
                    web_revision: "939f45786aee1c3b14e3f0b134bb42cb2cf2dd39"
                        .to_owned()
                        .try_into()
                        .unwrap(),
                    shared_revision: "1af3e5dabf9d5a17dd864ac7312151ebac4367c6"
                        .to_owned()
                        .try_into()
                        .unwrap(),
                    evaluator_hash:
                        "sha256:5db572c0540ecf067beb78537f69215d9d0cc538e28b8bdd44685fcef1f91a19"
                            .to_owned()
                            .try_into()
                            .unwrap(),
                    dimensions: BTreeMap::from([
                        (
                            name("connector"),
                            vec![name("adyen"), name("stripe"), name("getnet")],
                        ),
                        (
                            name("payment_method"),
                            methods.map(|(_, v)| name(v)).to_vec(),
                        ),
                        (
                            name("payment_method_type"),
                            kinds.map(|(_, v)| name(v)).to_vec(),
                        ),
                        (
                            name("mandate_type"),
                            vec![name("mandate"), name("non_mandate")],
                        ),
                    ]),
                },
                fields: BTreeMap::new(),
                unrestricted_countries: approved(values(&["GB", "AT", "US", "DE"])),
            },
            exclusions: ExclusionPolicy {
                schema_version: SCHEMA_VERSION,
                contract_id: name("synthetic-exclusions"),
                sources: vec![],
            },
        }
    }

    fn add(
        &mut self,
        scope: (&str, &str, &str, RuleGroup),
        outer: &str,
        inner: &str,
        field_type: FieldType,
        field_id: &str,
        kind: RenderKind,
        source_types: &[&str],
    ) {
        let (method, method_type, connector, group) = scope;
        let types = self
            .rules
            .0
            .entry(method.parse().unwrap())
            .or_insert_with(|| crate::configs::settings::PaymentMethodType(HashMap::new()));
        let connectors = types
            .0
            .entry(method_type.parse().unwrap())
            .or_insert_with(|| ConnectorFields {
                fields: HashMap::new(),
            });
        let fields = connectors
            .fields
            .entry(connector.parse().unwrap())
            .or_insert_with(|| RequiredFieldFinal {
                common: HashMap::new(),
                mandate: HashMap::new(),
                non_mandate: HashMap::new(),
            });
        let fields = match group {
            RuleGroup::Common => &mut fields.common,
            RuleGroup::Mandate => &mut fields.mandate,
            RuleGroup::NonMandate => &mut fields.non_mandate,
        };
        assert!(fields
            .insert(
                outer.to_owned(),
                RequiredFieldInfo {
                    required_field: inner.to_owned(),
                    display_name: "not-a-generated-label".to_owned(),
                    field_type,
                    value: None
                }
            )
            .is_none());
        self.policy.bindings.push(Binding {
            source: SourceIdentity {
                payment_method: name(method),
                payment_method_type: name(method_type),
                connector: name(connector),
                group,
                path: name(outer),
            },
            field: approved(name(field_id)),
            expected_required_field: Some(name(inner)),
        });
        self.catalog
            .fields
            .entry(name(field_id))
            .or_insert_with(|| {
                approved(FieldContract {
                    render_kind: kind,
                    write_path: name(inner),
                    read_path: name(inner.strip_prefix("payment_method_data.").unwrap_or(inner)),
                    review: name("synthetic-only"),
                    source_types: source_types.iter().map(|v| name(v)).collect(),
                    default_options: None,
                })
            });
    }

    #[allow(
        clippy::unwrap_in_result,
        reason = "invalid fixture capture must panic, not masquerade as an expected renderer refusal"
    )]
    fn attempt(&self) -> error_stack::Result<RenderedBundle, RenderError> {
        let effective = EffectiveInputs::capture(&self.rules, &self.banks).unwrap();
        render(RenderInputs {
            effective: &effective,
            policy: &self.policy,
            fields: &self.catalog,
            exclusions: &self.exclusions,
            build: &build(),
        })
    }

    fn field(&mut self, id: &str) -> &mut FieldContract {
        match self.catalog.fields.get_mut(&name(id)).unwrap() {
            Decision::Approved { value, .. } => value,
            _ => panic!("synthetic field is refused"),
        }
    }
}

fn assert_refused(fixture: &Fixture, code: DiagnosticCode) {
    let error = fixture.attempt().unwrap_err();
    assert!(
        error.current_context().counts.contains_key(&code),
        "{error:?}"
    );
    // Refused output cannot contain a partially rendered desired map or a bundle hash.
    let error = serde_json::to_value(error.current_context()).unwrap();
    assert!(error.get("desired").is_none() && error.get("bundle_hash").is_none());
}

fn card_fixture(field: FieldType, kind: RenderKind, source_types: &[&str]) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.add(
        ("card", "debit", "adyen", RuleGroup::Common),
        "outer.synthetic",
        "payment_method_data.card.number",
        field,
        "card.number",
        kind,
        source_types,
    );
    fixture
}

fn inheritance() -> Fixture {
    let mut fixture = card_fixture(
        FieldType::DropDown {
            options: values(&["A"]),
        },
        RenderKind::Dropdown,
        &["drop_down"],
    );
    fixture.add(
        ("card", "debit", "adyen", RuleGroup::Mandate),
        "outer.synthetic",
        "payment_method_data.card.number",
        FieldType::DropDown {
            options: values(&["A", "B"]),
        },
        "card.number",
        RenderKind::Dropdown,
        &["drop_down"],
    );
    fixture.field("card.number").default_options = Some(values(&["A", "B"]));
    fixture
}

#[test]
fn independent_inheritance_conditions_and_default_equal_options_are_exact() {
    let bundle = inheritance().attempt().unwrap();
    let contexts = serde_json::to_value(&bundle.desired).unwrap()["contexts"].clone();
    assert_eq!(
        contexts,
        json!([
            [{"connector":"adyen","payment_method":"Card","payment_method_type":"Debit"},{"dynamic_fields.card.number._is_required":{"kind":"required","value":true},"dynamic_fields.card.number._dropdown_options":{"kind":"options","value":["A"]}}],
            [{"connector":"adyen","payment_method":"Card","payment_method_type":"Debit","mandate_type":"mandate"},{"dynamic_fields.card.number._is_required":{"kind":"required","value":true},"dynamic_fields.card.number._dropdown_options":{"kind":"options","value":["A","B"]}}]
        ])
    );
    assert_eq!(bundle.scope.contributed_rules, 2);
    assert!(bundle.desired.diagnostics.is_empty());
    assert!(
        !serde_json::to_string(&bundle.desired.contexts.iter().collect::<Vec<_>>())
            .unwrap()
            .contains("not-a-generated-label")
    );
}

#[test]
fn bank_wallet_credit_and_non_mandate_labels_are_independent_literals() {
    for (method, kind, method_label, type_label) in [
        ("bank_debit", "ach", "BankDebit", "Ach"),
        ("wallet", "paypal", "Wallet", "Paypal"),
        ("card", "credit", "Card", "Credit"),
    ] {
        let mut fixture = Fixture::new();
        fixture.add(
            (method, kind, "stripe", RuleGroup::NonMandate),
            "outer.customer",
            "customer.email",
            FieldType::UserEmailAddress,
            "customer.email",
            RenderKind::Text,
            &["user_email_address"],
        );
        let bundle = fixture.attempt().unwrap();
        let (context, values) = bundle.desired.contexts.first_key_value().unwrap();
        assert_eq!(
            serde_json::to_value(context).unwrap(),
            json!({"connector":"stripe","payment_method":method_label,"payment_method_type":type_label,"mandate_type":"non_mandate"})
        );
        assert_eq!(
            serde_json::to_value(values).unwrap(),
            json!({"dynamic_fields.customer.email._is_required":{"kind":"required","value":true}})
        );
    }
}

#[test]
fn ordered_duplicates_case_and_unicode_are_preserved() {
    let options = values(&["B", "A", "A", "é", "Checking"]);
    let fixture = card_fixture(
        FieldType::DropDown {
            options: options.clone(),
        },
        RenderKind::Dropdown,
        &["drop_down"],
    );
    let bundle = fixture.attempt().unwrap();
    assert!(bundle.desired.contexts.values().any(|v| v
        .values()
        .any(|v| v == &ManagedValue::Options(options.clone()))));
    let changed = card_fixture(
        FieldType::DropDown {
            options: values(&["A", "B", "A", "é", "Checking"]),
        },
        RenderKind::Dropdown,
        &["drop_down"],
    )
    .attempt()
    .unwrap();
    assert_ne!(bundle.hash().unwrap(), changed.hash().unwrap());
    assert_ne!(
        bundle.desired_hash().unwrap(),
        changed.desired_hash().unwrap()
    );
}

#[test]
fn missing_empty_and_unrestricted_are_not_interchangeable() {
    let mut inherited = card_fixture(FieldType::Text, RenderKind::Dropdown, &["text"]);
    assert_refused(&inherited, DiagnosticCode::MissingRequiredOptions);
    inherited.field("card.number").default_options = Some(values(&["A", "B"]));
    let bundle = inherited.attempt().unwrap();
    assert_eq!(bundle.desired.contexts.values().next().unwrap().len(), 1);
    for kind in [
        RenderKind::Dropdown,
        RenderKind::Country,
        RenderKind::BankNames,
    ] {
        let fixture = card_fixture(
            FieldType::DropDown { options: vec![] },
            kind,
            &["drop_down"],
        );
        assert_refused(&fixture, DiagnosticCode::EmptyRequiredOptions);
    }
    let state = card_fixture(
        FieldType::DropDown { options: vec![] },
        RenderKind::State,
        &["drop_down"],
    )
    .attempt()
    .unwrap();
    assert!(state
        .desired
        .contexts
        .values()
        .flat_map(|v| v.values())
        .any(|v| v == &ManagedValue::Options(vec![])));
    assert_refused(
        &card_fixture(
            FieldType::DropDown {
                options: values(&["CA"]),
            },
            RenderKind::State,
            &["drop_down"],
        ),
        DiagnosticCode::UnsupportedOptions,
    );
}

#[test]
fn country_expansion_is_catalog_ordered_and_malformed_values_refuse() {
    let mut fixture = card_fixture(
        FieldType::UserCountry {
            options: values(&["ALL"]),
        },
        RenderKind::Country,
        &["user_country"],
    );
    let bundle = fixture.attempt().unwrap();
    assert!(bundle
        .desired
        .contexts
        .values()
        .flat_map(|v| v.values())
        .any(|v| v == &ManagedValue::Options(values(&["GB", "AT", "US", "DE"]))));
    fixture.catalog.unrestricted_countries = Decision::Refused {
        reason: name("owner-review-pending"),
    };
    assert_refused(&fixture, DiagnosticCode::CountryCatalogUnapproved);
    for list in [vec!["DK,FI,NO,SE"], vec!["gb"], vec!["ALL", "GB"]] {
        let f = card_fixture(
            FieldType::UserCountry {
                options: values(&list),
            },
            RenderKind::Country,
            &["user_country"],
        );
        assert!(f.attempt().is_err());
    }
    for code in ["AC", "TA", "ZZ"] {
        assert_refused(
            &card_fixture(
                FieldType::UserCountry {
                    options: values(&[code]),
                },
                RenderKind::Country,
                &["user_country"],
            ),
            DiagnosticCode::UnsupportedCountry,
        );
    }
    assert_refused(
        &card_fixture(
            FieldType::DropDown {
                options: values(&["ALL"]),
            },
            RenderKind::Dropdown,
            &["drop_down"],
        ),
        DiagnosticCode::UnsupportedOptions,
    );
}

#[test]
fn unexpanded_sentinels_in_inherited_defaults_are_refused() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Dropdown, &["text"]);
    fixture.field("card.number").default_options = Some(values(&["ALL"]));
    assert_refused(&fixture, DiagnosticCode::UnsupportedOptions);
}

#[test]
fn compiled_alias_sources_match_the_independent_scope_expectations() {
    let rules = RequiredFields::new(&BankRedirectConfig::default());
    let input = EffectiveInputs::capture(&rules, &BankRedirectConfig::default()).unwrap();
    for (kind, suffix) in [
        ("online_banking_czech_republic", "czech_republic"),
        ("online_banking_slovakia", "slovakia"),
        ("online_banking_fpx", "fpx"),
        ("online_banking_thailand", "thailand"),
        ("open_banking_uk", "uk"),
    ] {
        let path = format!("payment_method_data.bank_redirect.open_banking_{suffix}.issuer");
        assert!(input
            .rules()
            .iter()
            .any(|r| r.source.connector.as_str() == "adyen"
                && r.source.payment_method.as_str() == "bank_redirect"
                && r.source.payment_method_type.as_str() == kind
                && r.source.path.as_str() == path
                && r.required_field.as_str() == path));
    }
    for field in ["number", "cvc"] {
        let outer = format!("payment_method_data.gift_card.{field}");
        let inner = format!("payment_method_data.gift_card.givex.{field}");
        assert!(input
            .rules()
            .iter()
            .any(|r| r.source.connector.as_str() == "adyen"
                && r.source.payment_method.as_str() == "gift_card"
                && r.source.payment_method_type.as_str() == "givex"
                && r.source.group == RuleGroup::NonMandate
                && r.source.path.as_str() == outer
                && r.required_field.as_str() == inner));
    }
}

#[test]
fn bank_type_preserves_wire_values_and_refuses_capitalization() {
    let lower = card_fixture(
        FieldType::UserBankType {
            options: values(&["checking", "savings", "checking"]),
        },
        RenderKind::Dropdown,
        &["user_bank_type"],
    );
    assert!(lower.attempt().is_ok());
    assert_refused(
        &card_fixture(
            FieldType::UserBankType {
                options: values(&["Checking", "Savings"]),
            },
            RenderKind::Dropdown,
            &["user_bank_type"],
        ),
        DiagnosticCode::UnsupportedOptions,
    );
}

#[test]
fn missing_refused_and_incompatible_contracts_do_not_render() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture.policy.bindings[0].field = Decision::Refused {
        reason: name("pending"),
    };
    assert_refused(&fixture, DiagnosticCode::UnapprovedBinding);
    fixture.policy.bindings.clear();
    assert_refused(&fixture, DiagnosticCode::MissingBinding);
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &[]);
    assert_refused(&fixture, DiagnosticCode::UnsupportedFieldType);
    fixture.catalog.fields.insert(
        name("card.number"),
        Decision::Refused {
            reason: name("pending"),
        },
    );
    assert_refused(&fixture, DiagnosticCode::UnapprovedFieldContract);
    fixture.catalog.fields.clear();
    assert_refused(&fixture, DiagnosticCode::MissingFieldContract);
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture
        .policy
        .payment_methods
        .insert(name("card"), approved(name("card")));
    assert_refused(&fixture, DiagnosticCode::InvalidDimensionLabel);
}

#[test]
fn duplicate_bindings_and_identical_duplicate_producers_refuse_before_insertion() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture
        .policy
        .bindings
        .push(fixture.policy.bindings[0].clone());
    assert_refused(&fixture, DiagnosticCode::DuplicateBinding);
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture.add(
        ("card", "debit", "adyen", RuleGroup::Common),
        "other.outer",
        "payment_method_data.card.number",
        FieldType::Text,
        "card.number",
        RenderKind::Text,
        &["text"],
    );
    assert_refused(&fixture, DiagnosticCode::DuplicateOutput);
}

#[test]
fn alias_requires_exact_inner_path_and_scoped_source_approval() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture.policy.bindings[0].expected_required_field = Some(name("wrong.path"));
    assert_refused(&fixture, DiagnosticCode::InnerPathChanged);
    let mut fixture = Fixture::new();
    fixture.add(
        ("gift_card", "givex", "adyen", RuleGroup::NonMandate),
        "payment_method_data.gift_card.number",
        "payment_method_data.gift_card.givex.number",
        FieldType::UserGiftCardNumber,
        "gift_card.number",
        RenderKind::Text,
        &["user_gift_card_number"],
    );
    fixture.policy.bindings[0].expected_required_field = None;
    assert_refused(&fixture, DiagnosticCode::AliasApprovalMissing);
    fixture.policy.bindings[0].expected_required_field =
        Some(name("payment_method_data.gift_card.givex.number"));
    assert!(fixture.attempt().is_ok());
    fixture.policy.bindings[0].source.connector = name("stripe");
    assert_refused(&fixture, DiagnosticCode::MissingBinding);
}

#[test]
fn scoped_aliases_match_independent_step1_paths_without_approving_the_profile() {
    let pack: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/required_fields_sync_contracts/sdk-v1/cases.json"
    )))
    .unwrap();
    assert_eq!(pack["review"]["status"], "pending-owner-review");
    for case in pack["mapping_cases"].as_array().unwrap() {
        let get = |key: &str| case[key].as_str().unwrap();
        let mut fixture = Fixture::new();
        let givex = get("type") == "givex";
        // Step 1's connector `givex` is synthetic, not a compiled Connector variant.
        // The actual source scope is independently asserted to be Adyen/Givex.
        let connector = if givex {
            assert!(get("connector").parse::<Connector>().is_err());
            "adyen"
        } else {
            get("connector")
        };
        let (field_type, source_type) = if get("id") == "M-givex-cvc" {
            (FieldType::UserGiftCardPin, "user_gift_card_pin")
        } else if givex {
            (FieldType::UserGiftCardNumber, "user_gift_card_number")
        } else {
            (FieldType::Text, "text")
        };
        let outer = if get("id") == "M-givex-cvc" {
            "payment_method_data.gift_card.cvc"
        } else if givex {
            "payment_method_data.gift_card.number"
        } else {
            get("source_path")
        };
        fixture.add(
            (get("method"), get("type"), connector, RuleGroup::NonMandate),
            outer,
            get("source_path"),
            field_type,
            get("field_id"),
            RenderKind::Text,
            &[source_type],
        );
        fixture.field(get("field_id")).write_path = name(get("write_path"));
        fixture.field(get("field_id")).read_path = name(get("read_path"));
        let bundle = fixture.attempt().unwrap();
        let (context, fields) = bundle.desired.contexts.first_key_value().unwrap();
        assert_eq!(context.payment_method_type.as_str(), get("sdk_type"));
        assert_eq!(
            serde_json::to_value(fields).unwrap(),
            json!({format!("dynamic_fields.{}._is_required",get("field_id")):{"kind":"required","value":true}})
        );
        let source = bundle.inputs.rules().first().unwrap();
        assert_eq!(source.source.path.as_str(), outer);
        assert_eq!(source.required_field.as_str(), get("source_path"));
    }
}

#[test]
fn duplicate_write_paths_and_descriptor_replacement_are_refusals() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    fixture.add(
        ("card", "debit", "adyen", RuleGroup::Mandate),
        "second.outer",
        "payment_method_data.card.number",
        FieldType::Text,
        "second.alias",
        RenderKind::Text,
        &["text"],
    );
    assert_refused(&fixture, DiagnosticCode::DuplicateWritePath);
    let mut fixture = inheritance();
    fixture
        .field("card.number")
        .source_types
        .insert(name("text"));
    let fields = fixture
        .rules
        .0
        .get_mut(&PaymentMethod::Card)
        .unwrap()
        .0
        .get_mut(&PaymentMethodType::Debit)
        .unwrap()
        .fields
        .get_mut(&Connector::Adyen)
        .unwrap();
    fields
        .mandate
        .get_mut("outer.synthetic")
        .unwrap()
        .field_type = FieldType::Text;
    assert_refused(&fixture, DiagnosticCode::IncompatibleInheritance);
}

#[test]
fn explicit_exclusions_empty_groups_and_absence_never_grant_retirement() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    let source = fixture.policy.bindings.remove(0).source;
    fixture.exclusions.sources.push(Exclusion {
        source: source.clone(),
        reason: name("synthetic-only-scope"),
        review: name("synthetic-only"),
    });
    let bundle = fixture.attempt().unwrap();
    assert!(bundle.desired.contexts.is_empty());
    assert_eq!(bundle.scope.excluded_sources, vec![source]);
    assert_eq!(
        serde_json::to_value(bundle.scope).unwrap()["retirement_authority"],
        "not_granted"
    );
    fixture.rules.0.clear();
    assert_refused(&fixture, DiagnosticCode::SourceUnavailable);
    fixture.exclusions.sources.clear();
    assert!(fixture.attempt().unwrap().desired.contexts.is_empty());
    fixture.rules.0.insert(
        PaymentMethod::Wallet,
        crate::configs::settings::PaymentMethodType(HashMap::from([(
            PaymentMethodType::Paypal,
            ConnectorFields {
                fields: HashMap::from([(
                    Connector::Adyen,
                    RequiredFieldFinal {
                        common: HashMap::new(),
                        mandate: HashMap::new(),
                        non_mandate: HashMap::new(),
                    },
                )]),
            },
        )])),
    );
    assert!(fixture.attempt().unwrap().desired.contexts.is_empty());
}

fn eps_fixture(banks: &[BankNames]) -> Fixture {
    let mut fixture = Fixture::new();
    fixture.banks = BankRedirectConfig(HashMap::from([(
        PaymentMethodType::Eps,
        ConnectorBankNames(HashMap::from([(
            "stripe".to_owned(),
            BanksVector {
                banks: banks.iter().copied().collect(),
            },
        )])),
    )]));
    let compiled = RequiredFields::new(&fixture.banks);
    let field = compiled.0[&PaymentMethod::BankRedirect].0[&PaymentMethodType::Eps].fields
        [&Connector::Stripe]
        .non_mandate[EPS_PATH]
        .clone();
    // Explicit synthetic projection of ONE compiled source; not a successful full profile.
    fixture.add(
        ("bank_redirect", "eps", "stripe", RuleGroup::NonMandate),
        EPS_PATH,
        &field.required_field,
        field.field_type,
        "bank_redirect.eps.bank_name",
        RenderKind::BankNames,
        &["user_bank_options"],
    );
    fixture
}

#[test]
fn actual_eps_profile_variation_and_shuffled_sources_preserve_hash_invariants() {
    let first = eps_fixture(&[BankNames::BankAustria, BankNames::AbnAmro])
        .attempt()
        .unwrap();
    for _ in 0..10 {
        let second = eps_fixture(&[BankNames::AbnAmro, BankNames::BankAustria])
            .attempt()
            .unwrap();
        assert_eq!(
            first.canonical_bytes().unwrap(),
            second.canonical_bytes().unwrap()
        );
        assert_eq!(first.hash().unwrap(), second.hash().unwrap());
    }
    let changed = eps_fixture(&[BankNames::BankAustria]).attempt().unwrap();
    assert_ne!(first.hash().unwrap(), changed.hash().unwrap());
    let mut a = serde_json::to_value(&first.desired).unwrap()["contexts"].clone();
    let mut b = serde_json::to_value(&changed.desired).unwrap()["contexts"].clone();
    a[0][1]
        .as_object_mut()
        .unwrap()
        .remove("dynamic_fields.bank_redirect.eps.bank_name._dropdown_options");
    b[0][1]
        .as_object_mut()
        .unwrap()
        .remove("dynamic_fields.bank_redirect.eps.bank_name._dropdown_options");
    assert_eq!(a, b);
    assert_refused(&eps_fixture(&[]), DiagnosticCode::EmptyRequiredOptions);
}

#[test]
fn compiled_card_credit_assignment_is_preserved_and_full_profile_is_blocked() {
    let compiled = RequiredFields::new(&BankRedirectConfig::default());
    let captured = EffectiveInputs::capture(&compiled, &BankRedirectConfig::default()).unwrap();
    assert!(captured
        .rules()
        .iter()
        .any(|r| r.source.connector.as_str() == "getnet"
            && r.source.payment_method_type.as_str() == "credit"
            && r.required_field.as_str() == "payment_method_data.card.card_network"));
    assert!(!captured
        .rules()
        .iter()
        .any(|r| r.source.connector.as_str() == "getnet"
            && r.source.payment_method_type.as_str() == "debit"
            && r.required_field.as_str() == "payment_method_data.card.card_network"));
    let mut fixture = Fixture::new();
    fixture.rules = compiled;
    let error = fixture.attempt().unwrap_err();
    assert!(error.current_context().counts[&DiagnosticCode::MissingBinding] > 256);
    assert_eq!(error.current_context().diagnostics.len(), 256);
    assert!(error
        .current_context()
        .diagnostics
        .iter()
        .any(|d| d.code == DiagnosticCode::MalformedOptions));
    assert!(error
        .current_context()
        .counts
        .contains_key(&DiagnosticCode::MalformedOptions));
}

#[test]
fn source_display_serde_discrepancy_is_not_silently_normalized() {
    assert_eq!(
        PaymentMethodType::ClassicReward.to_string(),
        "classic_reward"
    );
    assert_eq!(
        serde_json::to_value(PaymentMethodType::ClassicReward).unwrap(),
        "classic"
    );
    assert!(vocabulary::type_label(PaymentMethodType::ClassicReward).is_none());
    assert!(source_name::<PaymentMethodType>(&name("classic_reward")).is_ok());
    assert!(source_name::<PaymentMethodType>(&name("classic")).is_err());
}

#[test]
fn prefill_changes_neither_desired_nor_retained_bundle() {
    let mut fixture = card_fixture(FieldType::Text, RenderKind::Text, &["text"]);
    let before = fixture.attempt().unwrap().canonical_bytes().unwrap();
    fixture
        .rules
        .0
        .get_mut(&PaymentMethod::Card)
        .unwrap()
        .0
        .get_mut(&PaymentMethodType::Debit)
        .unwrap()
        .fields
        .get_mut(&Connector::Adyen)
        .unwrap()
        .common
        .get_mut("outer.synthetic")
        .unwrap()
        .value = Some(hyperswitch_masking::Secret::new(
        "synthetic-prefill-do-not-retain".to_owned(),
    ));
    assert_eq!(
        before,
        fixture.attempt().unwrap().canonical_bytes().unwrap()
    );
}

/// Explicitly opted in by the runner after registering an owned filesystem target. Exports a
/// successful synthetic bundle for cross-process and actual pinned JS resolver checks only.
#[test]
#[ignore = "requires registered RF_RENDER_FIXTURE_ROOT"]
fn export_synthetic_renderer_bundle() {
    use std::io::Write as _;
    let root = std::path::PathBuf::from(std::env::var("RF_RENDER_FIXTURE_ROOT").unwrap());
    let owner: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(
        owner["purpose"],
        "Step 4 offline synthetic renderer/build checks"
    );
    let ordered = card_fixture(
        FieldType::DropDown {
            options: values(&["B", "A", "A", "é", "Checking"]),
        },
        RenderKind::Dropdown,
        &["drop_down"],
    );
    for (name, fixture) in [
        ("inheritance", inheritance()),
        ("ordered", ordered),
        ("empty", Fixture::new()),
        (
            "eps-ab",
            eps_fixture(&[BankNames::BankAustria, BankNames::AbnAmro]),
        ),
    ] {
        let output = root.join(format!("{name}-{}.json", common_utils::process_id()));
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output)
            .unwrap();
        file.write_all(&fixture.attempt().unwrap().canonical_bytes().unwrap())
            .unwrap();
    }
}

#[test]
fn every_current_field_type_is_exercised_with_explicit_compatibility() {
    let cases = vec![
        (
            FieldType::UserCardNumber,
            "user_card_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserGiftCardNumber,
            "user_gift_card_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserCardExpiryMonth,
            "user_card_expiry_month",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserCardExpiryYear,
            "user_card_expiry_year",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserCardCvc,
            "user_card_cvc",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserGiftCardPin,
            "user_gift_card_pin",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserCardNetwork,
            "user_card_network",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserFullName,
            "user_full_name",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserEmailAddress,
            "user_email_address",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserPhoneNumber,
            "user_phone_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserPhoneNumberCountryCode,
            "user_phone_number_country_code",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserCountry {
                options: values(&["GB", "AT"]),
            },
            "user_country",
            RenderKind::Country,
            Some(values(&["GB", "AT"])),
        ),
        (
            FieldType::UserCurrency {
                options: values(&["B", "A", "A"]),
            },
            "user_currency",
            RenderKind::Dropdown,
            Some(values(&["B", "A", "A"])),
        ),
        (
            FieldType::UserCryptoCurrencyNetwork,
            "user_crypto_currency_network",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBillingName,
            "user_billing_name",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserAddressLine1,
            "user_address_line1",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserAddressLine2,
            "user_address_line2",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserAddressCity,
            "user_address_city",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserAddressPincode,
            "user_address_pincode",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserAddressState,
            "user_address_state",
            RenderKind::State,
            None,
        ),
        (
            FieldType::UserAddressCountry {
                options: values(&["GB", "AT"]),
            },
            "user_address_country",
            RenderKind::Country,
            Some(values(&["GB", "AT"])),
        ),
        (
            FieldType::UserShippingName,
            "user_shipping_name",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserShippingAddressLine1,
            "user_shipping_address_line1",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserShippingAddressLine2,
            "user_shipping_address_line2",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserShippingAddressCity,
            "user_shipping_address_city",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserShippingAddressPincode,
            "user_shipping_address_pincode",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserShippingAddressState,
            "user_shipping_address_state",
            RenderKind::State,
            None,
        ),
        (
            FieldType::UserShippingAddressCountry {
                options: values(&["GB", "AT"]),
            },
            "user_shipping_address_country",
            RenderKind::Country,
            Some(values(&["GB", "AT"])),
        ),
        (
            FieldType::UserDocumentType {
                options: values(&["B", "A", "A"]),
            },
            "user_document_type",
            RenderKind::Dropdown,
            Some(values(&["B", "A", "A"])),
        ),
        (
            FieldType::UserSocialSecurityNumber,
            "user_social_security_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBlikCode,
            "user_blik_code",
            RenderKind::Text,
            None,
        ),
        (FieldType::UserBank, "user_bank", RenderKind::Text, None),
        (
            FieldType::UserBankOptions {
                options: values(&["B", "A", "A"]),
            },
            "user_bank_options",
            RenderKind::BankNames,
            Some(values(&["B", "A", "A"])),
        ),
        (
            FieldType::UserBankAccountNumber,
            "user_bank_account_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserSourceBankAccountId,
            "user_source_bank_account_id",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserDestinationBankAccountId,
            "user_destination_bank_account_id",
            RenderKind::Text,
            None,
        ),
        (FieldType::Text, "text", RenderKind::Text, None),
        (
            FieldType::DropDown {
                options: values(&["B", "A", "A"]),
            },
            "drop_down",
            RenderKind::Dropdown,
            Some(values(&["B", "A", "A"])),
        ),
        (
            FieldType::UserDateOfBirth,
            "user_date_of_birth",
            RenderKind::Text,
            None,
        ),
        (FieldType::UserVpaId, "user_vpa_id", RenderKind::Text, None),
        (
            FieldType::LanguagePreference {
                options: values(&["B", "A", "A"]),
            },
            "language_preference",
            RenderKind::LanguagePreference,
            Some(values(&["B", "A", "A"])),
        ),
        (
            FieldType::UserPixKey,
            "user_pix_key",
            RenderKind::Text,
            None,
        ),
        (FieldType::UserCpf, "user_cpf", RenderKind::Text, None),
        (FieldType::UserCnpj, "user_cnpj", RenderKind::Text, None),
        (FieldType::UserIban, "user_iban", RenderKind::Text, None),
        (
            FieldType::UserBsbNumber,
            "user_bsb_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBankSortCode,
            "user_bank_sort_code",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBankRoutingNumber,
            "user_bank_routing_number",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBankType {
                options: values(&["checking", "savings"]),
            },
            "user_bank_type",
            RenderKind::Dropdown,
            Some(values(&["checking", "savings"])),
        ),
        (
            FieldType::UserBankAccountHolderName,
            "user_bank_account_holder_name",
            RenderKind::Text,
            None,
        ),
        (FieldType::UserMsisdn, "user_msisdn", RenderKind::Text, None),
        (
            FieldType::UserClientIdentifier,
            "user_client_identifier",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::OrderDetailsProductName,
            "order_details_product_name",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBranchCode,
            "user_branch_code",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserBankIdentifier,
            "user_bank_identifier",
            RenderKind::Text,
            None,
        ),
        (
            FieldType::UserPixAccountNumber,
            "user_pix_account_number",
            RenderKind::Text,
            None,
        ),
    ];
    assert_eq!(cases.len(), 56);
    for (field, tag, kind, expected_options) in cases {
        let fixture = card_fixture(field, kind, &[tag]);
        let bundle = fixture.attempt().unwrap();
        let fields = bundle.desired.contexts.values().next().unwrap();
        let key: ManagedKey = "dynamic_fields.card.number._dropdown_options"
            .to_owned()
            .try_into()
            .unwrap();
        assert_eq!(
            fields.get(&key),
            expected_options.map(ManagedValue::Options).as_ref(),
            "{tag}"
        );
    }
}

#[test]
fn every_dimension_variant_matches_the_pinned_source_casing_contract() {
    // Independent translation of U CommonUtils.res's inspected snakeToPascalCase contract.
    // This is source-contract evidence, not execution of that compiled browser function.
    fn sdk_label(source: &str) -> String {
        source
            .split('_')
            .filter(|s| !s.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                let first = chars.next().unwrap().to_uppercase().collect::<String>();
                first + chars.as_str()
            })
            .collect()
    }
    let variants = [
        PaymentMethod::Card,
        PaymentMethod::CardRedirect,
        PaymentMethod::PayLater,
        PaymentMethod::Wallet,
        PaymentMethod::BankRedirect,
        PaymentMethod::BankTransfer,
        PaymentMethod::Crypto,
        PaymentMethod::BankDebit,
        PaymentMethod::Reward,
        PaymentMethod::RealTimePayment,
        PaymentMethod::Upi,
        PaymentMethod::Voucher,
        PaymentMethod::GiftCard,
        PaymentMethod::OpenBanking,
        PaymentMethod::MobilePayment,
        PaymentMethod::NetworkToken,
    ];
    assert_eq!(variants.len(), 16);
    for value in variants {
        assert_eq!(
            vocabulary::method_label(value),
            sdk_label(&value.to_string())
        );
        assert_eq!(serde_json::to_value(value).unwrap(), value.to_string());
    }
    let variants = [
        PaymentMethodType::Ach,
        PaymentMethodType::Affirm,
        PaymentMethodType::AfterpayClearpay,
        PaymentMethodType::Alfamart,
        PaymentMethodType::AliPay,
        PaymentMethodType::AliPayHk,
        PaymentMethodType::Alma,
        PaymentMethodType::AmazonPay,
        PaymentMethodType::Paysera,
        PaymentMethodType::ApplePay,
        PaymentMethodType::Atome,
        PaymentMethodType::Bacs,
        PaymentMethodType::BancontactCard,
        PaymentMethodType::Becs,
        PaymentMethodType::Benefit,
        PaymentMethodType::Bizum,
        PaymentMethodType::Blik,
        PaymentMethodType::Bluecode,
        PaymentMethodType::Boleto,
        PaymentMethodType::BcaBankTransfer,
        PaymentMethodType::BniVa,
        PaymentMethodType::Breadpay,
        PaymentMethodType::BriVa,
        PaymentMethodType::BhnCardNetwork,
        PaymentMethodType::CardRedirect,
        PaymentMethodType::CimbVa,
        PaymentMethodType::ClassicReward,
        PaymentMethodType::Credit,
        PaymentMethodType::CryptoCurrency,
        PaymentMethodType::Cashapp,
        PaymentMethodType::Dana,
        PaymentMethodType::DanamonVa,
        PaymentMethodType::Debit,
        PaymentMethodType::DuitNow,
        PaymentMethodType::Efecty,
        PaymentMethodType::Eft,
        PaymentMethodType::EftDebitOrder,
        PaymentMethodType::Eps,
        PaymentMethodType::Flexiti,
        PaymentMethodType::Fps,
        PaymentMethodType::Evoucher,
        PaymentMethodType::Giropay,
        PaymentMethodType::Givex,
        PaymentMethodType::GooglePay,
        PaymentMethodType::GoPay,
        PaymentMethodType::Gcash,
        PaymentMethodType::Ideal,
        PaymentMethodType::Interac,
        PaymentMethodType::Indomaret,
        PaymentMethodType::Klarna,
        PaymentMethodType::KakaoPay,
        PaymentMethodType::LocalBankRedirect,
        PaymentMethodType::MandiriVa,
        PaymentMethodType::Knet,
        PaymentMethodType::MbWay,
        PaymentMethodType::MobilePay,
        PaymentMethodType::Momo,
        PaymentMethodType::MomoAtm,
        PaymentMethodType::Multibanco,
        PaymentMethodType::Neteller,
        PaymentMethodType::OnlineBankingThailand,
        PaymentMethodType::OnlineBankingCzechRepublic,
        PaymentMethodType::OnlineBankingFinland,
        PaymentMethodType::OnlineBankingFpx,
        PaymentMethodType::OnlineBankingPoland,
        PaymentMethodType::OnlineBankingSlovakia,
        PaymentMethodType::Oxxo,
        PaymentMethodType::PagoEfectivo,
        PaymentMethodType::PermataBankTransfer,
        PaymentMethodType::OpenBankingUk,
        PaymentMethodType::PayBright,
        PaymentMethodType::Payjustnow,
        PaymentMethodType::Paypal,
        PaymentMethodType::Paze,
        PaymentMethodType::Pix,
        PaymentMethodType::PixKey,
        PaymentMethodType::PixEmv,
        PaymentMethodType::PixQr,
        PaymentMethodType::PixAutomaticoQr,
        PaymentMethodType::PixAutomaticoPush,
        PaymentMethodType::PaySafeCard,
        PaymentMethodType::Przelewy24,
        PaymentMethodType::PromptPay,
        PaymentMethodType::Pse,
        PaymentMethodType::Qris,
        PaymentMethodType::RedCompra,
        PaymentMethodType::RedPagos,
        PaymentMethodType::SamsungPay,
        PaymentMethodType::Sepa,
        PaymentMethodType::SepaBankTransfer,
        PaymentMethodType::SepaGuarenteedDebit,
        PaymentMethodType::Skrill,
        PaymentMethodType::Sofort,
        PaymentMethodType::Swish,
        PaymentMethodType::TouchNGo,
        PaymentMethodType::Trustly,
        PaymentMethodType::Twint,
        PaymentMethodType::UpiCollect,
        PaymentMethodType::UpiIntent,
        PaymentMethodType::UpiQr,
        PaymentMethodType::Vipps,
        PaymentMethodType::VietQr,
        PaymentMethodType::Venmo,
        PaymentMethodType::Walley,
        PaymentMethodType::Wero,
        PaymentMethodType::WeChatPay,
        PaymentMethodType::SevenEleven,
        PaymentMethodType::Lawson,
        PaymentMethodType::MiniStop,
        PaymentMethodType::FamilyMart,
        PaymentMethodType::Seicomart,
        PaymentMethodType::PayEasy,
        PaymentMethodType::LocalBankTransfer,
        PaymentMethodType::Mifinity,
        PaymentMethodType::OpenBankingPIS,
        PaymentMethodType::DirectCarrierBilling,
        PaymentMethodType::InstantBankTransfer,
        PaymentMethodType::InstantBankTransferFinland,
        PaymentMethodType::InstantBankTransferPoland,
        PaymentMethodType::RevolutPay,
        PaymentMethodType::IndonesianBankTransfer,
        PaymentMethodType::OpenBanking,
        PaymentMethodType::NetworkToken,
        PaymentMethodType::Payshap,
        PaymentMethodType::PayshapProxy,
        PaymentMethodType::Ted,
    ];
    assert_eq!(variants.len(), 126);
    for value in variants {
        if value == PaymentMethodType::ClassicReward {
            assert!(vocabulary::type_label(value).is_none());
            continue;
        }
        assert_eq!(
            vocabulary::type_label(value).unwrap(),
            sdk_label(&value.to_string())
        );
        assert_eq!(serde_json::to_value(value).unwrap(), value.to_string());
    }
}

#[cfg(feature = "dummy_connector")]
#[test]
fn dummy_rules_require_explicit_bindings_or_scoped_exclusions() {
    let mut fixture = Fixture::new();
    let dummy = Connector::DummyConnector1.to_string();
    fixture.add(
        ("card", "debit", &dummy, RuleGroup::Common),
        "outer.synthetic",
        "payment_method_data.card.number",
        FieldType::Text,
        "card.number",
        RenderKind::Text,
        &["text"],
    );
    assert_refused(&fixture, DiagnosticCode::MissingDimensionMapping);
    fixture
        .policy
        .connectors
        .insert(name(&dummy), approved(name(&dummy)));
    fixture
        .catalog
        .sdk
        .dimensions
        .get_mut(&name("connector"))
        .unwrap()
        .push(name(&dummy));
    assert_eq!(fixture.attempt().unwrap().scope.contributed_rules, 1);
    let source = fixture.policy.bindings.remove(0).source;
    assert_refused(&fixture, DiagnosticCode::MissingBinding);
    fixture.exclusions.sources.push(Exclusion {
        source,
        reason: name("synthetic-dummy-scope"),
        review: name("synthetic-only"),
    });
    let bundle = fixture.attempt().unwrap();
    assert!(bundle.desired.contexts.is_empty());
    assert_eq!(bundle.scope.excluded_sources.len(), 1);
    assert_eq!(
        serde_json::to_value(bundle.scope).unwrap()["retirement_authority"],
        "not_granted"
    );
}
