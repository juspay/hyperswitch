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
    // Empty `impl ConnectorIntegration<Execute, RefundsData, _> for D24 {}`. No
    // `NotImplemented` text anywhere: it inherits the trait default, which
    // returns `Ok(None)` and increments `UNIMPLEMENTED_FLOW`. No request is ever
    // built, so refunds cannot work — the same drift, reached a different way.
    "d24",
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
/// brace-matched with string literals, char literals and comments skipped.
///
/// Counting raw braces is not enough: a `{}` inside a `format!` string or a doc
/// comment ends the block early, and every impl after it is then read against
/// the wrong slice. Rust source in this directory contains both.
///
/// Matching on the error *string* instead — which is how I first wrote this —
/// silently misses every stub whose message does not mention refunds, and most
/// of them say `"get_url method"`.
fn impl_block<'a>(source: &'a str, marker: &str) -> Option<&'a str> {
    let start = source.find(marker)?;
    let bytes = source.as_bytes();
    let open = source[start..].find('{')? + start;

    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        let rest = &source[i..];

        // Comments
        if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
            continue;
        }
        if rest.starts_with("/*") {
            i += rest[2..].find("*/").map(|n| n + 4).unwrap_or(rest.len());
            continue;
        }

        // Raw strings: r"...", r#"..."#, r##"..."##
        if rest.starts_with('r') {
            let hashes = rest[1..].chars().take_while(|c| *c == '#').count();
            if rest[1 + hashes..].starts_with('"') {
                let closing = format!("\"{}", "#".repeat(hashes));
                let body = &rest[1 + hashes + 1..];
                i += 1
                    + hashes
                    + 1
                    + body
                        .find(&closing)
                        .map(|n| n + closing.len())
                        .unwrap_or(body.len());
                continue;
            }
        }

        // A lone `'` is far more often a lifetime (`'a`, `'static`) than a char
        // literal, and treating `&'static str` as an opening quote scans to the
        // next apostrophe anywhere in the file, straight past the braces. That
        // is what flagged payeezy, payload and powertranz as stubs when all
        // three have real refund implementations.
        //
        // A char literal is `'x'` or `'\n'`; a lifetime is not followed by a
        // closing quote two bytes on and does not begin with a backslash.
        let is_char_literal = bytes[i] == b'\''
            && (bytes.get(i + 1) == Some(&b'\\') || bytes.get(i + 2) == Some(&b'\''));
        if is_char_literal {
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == b'\'' {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }

        // Ordinary strings, honouring backslash escapes.
        if bytes[i] == b'"' {
            let quote = bytes[i];
            let mut j = i + 1;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == quote {
                    j += 1;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }

        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&source[open..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn declares_refunds_supported(source: &str) -> bool {
    source.contains("refunds: enums::FeatureStatus::Supported")
        || source.contains("refunds: FeatureStatus::Supported")
}

/// Whether the refund `Execute` flow cannot build a request.
///
/// Three shapes, not one. Matching only `NotImplemented` missed the second:
///
/// 1. no impl at all,
/// 2. an **empty** `impl ... for X {}`, which inherits the trait default —
///    `build_request` returns `Ok(None)` and bumps `UNIMPLEMENTED_FLOW`, so no
///    request is ever built and the text `NotImplemented` appears nowhere,
/// 3. an impl whose body reaches `NotImplemented`.
fn refund_execute_is_a_stub(source: &str) -> bool {
    let Some(block) = impl_block(source, "ConnectorIntegration<Execute, RefundsData") else {
        return true;
    };
    let body = block
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .unwrap_or(block)
        .trim();
    body.is_empty() || block.contains("NotImplemented")
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
fn an_empty_impl_is_a_stub_because_the_trait_default_builds_no_request() {
    // The miss XyneSpaces found on #14437. `d24` ships exactly this shape: the
    // text `NotImplemented` appears nowhere, because the default `build_request`
    // returns `Ok(None)` and increments `UNIMPLEMENTED_FLOW`. A detector keyed on
    // that string calls this connector clean while refunds cannot work at all.
    let empty = "\
refunds: enums::FeatureStatus::Supported,
impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for D24 {}";
    assert!(declares_refunds_supported(empty));
    assert!(
        refund_execute_is_a_stub(empty),
        "an empty impl must count as a stub"
    );

    let whitespace_only = "\
impl ConnectorIntegration<Execute, RefundsData, X> for A {

}";
    assert!(refund_execute_is_a_stub(whitespace_only));
}

#[test]
fn braces_inside_strings_and_comments_do_not_end_the_block() {
    // Counting raw braces ends the impl at the `{}` in the format string, so the
    // real body — and its NotImplemented — falls outside the slice and the
    // connector reads as clean.
    // The comment and string braces sit BEFORE the body on purpose. Counted, they
    // close the impl early and everything after — including the NotImplemented
    // that makes this a stub — falls outside the slice.
    let source = r###"
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    // a closing brace in a comment: }
    /* and a block comment: } */
    fn get_url(&self) -> R {
        let _ = format!("{} refunds {{literal}}", self.base);
        let _brace = '}';
        Err(NotImplemented("get_url method".to_string()).into())
    }
}
"###;
    let block = impl_block(source, "ConnectorIntegration<Execute, RefundsData")
        .expect("the impl block must be found");
    assert!(
        block.contains("NotImplemented"),
        "the real body must stay inside the slice"
    );
    assert!(refund_execute_is_a_stub(source));
}

#[test]
fn a_lifetime_is_not_mistaken_for_a_char_literal() {
    // Regression. Treating the `'` in `&'static str` as an opening quote scans
    // to the next apostrophe anywhere in the file, past the closing brace, and
    // swallows whatever impl comes next. That flagged payeezy, payload and
    // powertranz — all three with real refund implementations — as stubs.
    let source = r###"
refunds: enums::FeatureStatus::Supported,
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    fn get_content_type(&self) -> &'static str { "application/json" }
    fn get_url(&self) -> R { Ok(format!("{}/refunds", self.base_url())) }
}
impl SomethingElse for A {
    fn other(&self) -> R { Err(NotImplemented("get_url method".to_string()).into()) }
}
"###;
    let block = impl_block(source, "ConnectorIntegration<Execute, RefundsData")
        .expect("the impl block must be found");
    assert!(
        block.contains("get_url"),
        "the real body must be inside the slice"
    );
    assert!(
        !block.contains("NotImplemented"),
        "the next impl must not be swallowed by a lifetime read as a quote"
    );
    assert!(!refund_execute_is_a_stub(source));

    // A genuine char literal must still be skipped, braces and all. The brace
    // here is an *opening* one and deliberately unbalanced: a closing brace in a
    // char literal can miscount and still land on the right byte by accident,
    // because the extra decrement is cancelled by the real closer. An unmatched
    // `{` cannot cancel — miscount it and the block runs past its own end and
    // swallows whatever impl follows.
    let with_char = r###"
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    fn f(&self) { let _ = '{'; let _ = '\''; }
}
impl SomethingElse for A {
    fn other(&self) { Err(NotImplemented("get_url method".to_string()).into()) }
}
"###;
    let block =
        impl_block(with_char, "ConnectorIntegration<Execute, RefundsData").expect("block found");
    assert!(
        block.contains("let _ = '{'"),
        "a char literal brace must not end the block"
    );
    assert!(
        !block.contains("NotImplemented"),
        "an unbalanced brace in a char literal must not extend the block"
    );
}

#[test]
fn a_real_implementation_with_braces_in_a_string_is_not_flagged() {
    // The other direction: the literal-skipping must not make a working
    // connector look like a stub.
    let source = r###"
refunds: enums::FeatureStatus::Supported,
impl ConnectorIntegration<Execute, RefundsData, X> for A {
    fn get_url(&self) -> R {
        Ok(format!("{}/v1/refunds/{{id}}", self.base_url()))
    }
}
"###;
    assert!(!refund_execute_is_a_stub(source));
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
