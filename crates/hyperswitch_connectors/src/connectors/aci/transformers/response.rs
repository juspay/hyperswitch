//! The payment response, shared by authorize, sync, void, mandate setup and standalone 3DS, and
//! what each of those flows records from it.

use std::collections::HashMap;

use common_enums::enums;
use common_utils::{request::Method, types::StringMajorUnit};
use hyperswitch_domain_models::{
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorResponseData, ErrorResponse, RouterData,
    },
    router_request_types::{AuthenticationData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm},
};
use hyperswitch_masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    acquirer::{resolve_citi, AcquirerReferences, AcquirerResolution},
    cof::{card_on_file_emission, CofEmission},
    flow::PaymentFlow,
    metadata::AciMetadata,
    request::{AciPaymentType, PaymentBrand},
    result::ResultCode,
    three_ds::{merchant_visible_authentication_data, AciThreeDSecureResponse},
    Error,
};
use crate::types::ResponseRouterData;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsResponse {
    pub(super) id: String,
    /// Registration token ID returned when `createRegistration=true` was sent.
    /// ACI returns this as `registrationId` at the top level.
    pub(super) registration_id: Option<Secret<String>>,
    pub(super) ndc: String,
    pub(super) timestamp: String,
    pub(super) build_number: String,
    pub(super) result: ResultCode,
    pub(super) redirect: Option<AciRedirectionData>,
    /// The payment type from the response (DB, PA, CP, RF, etc.)
    pub(super) payment_type: Option<AciPaymentType>,
    /// The card network / payment brand (VISA, MASTERCARD, etc.)
    pub(super) payment_brand: Option<PaymentBrand>,
    /// The processed amount
    pub(super) amount: Option<StringMajorUnit>,
    /// The processed currency (ISO 4217)
    pub(super) currency: Option<String>,
    /// Merchant-facing human-readable short transaction ID
    pub(super) short_id: Option<String>,
    /// Acquirer / connector response details
    pub(super) result_details: Option<AciResponseResultDetails>,
    /// Masked card details returned in the response
    pub(super) card: Option<AciResponseCardDetails>,
    /// 3DS authentication data from ACI
    #[serde(rename = "threeDSecure")]
    #[serde(skip_serializing)]
    pub(super) three_d_secure: Option<AciThreeDSecureResponse>,
    /// Standing instruction fields (Mastercard agreementId/TLID returned on initial CIT)
    #[serde(skip_serializing)]
    pub(super) standing_instruction: Option<AciResponseStandingInstruction>,
}

/// Masked card details returned in an ACI payment response.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciResponseCardDetails {
    pub bin: Option<String>,
    #[serde(rename = "last4Digits")]
    pub last4_digits: Option<String>,
    pub holder: Option<Secret<String>>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    #[serde(rename = "binCountry")]
    pub bin_country: Option<String>,
}

/// Acquirer / connector-level response details in an ACI payment response.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AciResponseResultDetails {
    pub extended_description: Option<String>,
    #[serde(rename = "clearingInstituteName")]
    pub clearing_institute_name: Option<String>,
    #[serde(rename = "ConnectorTxID1")]
    pub connector_tx_id1: Option<String>,
    #[serde(rename = "ConnectorTxID2")]
    pub connector_tx_id2: Option<String>,
    #[serde(rename = "ConnectorTxID3")]
    pub connector_tx_id3: Option<String>,
    #[serde(rename = "AcquirerResponse")]
    pub acquirer_response: Option<String>,
    #[serde(rename = "AuthCode")]
    pub auth_code: Option<String>,
    /// Alternate authorisation code field returned by some acquirers and
    /// in ACI sandbox test mode. When `AuthCode` is absent, `ApprovalCode`
    /// holds the same value.
    #[serde(rename = "ApprovalCode")]
    pub approval_code: Option<String>,
    #[serde(rename = "MerchantAdviceCode")]
    pub merchant_advice_code: Option<String>,
    /// Scheme trace ID (network-level transaction reference). On ABSA this
    /// is the canonical CITI for COF; on Nedbank the same value is also
    /// embedded inside `ConnectorTxID3[4]`.
    #[serde(rename = "SchemeTransactionId")]
    pub scheme_transaction_id: Option<String>,
    /// One of the two response locations OPPWA documents for the scheme-generated
    /// CIT reference (the other being the top-level `standingInstruction.initialTransactionId`);
    /// which one is populated is acquirer-specific. Nedbank and MPGS both document this
    /// field; ABSA and CyberSource document only `standingInstruction.initialTransactionId`.
    ///
    /// The explicit rename is required: the struct is `rename_all = "PascalCase"`, which
    /// would otherwise expect `CardholderInitiatedTransactionId` and silently miss the
    /// documented `...ID` spelling.
    #[serde(rename = "CardholderInitiatedTransactionID")]
    pub cardholder_initiated_transaction_id: Option<String>,
    /// Visa/Mastercard Payment Account Reference.
    #[serde(rename = "PaymentAccountReference")]
    pub payment_account_reference: Option<String>,
    /// The scheme/network transaction id as CyberSource (EMC) returns it — its integration
    /// sheet's response mapping and live response sample both carry the scheme reference here,
    /// and never in `SchemeTransactionId` (zero occurrences in that sheet).
    #[serde(rename = "NetworkTransactionId")]
    pub network_transaction_id: Option<String>,
    /// CyberSource mirror of [`Self::network_transaction_id`] — the sheet's sample returns the
    /// identical value in both. Used only as its fallback.
    #[serde(rename = "TransactionId")]
    pub transaction_id: Option<String>,
}

/// Standing instruction fields returned in an ACI payment response.
/// Mastercard returns `agreementId` (TLID) on initial CIT responses.
/// During the transitional period both `agreementId` and `initialTransactionId` are returned.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciResponseStandingInstruction {
    pub agreement_id: Option<String>,
    pub initial_transaction_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRedirectionData {
    pub method: Option<Method>,
    pub parameters: Vec<Parameters>,
    pub url: Url,
    pub preconditions: Option<Vec<PreconditionData>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreconditionData {
    pub method: Option<Method>,
    pub parameters: Vec<Parameters>,
    pub url: Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Parameters {
    pub name: String,
    pub value: String,
}

impl<F, Req: PaymentFlow>
    TryFrom<ResponseRouterData<F, AciPaymentsResponse, Req, PaymentsResponseData>>
    for RouterData<F, Req, PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: ResponseRouterData<F, AciPaymentsResponse, Req, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let ResponseRouterData {
            response,
            data,
            http_code,
        } = item;

        let redirection_data = response.redirect.clone().map(AciRedirectionData::into_form);
        let status = match redirection_data {
            Some(_) => enums::AttemptStatus::AuthenticationPending,
            None => data.request.attempt_status(response.result.class()?),
        };

        let declared = AciMetadata::declared_acquirer(data.connector_meta_data.as_ref());
        let acquirer = response.acquirer(declared);
        let references = acquirer.as_ref().map(|(_, references)| references);
        let CofEmission {
            mandate_reference,
            connector_metadata,
            network_txn_id,
            network_txn_link_id,
        } = card_on_file_emission(
            &data,
            status,
            response
                .registration_id
                .clone()
                .map(ExposeInterface::expose),
            response.scheme_reference(acquirer.as_ref()),
            response.agreement_id(),
            &response.id,
            acquirer.as_ref().map(|(resolution, _)| *resolution),
        );
        let connector_response =
            response.connector_response(references, data.request.external_authentication());

        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                attempt_status: Some(status),
                ..response
                    .result
                    .decline(&response.id, response.result_details.as_ref(), http_code)
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata,
                network_txn_id,
                // Mastercard Economically Related Transaction ID (TLID), surfaced on the attempt
                // for merchants. MITs read it from the card-on-file record, not from here.
                network_txn_link_id,
                // ACI's canonical payment id. The acquirer's RRN is a best-effort,
                // acquirer-specific reading that can resolve to the wrong field (an approval
                // code) for an unrecognised layout; it is still exposed via network_txn_id /
                // payment_checks for reconciliation.
                connector_response_reference_id: Some(response.id.clone()),
                incremental_authorization_allowed: None,
                authentication_data: response
                    .three_d_secure
                    .as_ref()
                    .map(|tds| Box::new(tds.ucs_authentication_data())),
                charges: None,
                payment_account_reference: None,
            })
        };

        Ok(Self {
            status,
            response,
            connector_response,
            ..data
        })
    }
}

impl AciPaymentsResponse {
    /// The downstream acquirer and the references it reported, resolved before the composites
    /// are read: the merchant-declared acquirer is the durable source, ACI's
    /// `clearingInstituteName` is trusted when present, and the response-shape heuristics are
    /// only a guess — which also decides whether composite-extracted values may outrank
    /// documented fields.
    fn acquirer(
        &self,
        declared: Option<super::acquirer::AcquirerKind>,
    ) -> Option<(AcquirerResolution, AcquirerReferences)> {
        self.result_details.as_ref().map(|details| {
            let resolution = AcquirerResolution::resolve(declared, details);
            (resolution, resolution.acquirer.references(details))
        })
    }

    /// The scheme CIT reference, validated per source and kept exactly as received:
    /// `network_txn_id` is surfaced to merchants and must match what the acquirer reported.
    fn scheme_reference(
        &self,
        acquirer: Option<&(AcquirerResolution, AcquirerReferences)>,
    ) -> Option<String> {
        resolve_citi(
            self.standing_instruction.as_ref(),
            self.result_details.as_ref(),
            acquirer.map(|(_, references)| references),
            acquirer.is_some_and(|(resolution, _)| resolution.positively_identified),
        )
    }

    /// The Mastercard agreementId (TLID), recorded with the registration alongside the CITI:
    /// both must be sent on MITs during the transitional period.
    fn agreement_id(&self) -> Option<String> {
        self.standing_instruction
            .as_ref()
            .and_then(|si| si.agreement_id.clone())
    }

    /// The card connector-response block, or `None` when there is nothing to report.
    ///
    /// Every slot is gated, `authentication_data` included: a 3DS-only response — no auth code,
    /// no payment checks — used to fall through this gate and be dropped whole, which is the
    /// other half of why `payment_method_data.card.authentication_data` read null.
    fn connector_response(
        &self,
        references: Option<&AcquirerReferences>,
        request_authentication: Option<&AuthenticationData>,
    ) -> Option<ConnectorResponseData> {
        let auth_code = references.and_then(|r| r.auth_code.clone());
        let card_network = self
            .payment_brand
            .as_ref()
            .map(|brand| format!("{brand:?}"));
        let payment_checks = self.payment_checks(references);
        // The copy of the 3DS evidence that surfaces as
        // `payment_method_data.card.authentication_data` in the payments API response.
        let authentication_data = merchant_visible_authentication_data(
            self.three_d_secure.as_ref(),
            request_authentication,
        );
        (auth_code.is_some()
            || card_network.is_some()
            || payment_checks.is_some()
            || authentication_data.is_some())
        .then(|| {
            ConnectorResponseData::with_additional_payment_method_data(
                AdditionalPaymentMethodConnectorResponse::Card {
                    auth_code,
                    card_network,
                    payment_checks,
                    authentication_data,
                    domestic_network: None,
                    processor_card_network: None,
                    card_subtype: None,
                    card_segment_type: None,
                    funding_source: None,
                    card_type: None,
                    issuer_name: None,
                    issuer_country: None,
                },
            )
        })
    }

    fn payment_checks(&self, references: Option<&AcquirerReferences>) -> Option<serde_json::Value> {
        let cvv = self.result.cvv_response.clone();
        let acquirer_response = self
            .result_details
            .as_ref()
            .and_then(|details| details.acquirer_response.clone());
        let stan = references.and_then(|r| r.stan.clone());
        let original_transaction_id = references.and_then(|r| r.original_transaction_id.clone());
        (cvv.is_some()
            || acquirer_response.is_some()
            || stan.is_some()
            || original_transaction_id.is_some())
        .then(|| {
            serde_json::json!({
                "cvvResponse": cvv,
                "acquirerResponse": acquirer_response,
                "stan": stan,
                "originalTransactionId": original_transaction_id,
            })
        })
    }
}

impl AciRedirectionData {
    /// The form the shopper is sent through: the redirect's parameters, overlaid with the first
    /// precondition's (a 3DS method call). The method defaults to POST; bank redirects name
    /// theirs.
    fn into_form(self) -> RedirectForm {
        let mut form_fields: HashMap<_, _> = self
            .parameters
            .into_iter()
            .map(|parameter| (parameter.name, parameter.value))
            .collect();
        if let Some(precondition) = self.preconditions.and_then(|p| p.into_iter().next()) {
            form_fields.extend(
                precondition
                    .parameters
                    .into_iter()
                    .map(|parameter| (parameter.name, parameter.value)),
            );
        }
        RedirectForm::Form {
            endpoint: self.url.to_string(),
            method: self.method.unwrap_or(Method::Post),
            form_fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(extra: serde_json::Value) -> AciPaymentsResponse {
        let mut body = serde_json::json!({
            "id": "8ac7a4a1",
            "ndc": "ndc",
            "timestamp": "t",
            "buildNumber": "b",
            "result": {"code": "000.100.110", "description": "approved"},
        });
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
        serde_json::from_value(body).expect("payment response deserializes")
    }

    /// A 3DS-only response — no auth code, no payment checks — used to fall through the
    /// `connector_response` gate and be dropped whole, which is the other half of why
    /// `payment_method_data.card.authentication_data` read null.
    #[test]
    fn a_three_ds_only_response_still_produces_a_connector_response() {
        let three_ds_only = response(serde_json::json!({"threeDSecure": {"eci": "05"}}));

        let card = match three_ds_only
            .connector_response(None, None)
            .and_then(|data| data.additional_payment_method_data)
        {
            Some(AdditionalPaymentMethodConnectorResponse::Card {
                authentication_data,
                auth_code,
                payment_checks,
                ..
            }) => (authentication_data, auth_code, payment_checks),
            other => panic!("expected a card connector response, got {other:?}"),
        };

        assert!(card
            .0
            .is_some_and(|evidence| evidence.get("eci") == Some(&serde_json::json!("05"))));
        assert_eq!((card.1, card.2), (None, None));
        // Nothing to report at all is still nothing.
        assert!(response(serde_json::json!({}))
            .connector_response(None, None)
            .is_none());
    }

    /// The precondition's parameters overlay the redirect's, and the method defaults to POST.
    #[test]
    fn a_redirect_becomes_a_form_with_the_precondition_parameters() {
        let redirect: AciRedirectionData = serde_json::from_value(serde_json::json!({
            "url": "https://acs.example/challenge",
            "parameters": [{"name": "PaReq", "value": "a"}, {"name": "TermUrl", "value": "t"}],
            "preconditions": [
                {
                    "url": "https://acs.example/method",
                    "parameters": [{"name": "PaReq", "value": "b"}],
                },
                {
                    "url": "https://acs.example/ignored",
                    "parameters": [{"name": "Other", "value": "c"}],
                },
            ],
        }))
        .expect("redirect deserializes");

        let RedirectForm::Form {
            endpoint,
            method,
            form_fields,
        } = redirect.into_form()
        else {
            panic!("expected a form redirect");
        };
        assert_eq!(endpoint, "https://acs.example/challenge");
        assert_eq!(method, Method::Post);
        assert_eq!(
            form_fields,
            HashMap::from([
                ("PaReq".to_string(), "b".to_string()),
                ("TermUrl".to_string(), "t".to_string()),
            ])
        );
    }
}
