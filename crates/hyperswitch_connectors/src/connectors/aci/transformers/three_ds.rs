//! 3DS vocabulary shared by both directions: the authentication status codes, ACI's
//! `threeDSecure` response block, and the authentication evidence merchants see.

use common_enums::enums;
use hyperswitch_domain_models::router_request_types::{AuthenticationData, UcsAuthenticationData};
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

/// Narrow a 3DS transaction status to the `[YIAUN]` set OPPWA accepts on an authorization.
///
/// `R` (issuer rejected) and `C`/`D` (challenge still required) are not legal here, so the field is
/// omitted rather than sent out of spec — the challenge states are non-terminal and should never
/// reach authorize at all. Deliberately exhaustive with no wildcard, so that a new
/// `TransactionStatus` variant fails the build rather than being silently dropped.
pub(super) fn map_authentication_status(status: &enums::TransactionStatus) -> Option<char> {
    match status {
        enums::TransactionStatus::Success => Some('Y'),
        enums::TransactionStatus::Failure => Some('N'),
        enums::TransactionStatus::VerificationNotPerformed => Some('U'),
        enums::TransactionStatus::NotVerified => Some('A'),
        enums::TransactionStatus::InformationOnly => Some('I'),
        enums::TransactionStatus::Rejected
        | enums::TransactionStatus::ChallengeRequired
        | enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication
        | enums::TransactionStatus::SecurePaymentConfirmationRequired => None,
    }
}

/// Parse ACI's inbound `threeDSecure.authenticationStatus`.
///
/// `TransactionStatus` derives `EnumString` without a rename, so its `FromStr` expects `"Success"`
/// rather than `"Y"` — the mapping has to be explicit. Accepts the full eight-value 3DS set, unlike
/// the outbound narrowing above, and keeps a wildcard because this is an untrusted wire string.
fn parse_aci_authentication_status(raw: &str) -> Option<enums::TransactionStatus> {
    match raw.trim() {
        "Y" => Some(enums::TransactionStatus::Success),
        "N" => Some(enums::TransactionStatus::Failure),
        "U" => Some(enums::TransactionStatus::VerificationNotPerformed),
        "A" => Some(enums::TransactionStatus::NotVerified),
        "R" => Some(enums::TransactionStatus::Rejected),
        "C" => Some(enums::TransactionStatus::ChallengeRequired),
        "D" => Some(enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication),
        "S" => Some(enums::TransactionStatus::SecurePaymentConfirmationRequired),
        "I" => Some(enums::TransactionStatus::InformationOnly),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciThreeDSecureResponse {
    pub eci: Option<String>,
    pub verification_id: Option<Secret<String>>,
    pub version: Option<String>,
    pub flow: Option<String>,
    pub ds_transaction_id: Option<String>,
    pub acs_transaction_id: Option<String>,
    pub challenge_mandated_indicator: Option<String>,
    pub authentication_type: Option<String>,
    /// Cardholder-facing information text (PII); redacted in Debug/serialized output.
    pub card_holder_info: Option<Secret<String>>,
    pub authentication_status: Option<String>,
    pub xid: Option<String>,
    /// 3DS authentication cryptogram (Sensitive Authentication Data). Must never
    /// be logged or persisted in cleartext, so it is held as `Secret`.
    pub cavv: Option<Secret<String>>,
    pub error_code: Option<String>,
    pub error_description: Option<String>,
    pub error_source: Option<String>,
}

impl AciThreeDSecureResponse {
    /// The authentication value (CAVV), which ACI returns as `verificationId` or `cavv`.
    pub(super) fn authentication_value(&self) -> Option<Secret<String>> {
        self.verification_id.clone().or_else(|| self.cavv.clone())
    }

    /// The block as the unified connector service takes it.
    pub(super) fn ucs_authentication_data(&self) -> UcsAuthenticationData {
        UcsAuthenticationData {
            eci: self.eci.clone(),
            cavv: self.authentication_value(),
            threeds_server_transaction_id: self.ds_transaction_id.clone(),
            message_version: self.version.as_deref().and_then(|v| v.parse().ok()),
            ds_trans_id: self.ds_transaction_id.clone(),
            acs_trans_id: self.acs_transaction_id.clone(),
            // ACI returns this as a single `[YNUARCDI]` character.
            trans_status: self
                .authentication_status
                .as_deref()
                .and_then(parse_aci_authentication_status),
            transaction_id: self.xid.clone(),
            ucaf_collection_indicator: None,
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
        }
    }
}

/// The merchant-visible 3DS authentication evidence, for
/// `AdditionalPaymentMethodConnectorResponse::Card.authentication_data` — the surface that maps
/// into the payments API response as `payment_method_data.card.authentication_data`. (The
/// `UcsAuthenticationData` built elsewhere from the same response feeds the unified connector
/// service only and never reaches the API response, which is how this field stayed null while
/// the 3DS data was demonstrably parsed.)
///
/// Deliberately excludes the CAVV/`verificationId` cryptogram (Sensitive Authentication Data —
/// it stays on the `Secret`-typed UCS surface) and `cardHolderInfo` (PII). Evidence only:
/// status, ids, protocol version.
///
/// `None` when the block carries no evidence at all. ACI can echo a `threeDSecure` object whose
/// every field is null (a sync of an already-settled payment, say); mapping that to an all-null
/// blob would report "no authentication" as confidently as a real answer, and — because the
/// merge in `add_connector_response_to_additional_payment_data` only preserves what is stored
/// when the incoming value is `None` — would overwrite the evidence an earlier response
/// established. Nothing to say is said by saying nothing.
fn additional_authentication_data(tds: &AciThreeDSecureResponse) -> Option<serde_json::Value> {
    let blob = serde_json::json!({
        "eci": tds.eci,
        "authenticationStatus": tds.authentication_status,
        "dsTransactionId": tds.ds_transaction_id,
        "acsTransactionId": tds.acs_transaction_id,
        "xid": tds.xid,
        "version": tds.version.as_deref().map(normalize_three_ds_version),
        "flow": tds.flow,
        "challengeMandatedIndicator": tds.challenge_mandated_indicator,
    });

    carries_evidence(blob)
}

/// OPPWA documents `threeDSecure.version` as `major.minor` (e.g. `2.2`), which is the shape we
/// send and the shape the request-side builder produces — but a response is free to echo the
/// full protocol version (`2.2.0`). Trim to `major.minor` so a merchant parsing this field reads
/// one shape whichever route authenticated the payment. Anything that does not look like a
/// dotted version is passed through untouched rather than mangled.
fn normalize_three_ds_version(version: &str) -> String {
    let mut parts = version.split('.');
    match (parts.next(), parts.next()) {
        (Some(major), Some(minor)) => format!("{major}.{minor}"),
        _ => version.to_string(),
    }
}

/// `Some(blob)` when at least one key holds a value, `None` when every one of them is null.
fn carries_evidence(blob: serde_json::Value) -> Option<serde_json::Value> {
    blob.as_object()
        .is_some_and(|fields| fields.values().any(|value| !value.is_null()))
        .then_some(blob)
}

/// The merchant-visible 3DS evidence for one response: what ACI echoed back, falling back to
/// what the request carried in.
///
/// The fallback exists because with authentication performed outside the payment call — an
/// external authentication connector, or ACI's own standalone `/v1/threeDSecure` — ACI is handed
/// the evidence rather than producing it, so there is no `threeDSecure` block in the response to
/// read. An echoed block that carries no evidence falls through to the request rather than
/// winning with nothing to say.
pub(super) fn merchant_visible_authentication_data(
    response: Option<&AciThreeDSecureResponse>,
    request: Option<&AuthenticationData>,
) -> Option<serde_json::Value> {
    response
        .and_then(additional_authentication_data)
        .or_else(|| request.and_then(additional_authentication_data_from_request))
}

/// The same merchant-visible shape as [`additional_authentication_data`], rebuilt from the 3DS
/// result the router handed us on the request.
///
/// When the authentication was performed outside the payment call, ACI is given the evidence
/// instead of producing it and its response carries no `threeDSecure` block to echo back — so
/// without this the field would be null for a payment that was demonstrably authenticated. The
/// keys are deliberately identical to the response-derived blob, so a merchant reads one shape
/// whoever did the authenticating.
///
/// Excludes the CAVV cryptogram for the same reason as the response path. `xid` and
/// `challengeMandatedIndicator` are ACI-side artefacts with no equivalent here and stay null
/// rather than being approximated (`threeds_server_transaction_id` is a 3DS2 server id, not an
/// XID).
///
/// `None` when the request-side result carries no evidence either, for the same reason the
/// response-side builder returns `None` on an empty block.
fn additional_authentication_data_from_request(
    auth: &AuthenticationData,
) -> Option<serde_json::Value> {
    let blob = serde_json::json!({
        "eci": auth.eci,
        "authenticationStatus": auth
            .transaction_status
            .as_ref()
            .and_then(map_authentication_status),
        "dsTransactionId": auth.ds_trans_id,
        "acsTransactionId": auth.acs_trans_id,
        "xid": null,
        // `major.minor`, matching both what we send ACI and what ACI reports back.
        "version": auth
            .message_version
            .as_ref()
            .map(|v| format!("{}.{}", v.get_major(), v.get_minor())),
        "flow": auth.authentication_type.map(|ty| match ty {
            enums::DecoupledAuthenticationType::Challenge => "challenge",
            enums::DecoupledAuthenticationType::Frictionless => "frictionless",
        }),
        "challengeMandatedIndicator": null,
    });

    carries_evidence(blob)
}

#[cfg(test)]
mod tests {

    use common_utils::types::SemanticVersion;

    use super::*;

    /// The merchant-visible authentication blob must carry the 3DS evidence — and must never
    /// carry the CAVV/`verificationId` cryptogram (Sensitive Authentication Data) or the
    /// `cardHolderInfo` PII, in any key, under any name.
    #[test]
    fn merchant_visible_authentication_data_excludes_sensitive_authentication_data() {
        let cavv = "AAABBBCCCDDDeeeFFF000cavv=";
        let verification_id = "verif-SENSITIVE-123";
        let holder_info = "Please contact your bank";
        let tds = AciThreeDSecureResponse {
            eci: Some("05".to_string()),
            verification_id: Some(Secret::new(verification_id.to_string())),
            version: Some("2.2.0".to_string()),
            flow: Some("challenge".to_string()),
            ds_transaction_id: Some("ds-123".to_string()),
            acs_transaction_id: Some("acs-456".to_string()),
            challenge_mandated_indicator: Some("Y".to_string()),
            authentication_type: None,
            card_holder_info: Some(Secret::new(holder_info.to_string())),
            authentication_status: Some("Y".to_string()),
            xid: Some("xid-789".to_string()),
            cavv: Some(Secret::new(cavv.to_string())),
            error_code: None,
            error_description: None,
            error_source: None,
        };

        let blob = additional_authentication_data(&tds).expect("evidence present");

        let expect = |key: &str| blob.get(key).and_then(|v| v.as_str());
        assert_eq!(expect("eci"), Some("05"));
        assert_eq!(expect("authenticationStatus"), Some("Y"));
        assert_eq!(expect("dsTransactionId"), Some("ds-123"));
        assert_eq!(expect("acsTransactionId"), Some("acs-456"));
        // Trimmed from the `2.2.0` ACI sent, so both authentication routes report the
        // `major.minor` shape OPPWA documents.
        assert_eq!(expect("version"), Some("2.2"));

        let rendered = blob.to_string();
        assert!(
            !rendered.contains(cavv),
            "CAVV cryptogram leaked into the merchant-visible blob: {rendered}"
        );
        assert!(
            !rendered.contains(verification_id),
            "verificationId leaked into the merchant-visible blob: {rendered}"
        );
        assert!(
            !rendered.contains(holder_info),
            "cardHolderInfo PII leaked into the merchant-visible blob: {rendered}"
        );
    }

    fn three_d_secure(fields: serde_json::Value) -> AciThreeDSecureResponse {
        serde_json::from_value(fields).expect("every field is optional")
    }

    fn request_authentication_data() -> AuthenticationData {
        AuthenticationData {
            eci: Some("02".to_string()),
            cavv: Secret::new("request-side-cavv".to_string()),
            threeds_server_transaction_id: None,
            message_version: Some(SemanticVersion::new(2, 1, 0)),
            ds_trans_id: Some("request-ds".to_string()),
            created_at: common_utils::date_time::now(),
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            acs_trans_id: None,
            authentication_type: None,
            transaction_status: Some(enums::TransactionStatus::Success),
            cb_network_params: None,
            exemption_indicator: None,
        }
    }

    /// ACI can echo a `threeDSecure` object whose every field is null — a sync of a payment
    /// whose authentication it has since forgotten, say. That is not evidence of "no
    /// authentication", and mapping it to an all-null blob would report it as if it were: the
    /// merge in `add_connector_response_to_additional_payment_data` only keeps stored evidence
    /// when the incoming value is `None`, so an all-null blob would overwrite the real answer an
    /// earlier response established.
    #[test]
    fn an_empty_three_d_secure_block_is_not_evidence() {
        assert_eq!(
            additional_authentication_data(&three_d_secure(serde_json::json!({}))),
            None
        );
        assert_eq!(
            additional_authentication_data(&three_d_secure(serde_json::json!({
                "eci": null,
                "authenticationStatus": null,
                "version": null,
            }))),
            None
        );
        // One real field is enough to be worth reporting.
        assert!(
            additional_authentication_data(&three_d_secure(serde_json::json!({
                "eci": "05",
            })))
            .is_some()
        );
    }

    /// The CAVV lives outside the evidence keys, so a `threeDSecure` block carrying nothing but
    /// the cryptogram has nothing a merchant may see — and must not produce a blob whose only
    /// effect is to erase what is already stored.
    #[test]
    fn a_cavv_only_block_is_not_evidence() {
        assert_eq!(
            additional_authentication_data(&three_d_secure(serde_json::json!({
                "cavv": "AAABBBCCCDDDeeeFFF000cavv=",
                "cardHolderInfo": "Please contact your bank",
            }))),
            None
        );
    }

    /// OPPWA documents the version as `major.minor`, which is the shape we send and the shape
    /// the request-side builder produces — so a response echoing the full protocol version is
    /// trimmed to match rather than handing merchants two shapes for one field.
    #[test]
    fn three_ds_version_is_reported_as_major_minor_whatever_aci_sends() {
        let version_of = |sent: &str| {
            additional_authentication_data(&three_d_secure(serde_json::json!({
                "version": sent,
            })))
            .and_then(|blob| blob.get("version")?.as_str().map(str::to_string))
        };

        assert_eq!(version_of("2.2.0").as_deref(), Some("2.2"));
        assert_eq!(version_of("2.2").as_deref(), Some("2.2"));
        // Not a dotted version: passed through rather than mangled.
        assert_eq!(version_of("2").as_deref(), Some("2"));
    }

    /// The response wins when it has something to say; the request-side result is the fallback,
    /// not a competitor. An empty echoed block says nothing, so it falls through to the request
    /// rather than winning with nothing.
    #[test]
    fn response_evidence_wins_and_an_empty_block_falls_back_to_the_request() {
        let auth = request_authentication_data();
        let eci_of = |value: Option<serde_json::Value>| {
            value.and_then(|blob| blob.get("eci")?.as_str().map(str::to_string))
        };

        let from_response = three_d_secure(serde_json::json!({ "eci": "05" }));
        assert_eq!(
            eci_of(merchant_visible_authentication_data(
                Some(&from_response),
                Some(&auth)
            ))
            .as_deref(),
            Some("05"),
        );

        let empty = three_d_secure(serde_json::json!({}));
        assert_eq!(
            eci_of(merchant_visible_authentication_data(
                Some(&empty),
                Some(&auth)
            ))
            .as_deref(),
            Some("02"),
            "an empty echoed block must not shadow the evidence the request carried in",
        );

        assert_eq!(
            eci_of(merchant_visible_authentication_data(None, Some(&auth))).as_deref(),
            Some("02"),
        );
        assert_eq!(merchant_visible_authentication_data(None, None), None);
    }

    /// Authentication performed outside the payment call (an external authentication connector,
    /// or ACI's own standalone `/v1/threeDSecure`) leaves ACI nothing to echo back, so the
    /// merchant-visible blob is rebuilt from what the request carried in — same keys, and the
    /// CAVV still excluded.
    #[test]
    fn externally_authenticated_payments_surface_the_request_side_evidence() {
        let cavv = "AAABBBCCCDDDeeeFFF000cavv=";
        let auth = AuthenticationData {
            eci: Some("05".to_string()),
            cavv: Secret::new(cavv.to_string()),
            threeds_server_transaction_id: Some("3dsserver-abc".to_string()),
            message_version: Some(SemanticVersion::new(2, 2, 0)),
            ds_trans_id: Some("ds-123".to_string()),
            created_at: common_utils::date_time::now(),
            challenge_code: None,
            challenge_cancel: None,
            challenge_code_reason: None,
            message_extension: None,
            acs_trans_id: Some("acs-456".to_string()),
            authentication_type: Some(enums::DecoupledAuthenticationType::Challenge),
            transaction_status: Some(enums::TransactionStatus::Success),
            cb_network_params: None,
            exemption_indicator: None,
        };

        let blob = additional_authentication_data_from_request(&auth).expect("evidence present");

        let expect = |key: &str| blob.get(key).and_then(|v| v.as_str());
        assert_eq!(expect("eci"), Some("05"));
        assert_eq!(expect("authenticationStatus"), Some("Y"));
        assert_eq!(expect("dsTransactionId"), Some("ds-123"));
        assert_eq!(expect("acsTransactionId"), Some("acs-456"));
        // OPPWA documents the version as `major.minor`, not a full semver.
        assert_eq!(expect("version"), Some("2.2"));
        assert_eq!(expect("flow"), Some("challenge"));
        // Keys with no equivalent on this path stay present, so the shape a merchant parses
        // does not change with the authentication route.
        assert!(blob.get("xid").is_some_and(serde_json::Value::is_null));
        assert!(blob
            .get("challengeMandatedIndicator")
            .is_some_and(serde_json::Value::is_null));

        assert!(
            !blob.to_string().contains(cavv),
            "CAVV cryptogram leaked into the merchant-visible blob: {blob}"
        );
    }

    /// The 3DS cryptogram (`cavv`) and `cardHolderInfo` are Sensitive
    /// Authentication Data / PII and must never appear in cleartext in Debug
    /// output (which is what `logger::info!(connector_response=?response)`
    /// emits). Typing them as `Secret` redacts them.
    #[test]
    fn three_ds_secret_fields_are_redacted_in_debug() {
        let cavv = "AAABBBCCCDDDeeeFFF000=";
        let holder = "Jane Cardholder";
        let three_ds = AciThreeDSecureResponse {
            eci: Some("05".to_string()),
            verification_id: Some(Secret::new("verif-123".to_string())),
            version: None,
            flow: None,
            ds_transaction_id: None,
            acs_transaction_id: None,
            challenge_mandated_indicator: None,
            authentication_type: None,
            card_holder_info: Some(Secret::new(holder.to_string())),
            authentication_status: None,
            xid: None,
            cavv: Some(Secret::new(cavv.to_string())),
            error_code: None,
            error_description: None,
            error_source: None,
        };

        let debug_output = format!("{three_ds:?}");
        assert!(
            !debug_output.contains(cavv),
            "cavv cryptogram leaked in Debug output: {debug_output}"
        );
        assert!(
            !debug_output.contains(holder),
            "cardHolderInfo leaked in Debug output: {debug_output}"
        );
    }

    /// OPPWA accepts only `[YIAUN]` for `threeDSecure.authenticationStatus`. The challenge and
    /// rejected states have no legal representation, so the field must be omitted rather than sent
    /// out of spec.
    #[test]
    fn authentication_status_narrows_to_the_documented_values() {
        assert_eq!(
            map_authentication_status(&enums::TransactionStatus::Success),
            Some('Y')
        );
        assert_eq!(
            map_authentication_status(&enums::TransactionStatus::Failure),
            Some('N')
        );
        assert_eq!(
            map_authentication_status(&enums::TransactionStatus::VerificationNotPerformed),
            Some('U')
        );
        assert_eq!(
            map_authentication_status(&enums::TransactionStatus::NotVerified),
            Some('A')
        );
        assert_eq!(
            map_authentication_status(&enums::TransactionStatus::InformationOnly),
            Some('I')
        );
        for illegal in [
            enums::TransactionStatus::Rejected,
            enums::TransactionStatus::ChallengeRequired,
            enums::TransactionStatus::ChallengeRequiredDecoupledAuthentication,
        ] {
            assert_eq!(map_authentication_status(&illegal), None);
        }
    }

    /// The inbound direction accepts the full 3DS set, and must not be confused with the outbound
    /// narrowing — `TransactionStatus`'s own `FromStr` expects "Success", not "Y".
    #[test]
    fn authentication_status_round_trips_from_the_wire() {
        assert_eq!(
            parse_aci_authentication_status("Y"),
            Some(enums::TransactionStatus::Success)
        );
        assert_eq!(
            parse_aci_authentication_status(" R "),
            Some(enums::TransactionStatus::Rejected)
        );
        assert_eq!(parse_aci_authentication_status("Success"), None);
        assert_eq!(parse_aci_authentication_status(""), None);
    }
}
