use common_utils::types::FloatMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::{CardRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::PaymentsResponseData,
    types::PaymentsAuthorizeRouterData,
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::ResponseRouterData;

/// Directa24's code for WebPay, Transbank's Chilean redirect method.
/// D24 payment-method codes are an open set, not a documented enum.
pub const WEBPAY_PAYMENT_METHOD: &str = "WP";

pub struct D24RouterData<T> {
    /// D24 takes `amount` as a JSON number in major units. CLP is a zero-decimal
    /// currency in `common_enums`, so 10000 CLP converts to `10000.0`, and
    /// 1050 USD minor units converts to `10.5`.
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for D24RouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct D24PaymentsRequest {
    amount: FloatMajorUnit,
    payment_method: &'static str,
}

impl TryFrom<&D24RouterData<&PaymentsAuthorizeRouterData>> for D24PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &D24RouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        // D24 is a UCS-only connector: the complete WebPay deposit request (payer,
        // country, document, return URLs, HMAC headers) is assembled by the Unified
        // Connector Service. This HS-side path exists only to satisfy the connector
        // trait surface and is never exercised in production.
        match &item.router_data.request.payment_method_data {
            PaymentMethodData::CardRedirect(CardRedirectData::CardRedirect {}) => Ok(Self {
                amount: item.amount,
                payment_method: WEBPAY_PAYMENT_METHOD,
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported by D24 WebPay".to_string(),
            )
            .into()),
        }
    }
}

// Auth Struct
pub struct D24AuthType {
    pub(super) api_key: Secret<String>,
    /// The read-only API Key. D24 issues two credential sets; the read-only key is the
    /// `X-Login` used on `GET /v3/deposits/{id}` (PSync).
    #[allow(dead_code)]
    pub(super) read_only_api_key: Secret<String>,
    /// The API Signature — the HMAC-SHA256 key for the `Authorization` header.
    #[allow(dead_code)]
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for D24AuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            // UCS-only connector: HS forwards the raw ConnectorAuthType to the
            // Unified Connector Service, which expects
            // SignatureKey { api_key = API Key, key1 = Read-only API Key,
            // api_secret = API Signature }.
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                read_only_api_key: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

/// The eight deposit statuses D24 documents for WebPay.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum D24PaymentStatus {
    /// Paid and settled. Final.
    Completed,
    /// Created; the customer has not opened the redirect link yet.
    #[default]
    Created,
    /// The customer opened the link but has not finished. Ambiguous, not terminal.
    Pending,
    /// Final. D24: "No transaction will change its status from DECLINED."
    Declined,
    Cancelled,
    Expired,
    /// Funds released early to the merchant while the customer has *not* paid.
    /// Mapping this to `Charged` would book unpaid revenue.
    EarlyReleased,
    /// Transient anti-fraud hold; resolves either way.
    ForReview,
}

impl From<D24PaymentStatus> for common_enums::AttemptStatus {
    fn from(item: D24PaymentStatus) -> Self {
        match item {
            D24PaymentStatus::Completed => Self::Charged,
            D24PaymentStatus::Created => Self::AuthenticationPending,
            D24PaymentStatus::Pending
            | D24PaymentStatus::EarlyReleased
            | D24PaymentStatus::ForReview => Self::Pending,
            D24PaymentStatus::Declined => Self::Failure,
            // D24 documents that EXPIRED/CANCELLED can revert to COMPLETED after manual
            // intervention; HS treats both as terminal and stops polling. Reconciled out of band.
            D24PaymentStatus::Cancelled => Self::Voided,
            D24PaymentStatus::Expired => Self::Expired,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct D24PaymentsResponse {
    status: D24PaymentStatus,
    deposit_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, D24PaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, D24PaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.deposit_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                network_txn_link_id: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct D24ErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}
