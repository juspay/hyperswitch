//! The card-on-file record ACI keeps for one registration, and the per-attempt state that lets a
//! 3DS-challenged CIT write it only once it is complete.
//!
//! A MIT quotes three things the CIT agreed with the acquirer: the scheme trace id (CITI), the
//! Mastercard agreement id (TLID), and the `standingInstruction.type` the series was registered
//! under. They belong to the registration, not to the card, so they live together in the
//! mandate record of the MCA that holds the registration (`mandate_metadata`), stamped with the
//! registration they were captured for.
//!
//! The record is written once per registration, from the final response of the CIT that created
//! it. Core writes a payment method's mandate record while it is not yet Active, and rewrites an
//! Active ACI record only when a CIT's finalising response brings a different registration (a
//! re-CIT supersedes the old agreement; see `connector_replaces_mandate_on_new_cit` in core). A
//! legacy mandate row is filled only while it has no connector mandate id. ACI's part is to emit
//! nothing until the CIT is final and then emit the whole record, so the first write for a
//! registration is the complete one — and never to emit a registration's stamp from anything but
//! that registration's own CIT, since a differing registration on a CIT now replaces the record.
//! Everything here is ACI-owned JSON; core neither reads nor merges inside it.

use common_utils::pii::SecretSerdeValue;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use super::scheme_trace_id::SchemeTraceId;

/// Marks a mandate record's metadata as written by this module, and the shape it was written in.
const RECORD_MARKER: &str = "aciCof";
const RECORD_VERSION: u8 = 1;
/// Key of the per-attempt state in `payment_attempt.connector_metadata`.
const STASH_KEY: &str = "aciCof";

/// The CIT's card-on-file values for one ACI registration.
///
/// A stamped record is authoritative: a value it lacks really was absent on the CIT (a Visa card
/// has no agreement id), and the reader must not fill it from anywhere else. The one reading of
/// absence that differs is `siType`: a CIT that sent no type (MOTO) registered no series type,
/// so the MIT declares its own.
///
/// `agreementId` and `siType` keep the key names unstamped metadata already used, so a build
/// that predates this module still reads them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AciCofRecord {
    /// The registration this record was captured for. A record whose `rg` is not the
    /// registration being charged is not used: core merges a mandate record field by field
    /// (`new.or(existing)`), and the binding is what stops a stale record being paired with a
    /// newer registration.
    pub rg: String,
    /// The CITI exactly as the acquirer returned it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_transaction_id: Option<String>,
    /// The Mastercard agreement id (TLID).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<String>,
    /// The `standingInstruction.type` the CIT actually sent, including the UNSCHEDULED default.
    /// Absent when none went out (a MOTO CIT's block is stripped to `createRegistration`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub si_type: Option<String>,
}

/// What a mandate record's metadata says about the registration being charged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CofRecordView {
    /// Stamped for this registration: authoritative.
    Stamped(AciCofRecord),
    /// Stamped, but for another registration (or with no registration at all). None of its
    /// values may be used for this one.
    Unbound { stamped_rg: Option<String> },
    /// Written before records were stamped. Its keys are hints that the reader has to
    /// corroborate.
    Legacy {
        agreement_id: Option<String>,
        si_type: Option<String>,
    },
}

impl AciCofRecord {
    pub(super) fn new(
        rg: String,
        initial_transaction_id: Option<String>,
        agreement_id: Option<String>,
        si_type: Option<String>,
    ) -> Self {
        Self {
            rg,
            // Only a value that is a scheme reference may be recorded as the CITI. Callers pass
            // values that `resolve_citi` already accepted; this keeps the record's invariant
            // local to the record.
            initial_transaction_id: initial_transaction_id
                .filter(|raw| SchemeTraceId::parse(raw).is_ok()),
            agreement_id: agreement_id.filter(|value| !value.trim().is_empty()),
            si_type,
        }
    }

    /// The record as mandate metadata.
    pub(super) fn to_metadata(&self) -> SecretSerdeValue {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let Some(object) = value.as_object_mut() {
            object.insert(RECORD_MARKER.to_string(), RECORD_VERSION.into());
        }
        Secret::new(value)
    }

    /// Read mandate metadata for the registration `charged_rg`.
    ///
    /// Lenient by design: this runs on the MIT path, where refusing to build the request over a
    /// malformed key would be worse than omitting the value. Unknown keys are ignored and a
    /// value of the wrong type reads as absent.
    pub(super) fn from_metadata(
        metadata: Option<&serde_json::Value>,
        charged_rg: Option<&str>,
    ) -> CofRecordView {
        let string = |key: &str| {
            metadata
                .and_then(|meta| meta.get(key))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(String::from)
        };

        let is_stamped = metadata
            .and_then(|meta| meta.get(RECORD_MARKER))
            .is_some_and(|marker| !marker.is_null());
        if !is_stamped {
            return CofRecordView::Legacy {
                agreement_id: string("agreementId"),
                si_type: string("siType"),
            };
        }

        match string("rg") {
            Some(rg) if charged_rg == Some(rg.as_str()) => CofRecordView::Stamped(Self {
                rg,
                initial_transaction_id: string("initialTransactionId"),
                agreement_id: string("agreementId"),
                si_type: string("siType"),
            }),
            stamped_rg => CofRecordView::Unbound { stamped_rg },
        }
    }
}

/// Which side of the card-on-file agreement an attempt was sent as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum CofRole {
    Cit,
    Mit,
}

/// Per-attempt card-on-file state, kept in `payment_attempt.connector_metadata`.
///
/// A 3DS-challenged CIT is finished by a PSync, and the sync request does not say what the
/// payment was sent as: its `mandate_id` comes from the attempt, which is set for a legacy
/// mandate CIT and unset for a token MIT, the opposite of what the response path needs. So the
/// authorize response records the role here, and the sync reads it back from `connector_meta`.
///
/// `citi` and `agreementId` are only ever written from a final CIT response. A later sync of the
/// same attempt (a manual-capture CIT synced in `requires_capture`) re-emits exactly these, so
/// every write of the record carries the same values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AciCofStash {
    pub role: CofRole,
    #[serde(rename = "final", default)]
    pub is_final: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub si_type: Option<String>,
}

impl AciCofStash {
    pub(super) fn role(role: CofRole) -> Self {
        Self {
            role,
            is_final: false,
            rg: None,
            citi: None,
            agreement_id: None,
            si_type: None,
        }
    }

    /// The stash an attempt's `connector_meta` carries, if ACI wrote one.
    ///
    /// An attempt authorized by a build without this module has none, and neither does one
    /// whose metadata cannot be read; both are treated as unknown rather than guessed at.
    pub(super) fn from_connector_meta(connector_meta: Option<&serde_json::Value>) -> Option<Self> {
        connector_meta
            .and_then(|meta| meta.get(STASH_KEY))
            .cloned()
            .and_then(|stash| serde_json::from_value(stash).ok())
    }

    pub(super) fn to_connector_meta(&self) -> serde_json::Value {
        let mut meta = serde_json::Map::new();
        meta.insert(
            STASH_KEY.to_string(),
            serde_json::to_value(self).unwrap_or_default(),
        );
        serde_json::Value::Object(meta)
    }
}

#[cfg(test)]
mod tests {
    use hyperswitch_masking::PeekInterface;
    use serde_json::json;

    use super::*;

    const RG: &str = "8ac7a4a19a1b2c3d019a1b2c3d4e0001";

    fn record() -> AciCofRecord {
        AciCofRecord::new(
            RG.to_string(),
            Some("MDGXVJV4N0910".to_string()),
            Some("TLID123".to_string()),
            Some("UNSCHEDULED".to_string()),
        )
    }

    #[test]
    fn record_round_trips_to_stamped_for_its_own_registration() {
        let record = record();
        let metadata = record.to_metadata();
        let value = metadata.peek();
        assert_eq!(value.get("aciCof"), Some(&json!(1)));
        assert_eq!(value.get("rg"), Some(&json!(RG)));
        assert_eq!(
            value.get("initialTransactionId"),
            Some(&json!("MDGXVJV4N0910"))
        );
        assert_eq!(value.get("agreementId"), Some(&json!("TLID123")));
        assert_eq!(value.get("siType"), Some(&json!("UNSCHEDULED")));

        assert_eq!(
            AciCofRecord::from_metadata(Some(value), Some(RG)),
            CofRecordView::Stamped(record)
        );
    }

    /// A Visa card has no agreement id and a MOTO CIT sends no type: absent values are not
    /// written, and read back as absent.
    #[test]
    fn absent_values_round_trip_as_absent() {
        let record = AciCofRecord::new(
            RG.to_string(),
            Some("476253540570401".to_string()),
            None,
            None,
        );
        let metadata = record.to_metadata();
        let value = metadata.peek();
        assert!(value.get("agreementId").is_none());
        assert!(value.get("siType").is_none());
        assert_eq!(
            AciCofRecord::from_metadata(Some(value), Some(RG)),
            CofRecordView::Stamped(record)
        );
    }

    #[test]
    fn record_for_another_registration_is_unbound() {
        let metadata = record().to_metadata();
        assert_eq!(
            AciCofRecord::from_metadata(Some(metadata.peek()), Some("8ac7a4a1other")),
            CofRecordView::Unbound {
                stamped_rg: Some(RG.to_string())
            }
        );
        assert_eq!(
            AciCofRecord::from_metadata(Some(metadata.peek()), None),
            CofRecordView::Unbound {
                stamped_rg: Some(RG.to_string())
            }
        );
    }

    #[test]
    fn stamped_record_without_a_registration_is_unbound() {
        for stamp in [
            json!({ "aciCof": 1, "agreementId": "TLID123" }),
            json!({ "aciCof": 1, "rg": "  ", "agreementId": "TLID123" }),
        ] {
            assert_eq!(
                AciCofRecord::from_metadata(Some(&stamp), Some(RG)),
                CofRecordView::Unbound { stamped_rg: None }
            );
        }
    }

    #[test]
    fn unstamped_metadata_is_legacy_and_exposes_its_hints() {
        let legacy = json!({ "agreementId": "TLID123", "siType": "RECURRING" });
        assert_eq!(
            AciCofRecord::from_metadata(Some(&legacy), Some(RG)),
            CofRecordView::Legacy {
                agreement_id: Some("TLID123".to_string()),
                si_type: Some("RECURRING".to_string()),
            }
        );
        // An explicit null marker is not a stamp.
        let null_marker = json!({ "aciCof": null, "agreementId": "TLID123" });
        assert_eq!(
            AciCofRecord::from_metadata(Some(&null_marker), Some(RG)),
            CofRecordView::Legacy {
                agreement_id: Some("TLID123".to_string()),
                si_type: None,
            }
        );
        assert_eq!(
            AciCofRecord::from_metadata(None, Some(RG)),
            CofRecordView::Legacy {
                agreement_id: None,
                si_type: None,
            }
        );
    }

    #[test]
    fn new_drops_a_rejected_citi_and_a_blank_agreement_id() {
        for rejected in [
            "8acda4a6a0e7fd8b01a0f102d66819f8",
            "PLSWDsdWh5GALKpIZs",
            "pm_UmaozPE1KJaWOCxAcIyA",
            "  ",
        ] {
            let record = AciCofRecord::new(
                RG.to_string(),
                Some(rejected.to_string()),
                Some("  ".to_string()),
                None,
            );
            assert_eq!(record.initial_transaction_id, None, "{rejected:?}");
            assert_eq!(record.agreement_id, None);
        }
    }

    /// Unknown keys are ignored and a value of the wrong type reads as absent; surrounding
    /// whitespace is trimmed.
    #[test]
    fn from_metadata_is_lenient() {
        let stamp = json!({
            "aciCof": 7,
            "rg": format!(" {RG} "),
            "initialTransactionId": 476_253_540_570_401_u64,
            "agreementId": { "nested": true },
            "siType": " RECURRING ",
            "somethingNew": "ignored",
        });
        assert_eq!(
            AciCofRecord::from_metadata(Some(&stamp), Some(RG)),
            CofRecordView::Stamped(AciCofRecord {
                rg: RG.to_string(),
                initial_transaction_id: None,
                agreement_id: None,
                si_type: Some("RECURRING".to_string()),
            })
        );

        let legacy = json!({ "agreementId": 42, "siType": ["RECURRING"], "other": 1 });
        assert_eq!(
            AciCofRecord::from_metadata(Some(&legacy), Some(RG)),
            CofRecordView::Legacy {
                agreement_id: None,
                si_type: None,
            }
        );
    }

    #[test]
    fn stash_round_trips_through_connector_meta() {
        let pending = AciCofStash {
            rg: Some(RG.to_string()),
            si_type: Some("UNSCHEDULED".to_string()),
            ..AciCofStash::role(CofRole::Cit)
        };
        let finished = AciCofStash {
            role: CofRole::Cit,
            is_final: true,
            rg: Some(RG.to_string()),
            citi: Some("MDGXVJV4N0910".to_string()),
            agreement_id: Some("TLID123".to_string()),
            si_type: Some("UNSCHEDULED".to_string()),
        };
        let mit = AciCofStash::role(CofRole::Mit);
        for stash in [pending, finished, mit] {
            let meta = stash.to_connector_meta();
            assert_eq!(AciCofStash::from_connector_meta(Some(&meta)), Some(stash));
        }

        assert_eq!(
            AciCofStash::role(CofRole::Mit).to_connector_meta(),
            json!({ "aciCof": { "role": "mit", "final": false } })
        );
    }

    #[test]
    fn stash_is_none_when_missing_or_unreadable() {
        for meta in [
            json!({}),
            json!({ "other": { "role": "cit" } }),
            json!({ "aciCof": "cit" }),
            json!({ "aciCof": { "role": "neither" } }),
            json!({ "aciCof": { "final": true } }),
            json!("garbage"),
        ] {
            assert_eq!(
                AciCofStash::from_connector_meta(Some(&meta)),
                None,
                "{meta}"
            );
        }
        assert_eq!(AciCofStash::from_connector_meta(None), None);
    }
}
