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
//!   cargo test -p router --test deja_tape_conformance --features deja,v1 -- --ignored --nocapture
//! ```
//!
//! The `deja` feature is REQUIRED, not incidental: this harness probes the very
//! types deja records, and several of them (`redis_interface`'s reply enums, the
//! `http_outgoing` codec) exist or gain serde only under that feature. Hence the
//! crate-level `cfg` below — without it a `--features v1 --all-targets` build
//! fails to compile the test rather than skipping it.
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

#![cfg(feature = "deja")]
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
/// Reconstruct, then RE-capture and prove nothing was dropped.
///
/// This used to be `Ok(_) => Reconstructed` — it discarded the rebuilt value and
/// so tested "the payload parses", not "the payload survives". Those differ
/// exactly where it has hurt: `Serialize` and `Deserialize` are independent
/// impls, and every codec defect this gate exists for parsed fine while silently
/// dropping data. `DirValue::Connector` carried `skip_deserializing`;
/// `Encryptable`'s hand-written `Deserialize` rebuilt with an empty ciphertext.
/// Both would have passed a parse check, and both cost a sandbox cycle.
///
/// The property is one-directional on purpose: every value the tape carries must
/// come back. A re-capture that ADDS something (an absent optional field
/// materializing as `null`) loses nothing and is not a defect, so this asks
/// whether the tape's own values survived — not for byte equality, which would
/// fire on benign asymmetries and train everyone to ignore it.
fn probe<T: serde::de::DeserializeOwned + serde::Serialize>(value: &Value) -> Coverage {
    let rebuilt = match serde_json::from_value::<T>(value.clone()) {
        Ok(rebuilt) => rebuilt,
        Err(error) => return Coverage::Failed(format!("{}: {error}", std::any::type_name::<T>())),
    };
    let recaptured = match serde_json::to_value(&rebuilt) {
        Ok(recaptured) => recaptured,
        Err(error) => {
            return Coverage::Failed(format!(
                "{}: reconstructed, but will not re-serialize: {error}",
                std::any::type_name::<T>()
            ))
        }
    };
    match first_dropped_path(value, &recaptured, String::new()) {
        None => Coverage::Reconstructed,
        Some(path) => {
            // Carry BOTH sides. A path alone says a value was lost without
            // saying how, which leaves the reader to guess at precision, format
            // or ordering — the same anonymity that made a codec failure look
            // like a transport fault.
            let at = |root: &Value| -> String {
                let mut cursor = root;
                for step in path.split('.') {
                    let (name, index) = match step.split_once('[') {
                        Some((name, rest)) => (name, rest.trim_end_matches(']').parse::<usize>().ok()),
                        None => (step, None),
                    };
                    if !name.is_empty() && name != "(root)" {
                        match cursor.get(name) {
                            Some(next) => cursor = next,
                            None => return "<absent>".to_owned(),
                        }
                    }
                    if let Some(index) = index {
                        match cursor.get(index) {
                            Some(next) => cursor = next,
                            None => return "<absent>".to_owned(),
                        }
                    }
                }
                let rendered = cursor.to_string();
                rendered.chars().take(120).collect()
            };
            Coverage::Failed(format!(
                "{}: reconstructed but LOSSY at `{path}` — the tape has {}, the round trip \
                 produced {}. Replay would substitute a value the recording never had.",
                std::any::type_name::<T>(),
                at(value),
                at(&recaptured)
            ))
        }
    }
}

/// The first path at which `recaptured` fails to carry what `original` had.
///
/// Walks the tape's payload, not the re-captured one: additions are fine, losses
/// are not. A null in the tape asserts nothing (the field was absent-or-empty
/// either way).
fn first_dropped_path(original: &Value, recaptured: &Value, path: String) -> Option<String> {
    let here = || {
        if path.is_empty() {
            "(root)".to_owned()
        } else {
            path.clone()
        }
    };
    match original {
        Value::Null => None,
        Value::Object(fields) => {
            // A shape change is a TOTAL loss, not an absence of loss. `?` here
            // returned `None`, which this function defines as "nothing was
            // dropped" — so a codec that turns `{inner, encrypted}` into a bare
            // string passed as clean. That is precisely the `Encryptable` /
            // `skip_deserializing` class the gate was written to catch.
            let Some(target) = recaptured.as_object() else {
                return Some(here());
            };
            for (key, child) in fields {
                let next = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match target.get(key) {
                    Some(found) => {
                        if let Some(dropped) = first_dropped_path(child, found, next) {
                            return Some(dropped);
                        }
                    }
                    // A key the tape carried that the round trip does not
                    // produce at all — unless it carried nothing.
                    None if !child.is_null() => return Some(next),
                    None => {}
                }
            }
            None
        }
        // Arrays compare as MULTISETS, not position-wise. A captured collection
        // may be backed by a hash set — `ConstraintGraph`'s `InAggregator` is an
        // `FxHashSet`, so its JSON order is nondeterministic and a position-wise
        // check reports reordering as loss. It did, on 50 graph values, which is
        // exactly the false alarm that teaches people to ignore a gate. A
        // multiset still catches the property under test: a dropped or altered
        // element changes it. Pure reordering is a separate concern, and the
        // scorer already has order-nondeterminism handling for it.
        Value::Array(items) => {
            let Some(target) = recaptured.as_array() else {
                return Some(here());
            };
            if target.len() != items.len() {
                return Some(format!("{}[len]", here()));
            }
            // Position-wise FIRST. The multiset scan below is O(N^2) deep
            // compares, and a captured `ConstraintGraph`'s `nodes`/`edges`
            // serialize as arrays with thousands of entries — ~10^8 nested
            // comparisons, which hangs the gate rather than failing it. Order is
            // preserved for every array except the hash-set-backed ones, so the
            // ordered pass answers almost all of them in O(N).
            if items
                .iter()
                .zip(target)
                .all(|(child, found)| first_dropped_path(child, found, String::new()).is_none())
            {
                return None;
            }
            // Fall back to multiset semantics only when the ordered pass failed:
            // `InAggregator` is an `FxHashSet`, so its JSON order is
            // nondeterministic and reordering must not read as loss.
            let mut remaining: Vec<&Value> = target.iter().collect();
            for (index, child) in items.iter().enumerate() {
                match remaining
                    .iter()
                    .position(|found| first_dropped_path(child, found, String::new()).is_none())
                {
                    Some(at) => {
                        remaining.swap_remove(at);
                    }
                    None => return Some(format!("{}[{index}]", here())),
                }
            }
            None
        }
        scalar => (scalar != recaptured).then(here),
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
        // Both reported UNREGISTERED by a gate run against a real sbx tape
        // (`rec-bb14832-08121241-ki`): substitutable, reconstructed, and until
        // now invisible to this gate. `GatewayStatusMap` only derives
        // `Deserialize` under the `deja` feature, which the crate-level `cfg`
        // above guarantees is on.
        "diesel_models::capture::Capture" => probe::<dm::capture::Capture>(value),
        "alloc::vec::Vec<diesel_models::capture::Capture>" => {
            probe::<Vec<dm::capture::Capture>>(value)
        }
        "diesel_models::gsm::GatewayStatusMap" => probe::<dm::gsm::GatewayStatusMap>(value),
        "alloc::vec::Vec<diesel_models::gsm::GatewayStatusMap>" => {
            probe::<Vec<dm::gsm::GatewayStatusMap>>(value)
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

/// The registry for `deja::id` sites, keyed by the recorded OPERATION.
///
/// An `id` value carries no `type_name` to key on. The DB and keymanager codecs
/// stamp a `{version, result, value, type_name}` envelope, but `deja::id` uses
/// `SerdeCodec` / `ResultOkCodec`, whose capture path
/// (`deja::value::result_serialize{,_ok}`) records the BARE value. The operation
/// is what the tape does carry for every such site (`method_name`), so that is
/// the key.
///
/// These are `Substitute` boundaries — every preset except redis defaults to it
/// — so an unreconstructable value here fail-stops the call and takes the
/// worker with it, exactly like the substituted DB reads the typed registry
/// above guards. The gate suppressed them until now.
fn probe_by_id_operation(operation: &str, value: &Value) -> Coverage {
    match operation {
        // Generated identifiers, the argon2 hash, and the JWE compact
        // serialization — all `String`.
        "generate_id"
        | "generate_id_with_default_len"
        | "generate_id_with_len"
        | "generate_time_ordered_id"
        | "generate_time_ordered_id_without_prefix"
        | "generate_cryptographically_secure_random_string"
        | "create_merchant_publishable_key"
        | "generate_user_id"
        | "generate_password_hash"
        | "encrypt_jwe" => probe::<String>(value),

        // The merchant's AES-256 key, recorded Ok-only (`ring`'s error type is
        // not serializable).
        "generate_aes256_key" => probe::<[u8; 32]>(value),

        // A JWT's absolute expiry, recorded Ok-only.
        "generate_exp" => probe::<std::time::Duration>(value),

        // `common_utils::crypto::NonceSequence` is private to its module, so the
        // gate cannot name it. It is a newtype over a `u128` written as its 16
        // big-endian bytes, so `[u8; 16]` is its serde image exactly — but an
        // image is not the type, and this stops being exact the moment that
        // struct changes shape. Stamping the captured type at capture time is
        // what would make it precise (and would also delete the polymorphic-cache
        // guess below).
        "GcmAes256::nonce" => probe::<[u8; 16]>(value),

        _ => Coverage::Unregistered,
    }
}

/// `http_outgoing` rebuilds through a CUSTOM codec rather than serde, so the
/// gate asks that codec instead of re-deriving what it will accept. Anything
/// else here would be a second implementation of the rule, free to drift from
/// the one replay actually runs.
///
/// It is a `Substitute` boundary and the most expensive one to get wrong: on
/// run-0812 a single miss at `/cards/add` fail-stopped the call, killed the
/// worker, and cascaded into 8 dead correlations and 178 of 182 omitted calls.
///
/// There is no re-capture half. `HttpResponseCodec::capture` reads a body
/// smuggled out of the live send path through `http::Extensions`, which a
/// rebuilt response does not carry — so this proves the recorded response
/// reconstructs, not that it survives a round trip. A recorded ERROR carries no
/// `status` and is `is_error`, so it never reaches here.
fn probe_http_outgoing(value: &Value) -> Coverage {
    use deja::codec::ReplayCodec;
    use external_services::http_client::boundary::HttpResponseCodec;

    match HttpResponseCodec::reconstruct(value.clone()) {
        Some(_) => Coverage::Reconstructed,
        None => Coverage::Failed(
            "the recorded response will not rebuild — `HttpResponseCodec` needs a numeric \
             `status` it can read as an HTTP status code, and a `response_headers` map it can \
             build a header map from. Replay would fail-stop this call and the candidate's \
             worker would die mid-request."
                .to_owned(),
        ),
    }
}

/// Route an `imc` line by the operation that produced it.
///
/// All four cache boundaries carry the same `{"cache": …}` args, but only
/// `Cache::get_val` returns the cached value. Keying on the cache name alone
/// therefore handed `Cache::exists`'s `true` to `probe::<dm::configs::Config>`,
/// which fails — and one `Failed` fails the WHOLE tape with a spurious "replay
/// would fail-stop the boundary". `push`/`remove` escaped it only by accident:
/// `()` serializes to `null`, which `coverage` answers before any probe runs.
fn probe_by_imc_operation(operation: &str, cache: Option<&str>, value: &Value) -> Coverage {
    match operation {
        "in_memory_get" => match cache {
            Some(cache) => probe_by_cache_name(cache, value),
            // A cached value with no cache name is unaddressable: the registry
            // is keyed by name and the line says nothing else to key on.
            None => Coverage::Unregistered,
        },
        "in_memory_exists" => probe::<bool>(value),
        "in_memory_push" | "in_memory_remove" => probe::<()>(value),
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
    /// The instrumented site's operation — `method_name`, present at the top
    /// level of BOTH shapes. For every boundary whose codec records a bare value
    /// this is the only thing on the line that says which type it should become.
    operation: Option<String>,
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
        let operation = object
            .get("method_name")
            .and_then(Value::as_str)
            .map(str::to_owned);
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
            operation,
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
        // Everything below records a BARE value, so there is no `type_name` to
        // dispatch on and the operation carries the type instead.
        let operation = self.operation.as_deref().unwrap_or_default();
        match self.boundary.as_str() {
            "imc" => probe_by_imc_operation(operation, self.cache.as_deref(), value),
            "id" => probe_by_id_operation(operation, value),
            "http_outgoing" => probe_http_outgoing(value),
            "redis" if is_capture_ack(value) => Coverage::ValueDiscardedByCapture,
            _ => Coverage::Unregistered,
        }
    }

    /// How this line is reported: what it was captured from, and — where the
    /// type is not on the line — which site produced it, since that is what the
    /// reader has to go and register.
    fn label(&self) -> String {
        match (&self.type_name, &self.cache, &self.operation) {
            (Some(type_name), _, _) => format!("{} [{type_name}]", self.boundary),
            (None, Some(cache), Some(operation)) => {
                format!("{} [{operation} cache {cache}]", self.boundary)
            }
            (None, Some(cache), None) => format!("{} [cache {cache}]", self.boundary),
            (None, None, Some(operation)) => format!("{} [{operation}]", self.boundary),
            (None, None, None) => format!("{} [untyped]", self.boundary),
        }
    }
}

/// Boundaries whose values replay does not reconstruct through a codec, so an
/// unregistered value there is not a gap. `time` is the deterministic-live tier
/// (scalars), `http_incoming` is the request replay drives rather than
/// substitutes, and `superposition` values are plain JSON config resolved
/// without a typed target.
///
/// `id` and `http_outgoing` used to be on this list and should never have been.
/// Both default to `Substitute`, so both reconstruct a recorded value and
/// fail-stop when they cannot — the gate was exempting precisely the two
/// boundaries whose reconstruction failures kill a replay worker. `id` now
/// carries `encrypt_jwe`'s compact JWE, and `http_outgoing` is where run-0812's
/// miss on it became 8 dead correlations. They are dispatched by operation
/// above; an `id` operation nobody registered is reported as a gap, which is the
/// point.
///
/// `time` is the remaining one of this shape: it is `Substitute` too, and its
/// values would need the same operation registry. Left alone deliberately — the
/// timestamp codec's known defect is a lost millisecond on ROUND TRIP, which the
/// typed registry already catches on `PaymentIntent.created_at`, not a failure
/// to reconstruct.
const NOT_RECONSTRUCTED: &[&str] = &["time", "http_incoming", "superposition"];

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
