//! Handwritten seed expectations; fixtures confer no business or live-target approval.
use super::{
    super::super::seed::{self, Grant, Policy, Scope},
    *,
};

const TEMPLATE: &str = r#"# Manual header stays byte-identical.
[default-configs]
deja_record = { value = false, schema = { type = "boolean" } } # independent Deja
"dynamic_fields.choice._is_required" = { value = false, schema = { type = "boolean" } }
"dynamic_fields.choice._dropdown_options" = { value = ["A", "B"], schema = { type = "array", items = { type = "string" } } }
"dynamic_fields.choice._field_render_type" = { value = "Dropdown", schema = { type = "string" } }
"dynamic_fields.choice._confirm_request_write_path" = { value = "payment_method_data.choice", schema = { type = "string" } }
"dynamic_fields.choice._intent_data_read_path" = { value = "choice", schema = { type = "string" } }
"dynamic_fields.choice._default_label_text" = { value = "Choose", schema = { type = "string" } }
"dynamic_fields.choice._field_display_order" = { value = 1, schema = { type = "integer" } }

[dimensions.connector]
position = 0
type = "REGULAR"
schema = { type = "string", enum = ["a", "b"] }
[dimensions.payment_method]
position = 1
type = "REGULAR"
schema = { type = "string", enum = ["Card"] }
[dimensions.payment_method_type]
position = 2
type = "REGULAR"
schema = { type = "string", enum = ["Debit", "Credit"] }
[dimensions.mandate_type]
position = 3
type = "REGULAR"
schema = { type = "string", enum = ["mandate", "non_mandate"] }

[[overrides]]
_context_ = { connector = "a", payment_method = "Card", payment_method_type = "Debit" }
"dynamic_fields.choice._is_required" = false # retain inline comment
"dynamic_fields.choice._dropdown_options" = ["B", "A"]
manual = { nested = ["Z", "A", "A"], empty = [], flag = false, number = 1.5 }

[[overrides]]
_context_ = { country = "US", tenant = "synthetic" }
manual = "baseline — unchanged"
"dynamic_fields.choice._is_required" = false
"dynamic_fields.choice._dropdown_options" = ["ZZ"]
"#;
fn policy(desired: &VerifiedDesired) -> Policy {
    let prerequisites = prereqs(desired);
    let build = v!(&desired.desired.input_manifest.build);
    Policy {
        schema_version: 1,
        contract_id: n("synthetic-seed-policy"),
        profile_hash: d("profile"),
        router_feature_keys: serde_json::from_value(build["router_feature_keys"].clone()).unwrap(),
        payment_methods_feature_keys: serde_json::from_value(
            build["payment_methods_feature_keys"].clone(),
        )
        .unwrap(),
        scope: Decision::Approved {
            review: n("synthetic-only"),
            value: Scope {
                grants: vec![Grant {
                    condition: condition("a"),
                    attributes: BTreeSet::from([key(REQUIRED), key(OPTIONS)]),
                    retire: BTreeSet::new(),
                    review: n("synthetic-only"),
                }],
                dimensions: prerequisites.dimensions,
                fields: prerequisites.fields,
                create_defaults: vec![],
            },
        },
    }
}
fn scope(policy: &mut Policy) -> &mut Scope {
    let Decision::Approved { value, .. } = &mut policy.scope else {
        panic!("fixture approval")
    };
    value
}
fn generate(text: &str, desired: &VerifiedDesired, policy: &Policy) -> PlanResult<seed::Generated> {
    seed::generate(text, desired, policy, &d("profile"))
}
fn document(text: &str) -> Value {
    toml_edit::de::from_str(text).unwrap()
}

#[test]
fn seed_is_deterministic_and_preserves_manual_content_and_explicit_default_equal_values() {
    let desired = desired();
    let policy = policy(&desired);
    let first = generate(TEMPLATE, &desired, &policy).unwrap();
    assert_eq!(
        first.text,
        generate(TEMPLATE, &desired, &policy).unwrap().text
    );
    let actual = document(&first.text);
    let before = document(TEMPLATE);
    assert_eq!(actual["overrides"][0][REQUIRED], v!(true));
    assert_eq!(actual["overrides"][0][OPTIONS], v!(["A", "B"]));
    assert_eq!(
        actual["overrides"][0]["manual"],
        before["overrides"][0]["manual"]
    );
    assert_eq!(actual["overrides"][1], before["overrides"][1]);
    assert_eq!(actual["default-configs"], before["default-configs"]);
    assert!(first
        .text
        .starts_with("# Manual header stays byte-identical."));
    assert!(first.text.contains("# retain inline comment"));
    assert!(first.text.contains("# independent Deja"));
    let checked = generate(&first.text, &desired, &policy).unwrap();
    assert_eq!(first.text, checked.text);
    assert!(checked.report.changes.is_empty());
    assert_eq!(first.report.changes.len(), 2);
}

#[test]
fn seed_refuses_ambiguous_identity_missing_grants_approval_and_build_drift() {
    let desired = desired();
    let base = policy(&desired);
    let duplicate = format!("{TEMPLATE}\n[[overrides]]\n_context_ = {{ payment_method_type = 'Debit', payment_method = 'Card', connector = 'a' }}\nmanual = true\n");
    assert_eq!(
        generate(&duplicate, &desired, &base)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::AmbiguousIdentity
    );
    for change in 0..6 {
        let mut policy = base.clone();
        match change {
            0 => scope(&mut policy).grants.clear(),
            1 => {
                let grant = scope(&mut policy).grants[0].clone();
                scope(&mut policy).grants.push(grant);
            }
            2 => {
                policy.scope = Decision::Refused {
                    reason: n("review-pending"),
                }
            }
            3 => policy.profile_hash = d("wrong"),
            4 => policy.schema_version = 2,
            _ => {
                policy.router_feature_keys.insert(n("V2"));
            }
        }
        assert!(generate(TEMPLATE, &desired, &policy).is_err(), "{change}");
    }
}

#[test]
fn seed_retirement_is_explicit_and_retains_shared_rows() {
    let mut desired = desired();
    let mut policy = policy(&desired);
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .remove(&key(OPTIONS));
    assert_eq!(
        generate(TEMPLATE, &desired, &policy)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::RetirementRequired
    );
    scope(&mut policy).grants[0].retire.insert(key(OPTIONS));
    let generated = generate(TEMPLATE, &desired, &policy).unwrap();
    let value = document(&generated.text);
    assert!(value["overrides"][0].get(OPTIONS).is_none());
    assert_eq!(
        value["overrides"][0]["manual"],
        document(TEMPLATE)["overrides"][0]["manual"]
    );
    assert_eq!(
        value["default-configs"][OPTIONS],
        document(TEMPLATE)["default-configs"][OPTIONS]
    );
}

#[test]
fn seed_adds_only_declared_prerequisites_and_refuses_manual_metadata_corrections() {
    let desired = desired();
    let mut policy = policy(&desired);
    let missing = TEMPLATE
        .lines()
        .filter(|line| !line.starts_with(&format!("\"{OPTIONS}\" = {{ value")))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        generate(&missing, &desired, &policy)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::MissingDefault
    );
    scope(&mut policy).create_defaults.push(DefaultCreation {
        key: key(OPTIONS),
        value: ManagedValue::Options(vec!["A".into(), "B".into()]),
        description: "synthetic declaration".into(),
        review: n("fixture-only"),
    });
    let added = generate(&missing, &desired, &policy).unwrap();
    assert_eq!(
        document(&added.text)["default-configs"][OPTIONS]["value"],
        v!(["A", "B"])
    );
    let wrong = TEMPLATE.replace("value = \"Choose\"", "value = \"unreviewed label\"");
    assert_eq!(
        generate(&wrong, &desired, &policy)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::IncompatiblePresentation
    );
    let missing_enum = TEMPLATE.replace("enum = [\"a\", \"b\"]", "enum = [\"b\"]");
    assert!(generate(&missing_enum, &desired, &policy).is_err());
    scope(&mut policy)
        .dimensions
        .get_mut(&n("connector"))
        .unwrap()
        .allow_enum_extension = true;
    let added = generate(&missing_enum, &desired, &policy).unwrap();
    assert_eq!(
        document(&added.text)["dimensions"]["connector"]["schema"]["enum"],
        v!(["b", "a"])
    );
}

#[test]
fn seed_limits_refuse_without_truncating_or_splitting_the_output() {
    let desired = desired();
    let mut policy = policy(&desired);
    assert_eq!(
        generate(&" ".repeat(seed::MAX_SEED_BYTES + 1), &desired, &policy)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::LimitExceeded
    );
    let grant = scope(&mut policy).grants[0].clone();
    scope(&mut policy).grants = vec![grant; 10_001];
    assert_eq!(
        generate(TEMPLATE, &desired, &policy)
            .err()
            .unwrap()
            .current_context()
            .code,
        PlanCode::LimitExceeded
    );
}

#[test]
fn seed_refuses_malformed_managed_values_functions_and_declarations() {
    let desired = desired();
    let policy = policy(&desired);
    for template in [
        TEMPLATE.replace("false # retain inline comment", "'false'"),
        TEMPLATE.replace("= [\"B\", \"A\"]", "= [1, 2]"),
        TEMPLATE.replace("position = 0", "position = 0\nmandatory = 'false'"),
        TEMPLATE.replace(
            "position = 0",
            "position = 0\nvalue_validation_function_name = 'custom'",
        ),
    ] {
        assert!(generate(&template, &desired, &policy).is_err());
    }
    let mut malformed = policy.clone();
    scope(&mut malformed).create_defaults.push(DefaultCreation {
        key: key(REQUIRED),
        value: ManagedValue::Options(vec!["A".into()]),
        description: "synthetic wrong type".into(),
        review: n("fixture-only"),
    });
    assert!(generate(TEMPLATE, &desired, &malformed).is_err());
    let mut json = v!(policy);
    json["scope"]["value"]["grants"][0]["attributes"] = v!([REQUIRED, REQUIRED]);
    assert!(serde_json::from_value::<Policy>(json).is_err());
    let mut json = v!(policy);
    json["force"] = v!(true);
    assert!(serde_json::from_value::<Policy>(json).is_err());
}

#[test]
fn seed_new_rows_and_owned_only_retirement_preserve_manual_row_order() {
    let mut desired = desired();
    let mut policy = policy(&desired);
    let values = desired.desired.contexts.remove(&condition("a")).unwrap();
    desired.desired.contexts.insert(condition("b"), values);
    let mut added = scope(&mut policy).grants[0].clone();
    added.condition = condition("b");
    scope(&mut policy).grants.push(added);
    scope(&mut policy).grants[0].retire = BTreeSet::from([key(REQUIRED), key(OPTIONS)]);
    let template = TEMPLATE
        .lines()
        .filter(|line| !line.starts_with("manual = {"))
        .collect::<Vec<_>>()
        .join("\n");
    let generated = generate(&template, &desired, &policy).unwrap();
    let actual = document(&generated.text);
    assert_eq!(actual["overrides"].as_array().unwrap().len(), 2);
    assert_eq!(actual["overrides"][0], document(TEMPLATE)["overrides"][1]);
    assert_eq!(actual["overrides"][1]["_context_"], v!(condition("b")));
    assert_eq!(actual["overrides"][1][OPTIONS], v!(["A", "B"]));
    assert_eq!(
        generated.text,
        generate(&generated.text, &desired, &policy).unwrap().text
    );
}

#[test]
fn seed_order_is_semantic_even_if_a_wrong_generator_and_seed_agree() {
    let mut desired = desired();
    let policy = policy(&desired);
    desired
        .desired
        .contexts
        .get_mut(&condition("a"))
        .unwrap()
        .insert(
            key(OPTIONS),
            ManagedValue::Options(vec!["B".into(), "A".into()]),
        );
    let wrong = generate(TEMPLATE, &desired, &policy).unwrap();
    assert_eq!(
        wrong.text,
        generate(&wrong.text, &desired, &policy).unwrap().text
    );
    // This is the independent contract, not regenerated from desired or the edited seed.
    assert_ne!(
        document(&wrong.text)["overrides"][0][OPTIONS],
        v!(["A", "B"])
    );
}

#[test]
#[ignore = "requires explicitly registered RF_SEED_FIXTURE_ROOT; exports synthetic good/bad semantic inputs"]
fn seed_export_independent_semantic_inputs() {
    use std::io::Write as _;
    let root = std::path::PathBuf::from(std::env::var("RF_SEED_FIXTURE_ROOT").unwrap());
    let owner: Value =
        serde_json::from_slice(&std::fs::read(root.join("owner.json")).unwrap()).unwrap();
    assert_eq!(owner["purpose"], "Step 14 isolated seed and CI checks");
    assert_eq!(owner["root"], v!(root));
    assert_eq!(owner["cleanup_scope"], v!(root));
    assert!(owner["hosted_target"].is_null());
    let work = root.join("seed-semantic-fixtures");
    std::fs::create_dir(&work).unwrap();
    let mut desired = desired();
    let policy = policy(&desired);
    for name in ["good", "wrong"] {
        if name == "wrong" {
            desired
                .desired
                .contexts
                .get_mut(&condition("a"))
                .unwrap()
                .insert(
                    key(OPTIONS),
                    ManagedValue::Options(vec!["B".into(), "A".into()]),
                );
        }
        let generated = generate(TEMPLATE, &desired, &policy).unwrap();
        assert_eq!(
            generated.text,
            generate(&generated.text, &desired, &policy).unwrap().text
        );
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(work.join(format!("{name}.json")))
            .unwrap();
        file.write_all(&serde_json::to_vec(&document(&generated.text)).unwrap())
            .unwrap();
    }
}
