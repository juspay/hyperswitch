//! `[mandates.supported_payment_methods]` decides whether `setup_future_usage: off_session` is
//! honoured for a connector and payment method type. A config file that drifts from the others
//! silently downgrades those mandates to `on_session` in that one environment, so these tests
//! compare the tables across files. Connector lists are compared as sets, so that differences in
//! ordering do not hide real drift.
//!
//! The test only needs to parse TOML, so it lives in this lightweight crate rather than `router`,
//! which lets CI run it on every pull request without building the router.

// Integration test: assertions use panic!/expect(); allow the production-code lints.
#![allow(clippy::panic, clippy::expect_used)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

/// `"<payment_method>.<payment_method_type>"` -> connectors that support mandates for it.
type MandateMatrix = BTreeMap<String, BTreeSet<String>>;

const REFERENCE_DEPLOYMENT_CONFIG: &str = "config/deployments/production.toml";

/// Differences from `production.toml` that exist on `main` today, in the format produced by
/// [`drift`]. Remove an entry once the corresponding file is brought in line; add one only when
/// the divergence is intentional.
const KNOWN_DIVERGENCES: &[(&str, &[&str])] = &[
    (
        "config/deployments/sandbox.toml",
        &["card.credit: -zift", "card.debit: -zift"],
    ),
    (
        "config/deployments/integration_test.toml",
        &[
            "wallet.apple_pay: -worldpayvantiv",
            "wallet.google_pay: -worldpayvantiv",
        ],
    ),
    (
        "config/development.toml",
        &["bank_redirect.sofort: +stripe"],
    ),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("repo root")
        .to_path_buf()
}

fn load_mandate_matrix(rel: &str) -> MandateMatrix {
    let contents = fs::read_to_string(repo_root().join(rel))
        .unwrap_or_else(|error| panic!("read {rel}: {error}"));
    let config: toml::Table = contents
        .parse()
        .unwrap_or_else(|error| panic!("parse {rel}: {error}"));
    let supported_payment_methods = config
        .get("mandates")
        .and_then(|mandates| mandates.get("supported_payment_methods"))
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{rel} has no [mandates.supported_payment_methods] table"));

    supported_payment_methods
        .iter()
        .flat_map(|(payment_method, payment_method_types)| {
            payment_method_types
                .as_table()
                .unwrap_or_else(|| panic!("{rel}: `{payment_method}` is not a table"))
                .iter()
                .map(move |(payment_method_type, entry)| {
                    let key = format!("{payment_method}.{payment_method_type}");
                    let connectors = entry
                        .get("connector_list")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_else(|| panic!("{rel}: `{key}` has no connector_list"))
                        .split(',')
                        .map(str::trim)
                        .filter(|connector| !connector.is_empty())
                        .map(String::from)
                        .collect();
                    (key, connectors)
                })
        })
        .collect()
}

/// Lists every connector that `actual` lacks (`-`) or adds (`+`) relative to `reference`, per
/// payment method type. A payment method type missing entirely shows up as `-` for each of its
/// connectors.
fn drift(reference: &MandateMatrix, actual: &MandateMatrix) -> BTreeSet<String> {
    let empty = BTreeSet::new();
    let keys: BTreeSet<&String> = reference.keys().chain(actual.keys()).collect();

    keys.into_iter()
        .flat_map(|key| {
            let expected = reference.get(key).unwrap_or(&empty);
            let found = actual.get(key).unwrap_or(&empty);
            let missing = expected
                .difference(found)
                .map(move |connector| format!("{key}: -{connector}"));
            let extra = found
                .difference(expected)
                .map(move |connector| format!("{key}: +{connector}"));
            missing.chain(extra).collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn docker_compose_mandates_match_development() {
    let development = load_mandate_matrix("config/development.toml");
    let docker_compose = load_mandate_matrix("config/docker_compose.toml");

    assert_eq!(
        drift(&development, &docker_compose),
        BTreeSet::new(),
        "config/docker_compose.toml [mandates.supported_payment_methods] differs from \
         config/development.toml"
    );
}

#[test]
fn mandates_match_production_except_known_divergences() {
    let production = load_mandate_matrix(REFERENCE_DEPLOYMENT_CONFIG);

    for (rel, known) in KNOWN_DIVERGENCES {
        let known: BTreeSet<String> = known.iter().copied().map(String::from).collect();

        assert_eq!(
            drift(&production, &load_mandate_matrix(rel)),
            known,
            "{rel} [mandates.supported_payment_methods] differs from \
             {REFERENCE_DEPLOYMENT_CONFIG} in ways not listed in KNOWN_DIVERGENCES"
        );
    }
}
