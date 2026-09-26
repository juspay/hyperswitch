//! Guards the feature matrix against drifting from what connectors implement.
//!
//! `GET /feature_matrix` is built from each connector's `supported_payment_methods`
//! table. That table is a hand-maintained declaration, and nothing ties it to the
//! flows the connector actually implements, so the two drift apart silently. A
//! merchant reads the matrix, attempts the operation, and gets `NotImplemented`
//! from a request that never reached the processor.
//!
//! This is not hypothetical or rare — #14277 (Coinbase), #14372 (Boku),
//! #14373 (Breadpay), #14374 (Coingate) and #14275 (Adyen overcapture) are all
//! instances, and the count below is why a test is cheaper than one issue per
//! connector.
//!
//! # What this checks, and what it deliberately does not
//!
//! Only one rule: a connector must not declare `refunds: Supported` while its
//! refund `Execute` implementation is a stub — `NotImplemented` reachable from
//! `get_url` or `build_request`, so nothing is ever sent.
//!
//! It deliberately does **not** check mandates the same way. A missing
//! `SetupMandate` flow does not mean mandates are unsupported: most connectors
//! support them through `setup_future_usage` on an ordinary payment and never
//! implement the standalone zero-auth flow. Ten of the eleven connectors whose
//! `SetupMandate` returns `NotImplemented` build `MandateReference` and handle
//! `connector_mandate_id` in the authorize path, so a mandate rule of this shape
//! would be ~90% false positives. The refund rule has no such escape hatch:
//! there is one refund `Execute` flow, and if it cannot build a request, refunds
//! do not work.

use std::{collections::BTreeSet, fs, path::Path};

/// Connectors that declare refund support today while their refund `Execute`
/// flow is a stub. Each is a real defect, not an exemption on the merits —
/// the list exists so this test can stop *new* drift immediately instead of
/// waiting for all of them to be fixed, since each fix is a behaviour change
/// that needs its own review.
///
/// Removing a name here is the last step of fixing that connector. Adding one
/// should not happen: if this test fails for a connector not listed, the
/// declaration and the implementation disagree and one of them is wrong.
const KNOWN_REFUND_DECLARATION_DRIFT: &[&str] = &[
    "bitpay",   // #14296
    "breadpay", // #14373
    "etisalat",
    "fiservcommercehub",
    "givepayments",
    "hyperpg",
    "ilixium",
    "imerchantsolutions",
    "merchante",
    "paynearme",
    "worldpayraft",
];

/// The text of the block opened by the first `{` at or after `marker`,
/// brace-matched so a nested block does not end it early.
///
/// Matching on the error *string* instead — which is how I first wrote this —
/// silently misses every stub whose message does not mention refunds, and most
/// of them say `"get_url method"`.
fn impl_block<'a>(source: &'a str, marker: &str) -> Option<&'a str> {
    let start = source.find(marker)?;
    let open = source[start..].find('{')? + start;
    let mut depth = 0usize;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[open..open + offset + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

fn declares_refunds_supported(source: &str) -> bool {
    source.contains("refunds: enums::FeatureStatus::Supported")
        || source.contains("refunds: FeatureStatus::Supported")
}

fn refund_execute_is_a_stub(source: &str) -> bool {
    impl_block(source, "ConnectorIntegration<Execute, RefundsData")
        .map(|block| block.contains("NotImplemented"))
        .unwrap_or(true)
}

fn connector_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/connectors");
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).expect("connector directory must be readable") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("connector file name must be UTF-8")
            .to_string();
        out.push((
            name,
            fs::read_to_string(&path).expect("connector source must be readable"),
        ));
    }
    out.sort();
    out
}

#[test]
fn a_connector_declaring_refunds_must_be_able_to_build_a_refund_request() {
    let drifting: BTreeSet<String> = connector_sources()
        .into_iter()
        .filter(|(_, source)| declares_refunds_supported(source))
        .filter(|(_, source)| refund_execute_is_a_stub(source))
        .map(|(name, _)| name)
        .collect();

    let known: BTreeSet<String> = KNOWN_REFUND_DECLARATION_DRIFT
        .iter()
        .map(|name| (*name).to_string())
        .collect();

    let newly_drifting: Vec<&String> = drifting.difference(&known).collect();
    assert!(
        newly_drifting.is_empty(),
        "these connectors declare `refunds: FeatureStatus::Supported` but their refund \
         Execute flow returns NotImplemented, so /feature_matrix advertises a refund that \
         cannot be attempted: {newly_drifting:?}. Either implement the flow or declare \
         `FeatureStatus::NotSupported`."
    );

    let fixed: Vec<&String> = known.difference(&drifting).collect();
    assert!(
        fixed.is_empty(),
        "these connectors are listed in KNOWN_REFUND_DECLARATION_DRIFT but no longer drift: \
         {fixed:?}. Remove them from the list — leaving a fixed connector there lets it \
         regress unnoticed."
    );
}

#[test]
fn the_brace_matcher_stops_at_the_matching_brace() {
    // Without brace matching, the `NotImplemented` in the *next* impl leaks into
    // this one and every connector after a stubbed one looks broken.
    let source = "\
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    fn build_request(&self) -> R { Ok(nested { inner: 1 }) }
}
impl Other for A {
    fn f(&self) { NotImplemented(\"get_url method\") }
}";
    let block = impl_block(source, "ConnectorIntegration<Execute, RefundsData")
        .expect("the impl block must be found");
    assert!(block.contains("build_request"));
    assert!(
        block.contains("nested"),
        "a nested block must not end the impl early"
    );
    assert!(
        !block.contains("NotImplemented"),
        "the next impl must not leak in"
    );
}

#[test]
fn a_stub_is_detected_however_its_error_is_worded() {
    // The bug in my first pass: matching the message rather than the code. Most
    // stubs say "get_url method", which contains no hint that refunds are meant.
    for message in ["Refund flow not Implemented", "get_url method", ""] {
        let source = format!(
            "refunds: enums::FeatureStatus::Supported,
impl ConnectorIntegration<Execute, RefundsData, X> for A {{
    fn get_url(&self) -> R {{ Err(NotImplemented(\"{message}\".to_string()).into()) }}
}}"
        );
        assert!(declares_refunds_supported(&source));
        assert!(
            refund_execute_is_a_stub(&source),
            "missed a stub worded {message:?}"
        );
    }

    let real = "\
refunds: enums::FeatureStatus::Supported,
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    fn get_url(&self) -> R { Ok(format!(\"{}/refunds\", self.base_url())) }
}";
    assert!(
        !refund_execute_is_a_stub(real),
        "a real implementation must not be flagged"
    );
}
