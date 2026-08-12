//! Every value a replay would substitute must be readable back.
//!
//! Replay hands a boundary the value its recording captured, reconstructed
//! through the boundary's codec. A type that SERIALIZES but does not
//! DESERIALIZE therefore fails at replay time and nowhere earlier: the
//! compiler cannot see it, and no test that only ever writes a value will
//! catch it. `DirValue::Connector` and `DirKeyKind::Connector` carried
//! `skip_deserializing` for years exactly this way, harmless until a recorded
//! constraint graph became substitutable — then reconstruction of the whole
//! graph failed on that one variant, the boundary fail-stopped, and sixteen of
//! forty-four replayed requests closed their connection without a response.
//! Three sandbox cycles were spent on that class.
//!
//! This is the cheap gate for it: point the harness at a REAL recording and
//! assert that every recorded value reconstructs into the type it was captured
//! from. Real recordings are what make it worth running — the graph that broke
//! only broke because production traffic put a connector node in it, which no
//! hand-written fixture would have thought to do.
//!
//! # Running it
//!
//! ```text
//! DEJA_TAPE=/path/to/events.jsonl \
//!   cargo test -p router --test deja_tape_conformance --features v1 -- --ignored --nocapture
//! ```
//!
//! `#[ignore]` is deliberate: the harness needs a recording, CI has none, and a
//! test that silently passes when its input is missing is worse than no test.
//! Run it before pushing anything that changes a captured type or a codec.
//!
//! Accepts either shape of recorded line, since the tape itself is deliberately
//! not reachable through the orchestrator API (the scope invariant) while the
//! call ledger is:
//!   - a tape event:   `{"boundary":…,"args":…,"result":…}`
//!   - a ledger row:   `{"boundary":…,"recorded":{"args":…,"result":…}}`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

/// What the harness could establish about one recorded value.
#[derive(Debug, PartialEq, Eq)]
enum Coverage {
    /// Reconstructed into the type it was captured from.
    Reconstructed,
    /// Present, typed, and NOT reconstructable — the defect this gate exists
    /// for.
    Failed(String),
    /// Nothing in this file knows what type this value should become. Not a
    /// pass: an unregistered substitutable type is an unguarded one, and the
    /// harness reports it as a gap rather than swallowing it.
    Unregistered,
    /// Carries no value to reconstruct (an error result, a miss, a boundary
    /// whose codec substitutes nothing).
    NoValue,
    /// The capture itself throws the value away, so replay has nothing to
    /// substitute. Not a failure of this gate, but not a pass either — it is a
    /// fidelity gap in the recording, reported so it stays visible.
    ValueDiscardedByCapture,
}

/// Try to deserialize `value` as `T`, naming the failure the way replay would
/// see it.
fn probe<T: serde::de::DeserializeOwned>(value: &Value) -> Coverage {
    match serde_json::from_value::<T>(value.clone()) {
        Ok(_) => Coverage::Reconstructed,
        Err(error) => Coverage::Failed(format!("{}: {error}", std::any::type_name::<T>())),
    }
}

/// Try every candidate type, for a value whose target type the recording does
/// not state — a cache that holds several kinds. Reconstructing as ANY of them
/// is the property under test: that the value is readable at all. It does not
/// prove the right type won, which is why single-type caches are keyed
/// directly and only genuinely polymorphic ones come here.
fn probe_any(candidates: &[(&str, fn(&Value) -> Coverage)], value: &Value) -> Coverage {
    let mut failures = Vec::new();
    for (name, candidate) in candidates {
        match candidate(value) {
            Coverage::Reconstructed => return Coverage::Reconstructed,
            Coverage::Failed(why) => failures.push(format!("{name}: {why}")),
            other => return other,
        }
    }
    Coverage::Failed(format!(
        "no candidate type could read it — {}",
        failures.join(" | ")
    ))
}

/// The registry, keyed by the `type_name` the codec stamped on the recorded
/// envelope. Adding a substitutable type means adding a line here; a type that
/// reaches a tape without one is reported as `Unregistered`.
fn probe_by_type_name(type_name: &str, value: &Value) -> Coverage {
    use common_utils::types::keymanager as km;
    use diesel_models as dm;

    match type_name {
        // Keymanager — substituted so replay reproduces ciphertext byte-exactly
        // and needs no live key manager.
        "common_utils::types::keymanager::EncryptDataResponse" => {
            probe::<km::EncryptDataResponse>(value)
        }
        "common_utils::types::keymanager::DecryptDataResponse" => {
            probe::<km::DecryptDataResponse>(value)
        }
        "common_utils::types::keymanager::BatchEncryptDataResponse" => {
            probe::<km::BatchEncryptDataResponse>(value)
        }
        "common_utils::types::keymanager::BatchDecryptDataResponse" => {
            probe::<km::BatchDecryptDataResponse>(value)
        }

        // Redis wire values and reply enums.
        "deja::value::RedisWireValue" => probe::<deja::value::RedisWireValue>(value),
        "redis_interface::types::DelReply" => probe::<redis_interface::types::DelReply>(value),
        "redis_interface::types::SetnxReply" => probe::<redis_interface::types::SetnxReply>(value),

        // Database rows, single and multi.
        "diesel_models::payment_intent::PaymentIntent" => probe::<dm::PaymentIntent>(value),
        "alloc::vec::Vec<diesel_models::payment_intent::PaymentIntent>" => {
            probe::<Vec<dm::PaymentIntent>>(value)
        }
        "diesel_models::payment_attempt::PaymentAttempt" => probe::<dm::PaymentAttempt>(value),
        "alloc::vec::Vec<diesel_models::payment_attempt::PaymentAttempt>" => {
            probe::<Vec<dm::PaymentAttempt>>(value)
        }
        "diesel_models::customers::Customer" => probe::<dm::Customer>(value),
        "alloc::vec::Vec<diesel_models::customers::Customer>" => probe::<Vec<dm::Customer>>(value),
        "diesel_models::address::Address" => probe::<dm::Address>(value),
        "alloc::vec::Vec<diesel_models::address::Address>" => probe::<Vec<dm::Address>>(value),
        "diesel_models::merchant_account::MerchantAccount" => probe::<dm::MerchantAccount>(value),
        "alloc::vec::Vec<diesel_models::merchant_account::MerchantAccount>" => {
            probe::<Vec<dm::MerchantAccount>>(value)
        }
        "diesel_models::merchant_key_store::MerchantKeyStore" => {
            probe::<dm::merchant_key_store::MerchantKeyStore>(value)
        }
        "diesel_models::merchant_connector_account::MerchantConnectorAccount" => {
            probe::<dm::MerchantConnectorAccount>(value)
        }
        "alloc::vec::Vec<diesel_models::merchant_connector_account::MerchantConnectorAccount>" => {
            probe::<Vec<dm::MerchantConnectorAccount>>(value)
        }
        "diesel_models::business_profile::Profile" => probe::<dm::business_profile::Profile>(value),
        "diesel_models::organization::Organization" => {
            probe::<dm::organization::Organization>(value)
        }
        "diesel_models::api_keys::ApiKey" => probe::<dm::api_keys::ApiKey>(value),
        "diesel_models::configs::Config" => probe::<dm::configs::Config>(value),
        "diesel_models::payment_method::PaymentMethod" => probe::<dm::PaymentMethod>(value),
        "alloc::vec::Vec<diesel_models::payment_method::PaymentMethod>" => {
            probe::<Vec<dm::PaymentMethod>>(value)
        }
        "diesel_models::process_tracker::ProcessTracker" => {
            probe::<dm::process_tracker::ProcessTracker>(value)
        }
        "diesel_models::refund::Refund" => probe::<dm::Refund>(value),
        "alloc::vec::Vec<diesel_models::refund::Refund>" => probe::<Vec<dm::Refund>>(value),
        "diesel_models::dispute::Dispute" => probe::<dm::Dispute>(value),
        "alloc::vec::Vec<diesel_models::dispute::Dispute>" => probe::<Vec<dm::Dispute>>(value),
        "diesel_models::authorization::Authorization" => {
            probe::<dm::authorization::Authorization>(value)
        }
        "alloc::vec::Vec<diesel_models::authorization::Authorization>" => {
            probe::<Vec<dm::authorization::Authorization>>(value)
        }

        // Key-manager key lifecycle.
        "common_utils::types::keymanager::DataKeyCreateResponse" => {
            probe::<km::DataKeyCreateResponse>(value)
        }

        // Scalars the codec stamps.
        "bool" => probe::<bool>(value),
        "()" => Coverage::NoValue,
        _ => Coverage::Unregistered,
    }
}

/// The in-memory cache substitutes the cached value itself, and its codec
/// stamps no `type_name` — the call site's `get_val::<T>` is monomorphic, so the
/// tape never needed to say. Caches that hold exactly one type are therefore
/// keyed by cache name; polymorphic ones (accounts, config) are left to the
/// typed registry above and reported as gaps.
///
/// The two single-type caches are precisely the ones the last regression lived
/// in, which is why they are covered here rather than skipped.
fn probe_by_cache_name(cache: &str, value: &Value) -> Coverage {
    use diesel_models as dm;
    use euclid::frontend::dir::DirValue;
    use hyperswitch_constraint_graph::ConstraintGraph;
    use router::core::payments::routing::CachedAlgorithm;

    match cache {
        // Single-type caches — keyed directly, and the two the last regression
        // lived in.
        "CGRAPH_CACHE" | "PM_FILTERS_CGRAPH_CACHE" => {
            probe::<Arc<ConstraintGraph<DirValue>>>(value)
        }
        "ROUTING_CACHE" => probe::<Arc<CachedAlgorithm>>(value),
        "CONFIG_CACHE" => probe::<dm::configs::Config>(value),

        // Genuinely polymorphic: the accounts cache holds whatever an account
        // lookup put there. Note the profile is the DECRYPTED DOMAIN model
        // (its cache goes through the transform variant) while the others are
        // pre-transform diesel rows — which is exactly why only the profile
        // needed serde added to it.
        "ACCOUNTS_CACHE" => probe_any(
            &[
                (
                    "domain Profile",
                    probe::<hyperswitch_domain_models::business_profile::Profile>,
                ),
                ("MerchantAccount", probe::<dm::MerchantAccount>),
                (
                    "MerchantKeyStore",
                    probe::<dm::merchant_key_store::MerchantKeyStore>,
                ),
                ("ApiKey", probe::<dm::api_keys::ApiKey>),
                (
                    "MerchantConnectorAccount",
                    probe::<dm::MerchantConnectorAccount>,
                ),
            ],
            value,
        ),
        _ => Coverage::Unregistered,
    }
}

/// Whether a recorded redis result is the `get_multiple_keys` acknowledgement.
///
/// That boundary hand-writes its `result =` to `{"ok": true}` / `{"ok": false,
/// error}` instead of capturing the values it read (the return type is generic
/// over the caller's value type). So replay has nothing to substitute for an
/// MGET — a deliberate, pre-existing capture gap, reported rather than counted
/// as covered.
fn is_capture_ack(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.len() <= 2 && object.contains_key("ok"))
}

/// One recorded line, normalized across the tape and ledger shapes.
struct Recorded {
    boundary: String,
    cache: Option<String>,
    type_name: Option<String>,
    /// The value replay would hand back, unwrapped from its codec envelope.
    value: Option<Value>,
    is_error: bool,
}

impl Recorded {
    fn parse(line: &str) -> Option<Self> {
        let raw: Value = serde_json::from_str(line).ok()?;
        let object = raw.as_object()?;
        // A ledger row nests the recording under `recorded`; a tape event
        // carries it at the top level.
        let recorded = object.get("recorded").and_then(Value::as_object);
        let field = |name: &str| {
            recorded
                .and_then(|inner| inner.get(name))
                .or_else(|| object.get(name))
        };

        let boundary = object.get("boundary")?.as_str()?.to_owned();
        let cache = field("args")
            .and_then(Value::as_object)
            .and_then(|args| args.get("cache"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let result = field("result");
        let is_error = field("is_error").and_then(Value::as_bool).unwrap_or(false)
            || result
                .and_then(Value::as_object)
                .and_then(|envelope| envelope.get("result"))
                .and_then(Value::as_str)
                == Some("Err");

        // Unwrap the codec envelope: a typed result carries
        // `{version, result: "Ok", value, type_name}`; an untyped one is the
        // value itself.
        let (type_name, value) = match result {
            Some(Value::Object(envelope)) if envelope.contains_key("version") => (
                envelope
                    .get("type_name")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                envelope.get("value").cloned(),
            ),
            Some(other) => (None, Some(other.clone())),
            None => (None, None),
        };

        Some(Self {
            boundary,
            cache,
            type_name,
            value,
            is_error,
        })
    }

    fn coverage(&self) -> Coverage {
        if self.is_error {
            // A recorded error replays as the same typed error; there is no
            // captured value to reconstruct.
            return Coverage::NoValue;
        }
        let Some(value) = &self.value else {
            return Coverage::NoValue;
        };
        if value.is_null() {
            // A recorded miss (cache absent, row NotFound) substitutes `None`.
            return Coverage::NoValue;
        }
        if let Some(type_name) = &self.type_name {
            return probe_by_type_name(type_name, value);
        }
        if self.boundary == "imc" {
            if let Some(cache) = &self.cache {
                return probe_by_cache_name(cache, value);
            }
        }
        if self.boundary == "redis" && is_capture_ack(value) {
            return Coverage::ValueDiscardedByCapture;
        }
        Coverage::Unregistered
    }

    /// How this line is reported: what it was captured from.
    fn label(&self) -> String {
        match (&self.type_name, &self.cache) {
            (Some(type_name), _) => format!("{} [{type_name}]", self.boundary),
            (None, Some(cache)) => format!("{} [cache {cache}]", self.boundary),
            (None, None) => format!("{} [untyped]", self.boundary),
        }
    }
}

/// Boundaries whose values replay does not reconstruct through a codec, so an
/// unregistered value there is not a gap. `time`/`id` are the deterministic-live
/// tier (scalars), `http_incoming` is the request itself, `http_outgoing` is
/// environmental (egress is sealed), and `superposition` values are plain JSON
/// config resolved without a typed target.
const NOT_RECONSTRUCTED: &[&str] = &[
    "time",
    "id",
    "http_incoming",
    "http_outgoing",
    "superposition",
];

#[test]
#[ignore = "needs a real recording: set DEJA_TAPE to a tape or call-ledger path"]
fn every_recorded_value_reconstructs() {
    let path = std::env::var("DEJA_TAPE").expect(
        "set DEJA_TAPE to a recording (events.jsonl) or a run's call_ledger artifact — \
         this gate is only meaningful against real captured values",
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));

    let mut reconstructed: BTreeMap<String, usize> = BTreeMap::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut unregistered: BTreeMap<String, usize> = BTreeMap::new();
    let mut discarded: BTreeMap<String, usize> = BTreeMap::new();
    let mut no_value = 0usize;
    let mut lines = 0usize;

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Some(record) = Recorded::parse(line) else {
            continue;
        };
        lines += 1;
        let label = record.label();
        match record.coverage() {
            Coverage::Reconstructed => *reconstructed.entry(label).or_default() += 1,
            Coverage::Failed(why) => failed.push((label, why)),
            Coverage::NoValue => no_value += 1,
            Coverage::ValueDiscardedByCapture => *discarded.entry(label).or_default() += 1,
            Coverage::Unregistered => {
                if !NOT_RECONSTRUCTED.contains(&record.boundary.as_str()) {
                    *unregistered.entry(label).or_default() += 1;
                }
            }
        }
    }

    println!("\n=== {path} ===");
    println!("recorded lines: {lines}, values with nothing to reconstruct: {no_value}");
    println!("\nreconstructed:");
    for (label, count) in &reconstructed {
        println!("  {count:5}  {label}");
    }
    if !discarded.is_empty() {
        println!("\ncapture discards the value (replay has nothing to substitute):");
        for (label, count) in &discarded {
            println!("  {count:5}  {label}");
        }
    }
    if !unregistered.is_empty() {
        println!("\nUNREGISTERED (substitutable but unguarded — add a probe):");
        for (label, count) in &unregistered {
            println!("  {count:5}  {label}");
        }
    }
    if !failed.is_empty() {
        println!("\nFAILED to reconstruct:");
        for (label, why) in &failed {
            println!("  {label}\n      {why}");
        }
    }

    assert!(
        failed.is_empty(),
        "{} recorded value(s) cannot be reconstructed — replay would fail-stop the boundary and \
         the candidate's worker would die mid-request. First: {}",
        failed.len(),
        failed
            .first()
            .map(|(label, why)| format!("{label} — {why}"))
            .unwrap_or_default()
    );
    assert!(
        reconstructed.values().sum::<usize>() > 0,
        "no value in {path} was reconstructed — the file parsed but nothing was covered, so this \
         run proves nothing"
    );
    assert!(
        unregistered.is_empty(),
        "substitutable value(s) with no registered probe: {:?} — register them in \
         probe_by_type_name/probe_by_cache_name so they are guarded rather than assumed",
        unregistered.keys().collect::<Vec<_>>()
    );
}
