//! ACI's `result` block: what a result code means, and how a failed one becomes an
//! [`ErrorResponse`].

use error_stack::report;
use hyperswitch_domain_models::router_data::ErrorResponse;
use hyperswitch_interfaces::errors;
use serde::{Deserialize, Serialize};

use super::{response::AciResponseResultDetails, Error};
use crate::connectors::aci::aci_result_codes::{FAILURE_CODES, PENDING_CODES, SUCCESSFUL_CODES};

/// The outcome a result code reports, per ACI's response-code catalogue. The three code lists
/// are disjoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResultClass {
    /// Success and Success-Flagged: the transaction went through. A flag is an out-of-band risk
    /// review signal, not a pending state.
    Success,
    /// Pending and Pending-Delayed: the final status is not known yet.
    Pending,
    /// Every Reject category and Chargeback.
    Failure,
}

impl ResultClass {
    /// The class of `code`, or `None` for a code the catalogue does not list.
    pub(super) fn of(code: &str) -> Option<Self> {
        if SUCCESSFUL_CODES.contains(&code) {
            Some(Self::Success)
        } else if PENDING_CODES.contains(&code) {
            Some(Self::Pending)
        } else if FAILURE_CODES.contains(&code) {
            Some(Self::Failure)
        } else {
            None
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultCode {
    pub(crate) code: String,
    pub(crate) description: String,
    pub(crate) parameter_errors: Option<Vec<ErrorParameters>>,
    #[serde(rename = "cvvResponse")]
    pub cvv_response: Option<String>,
}

impl ResultCode {
    /// The class of this result, failing on a code outside the catalogue: a status cannot be
    /// derived from a code nobody has classified.
    pub(super) fn class(&self) -> Result<ResultClass, Error> {
        ResultClass::of(&self.code).ok_or_else(|| {
            report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(self.code.clone())
            ))
        })
    }

    /// Whether this result is in the failure catalogue. An unlisted code is not a failure.
    pub(super) fn is_failure(&self) -> bool {
        ResultClass::of(&self.code) == Some(ResultClass::Failure)
    }

    /// The [`ErrorResponse`] for transaction `id`, which ACI processed and failed.
    ///
    /// Carries no attempt status; flows that have one set it with struct-update syntax.
    pub(super) fn decline(
        self,
        id: &str,
        details: Option<&AciResponseResultDetails>,
        status_code: u16,
    ) -> ErrorResponse {
        let network_codes = AciNetworkCodes::from_details(details);
        ErrorResponse {
            code: self.code,
            message: self.description.clone(),
            reason: Some(self.description),
            status_code,
            attempt_status: None,
            connector_transaction_id: Some(id.to_string()),
            connector_response_reference_id: Some(id.to_string()),
            network_decline_code: None,
            network_advice_code: network_codes.advice_code,
            network_error_message: network_codes.error_message,
            connector_metadata: None,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ErrorParameters {
    pub(crate) name: String,
    pub(crate) value: Option<String>,
    pub(crate) message: String,
}

/// The body of a non-2xx ACI response.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciErrorResponse {
    ndc: String,
    timestamp: String,
    build_number: String,
    result: ResultCode,
    /// ACI's transaction id. Present on error payloads that describe a transaction that was
    /// actually created (as opposed to request-validation failures), so it is worth
    /// surfacing as the connector transaction id rather than discarding it.
    id: Option<String>,
    /// Carries the acquirer Merchant Advice Code and original response code — see
    /// [`AciNetworkCodes`].
    result_details: Option<AciResponseResultDetails>,
}

impl AciErrorResponse {
    /// The error every flow reports for a non-2xx response.
    pub(crate) fn into_error_response(self, status_code: u16) -> ErrorResponse {
        let network_codes = AciNetworkCodes::from_details(self.result_details.as_ref());
        ErrorResponse {
            status_code,
            code: self.result.code,
            message: self.result.description,
            reason: self
                .result
                .parameter_errors
                .map(|errors| {
                    errors
                        .into_iter()
                        .map(|error| {
                            format!(
                                "Field is {} and the message is {}",
                                error.name, error.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                // `parameter_errors` is only populated for request-validation failures. For an
                // acquirer decline it is absent, which left `reason` null and the merchant with no
                // decline text at all — fall back to the acquirer's human-readable description.
                .or_else(|| network_codes.error_message.clone()),
            attempt_status: None,
            // Deliberately NOT populated from `id`. This is the single error handler shared by
            // every flow, and the router writes this field straight onto
            // `payment_attempt.connector_transaction_id`. On a Capture/Void/Refund error the body
            // carries that sub-transaction's own id; overwriting the authorization id with it
            // makes the attempt unaddressable, because PSync/Void/Refund all build
            // `v1/payments/{id}` from that column.
            connector_transaction_id: None,
            connector_response_reference_id: self.id,
            network_advice_code: network_codes.advice_code,
            // See [`AciNetworkCodes`] for why this stays unset.
            network_decline_code: None,
            network_error_message: network_codes.error_message,
            connector_metadata: None,
        }
    }
}

/// Issuer/acquirer decisioning codes lifted out of `resultDetails`, in the shape
/// hyperswitch's retry and revenue-recovery layers consume them.
///
/// All four ACI acquirer integration sheets (Nedbank, ABSA, MPGS, CyberSource) mark the
/// Visa/Mastercard Acquirer Merchant Advice Code mandate as supported and document it as
/// arriving in `resultDetails.MerchantAdviceCode`. It drives the `[merchant_advice_codes.*]`
/// config lookups — notably `03` (do not try again) and `21` (recurring payment cancelled:
/// deregister the stored credential) — which are load-bearing for stored-credential retry
/// hygiene. Leaving these `None` made every ACI decline indistinguishable from a soft decline.
///
/// Note what is deliberately absent: `network_decline_code`. That field is consumed by
/// `get_gsm_record` as an ISO 8583 issuer code in a priority-1 lookup keyed
/// `<card_network>_<issuer_code>`, ahead of the connector-error-code lookup. The obvious
/// candidate, `resultDetails.AcquirerResponse`, is described by all four acquirer sheets as
/// bank-specific rather than ISO, so promoting it would match GSM rows authored for a different
/// meaning of the same digits and mis-classify declines that resolve correctly today. It needs
/// per-acquirer normalisation first, so `network_decline_code` stays `None` for now.
#[derive(Debug, Default, Clone)]
struct AciNetworkCodes {
    advice_code: Option<String>,
    error_message: Option<String>,
}

impl AciNetworkCodes {
    fn from_details(details: Option<&AciResponseResultDetails>) -> Self {
        details
            .map(|d| Self {
                advice_code: d.merchant_advice_code.clone(),
                error_message: d.extended_description.clone(),
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_codes_classify_into_the_catalogue() {
        assert_eq!(ResultClass::of("000.100.110"), Some(ResultClass::Success));
        assert_eq!(ResultClass::of("000.200.000"), Some(ResultClass::Pending));
        assert_eq!(ResultClass::of("800.100.151"), Some(ResultClass::Failure));
        assert_eq!(ResultClass::of("123.456.789"), None);
    }

    #[test]
    fn an_unlisted_code_has_no_class_and_is_not_a_failure() {
        let unlisted = ResultCode {
            code: "123.456.789".to_string(),
            ..Default::default()
        };
        assert!(unlisted.class().is_err());
        assert!(!unlisted.is_failure());
    }

    /// All four acquirer sheets document the Visa/Mastercard Acquirer Merchant Advice Code
    /// as arriving in `resultDetails.MerchantAdviceCode`. It must reach `ErrorResponse`'s
    /// `network_advice_code` for the `[merchant_advice_codes.*]` config lookups to fire —
    /// `21` in particular means the cardholder cancelled the recurring agreement and the
    /// stored credential should be deregistered rather than retried.
    #[test]
    fn network_codes_are_lifted_from_result_details() {
        let details = AciResponseResultDetails {
            merchant_advice_code: Some("21".to_string()),
            acquirer_response: Some("51".to_string()),
            extended_description: Some("Insufficient funds".to_string()),
            ..Default::default()
        };

        let error = ResultCode {
            code: "800.100.151".to_string(),
            description: "declined".to_string(),
            ..Default::default()
        }
        .decline("8ac7a4a1", Some(&details), 200);
        assert_eq!(error.network_advice_code.as_deref(), Some("21"));
        assert_eq!(
            error.network_error_message.as_deref(),
            Some("Insufficient funds")
        );
        assert_eq!(error.network_decline_code, None);
        assert_eq!(error.reason.as_deref(), Some("declined"));
        assert_eq!(error.connector_transaction_id.as_deref(), Some("8ac7a4a1"));
    }

    /// A response with no `resultDetails` must not panic or fabricate codes.
    #[test]
    fn network_codes_default_to_none_without_result_details() {
        let codes = AciNetworkCodes::from_details(None);
        assert_eq!(codes.advice_code, None);
        assert_eq!(codes.error_message, None);
    }
}
