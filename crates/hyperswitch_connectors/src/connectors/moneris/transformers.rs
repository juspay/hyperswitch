use common_enums::enums;
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{AccessToken, ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefreshTokenRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        BrowserInformationData, CardData as _, PaymentsAuthorizeRequestData,
        RouterData as OtherRouterData,
    },
};

const CLIENT_CREDENTIALS: &str = "client_credentials";

pub struct MonerisRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for MonerisRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub mod auth_headers {
    pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
    pub const API_VERSION: &str = "Api-Version";
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonerisPaymentsRequest {
    idempotency_key: String,
    amount: Amount,
    payment_method: PaymentMethod,
    automatic_capture: bool,
    ipv4: Secret<String, common_utils::pii::IpAddress>,
}
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    currency: enums::Currency,
    amount: MinorUnit,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethod {
    payment_method_source: PaymentMethodSource,
    card: MonerisCard,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethodSource {
    Card,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonerisCard {
    card_number: cards::CardNumber,
    expiry_month: Secret<i64>,
    expiry_year: Secret<i64>,
    card_security_code: Secret<String>,
}

impl TryFrom<&MonerisRouterData<&PaymentsAuthorizeRouterData>> for MonerisPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &MonerisRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Moneris",
            })?
        };
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ref req_card) => {
                let idempotency_key = uuid::Uuid::new_v4().to_string();
                let amount = Amount {
                    currency: item.router_data.request.currency,
                    amount: item.amount,
                };
                let payment_method = PaymentMethod {
                    payment_method_source: PaymentMethodSource::Card,
                    card: MonerisCard {
                        card_number: req_card.card_number.clone(),
                        expiry_month: Secret::new(
                            req_card
                                .card_exp_month
                                .peek()
                                .parse::<i64>()
                                .change_context(errors::ConnectorError::ParsingFailed)?,
                        ),
                        expiry_year: Secret::new(
                            req_card
                                .get_expiry_year_4_digit()
                                .peek()
                                .parse::<i64>()
                                .change_context(errors::ConnectorError::ParsingFailed)?,
                        ),
                        card_security_code: req_card.card_cvc.clone(),
                    },
                };
                let automatic_capture = item.router_data.request.is_auto_capture()?;

                let browser_info = item.router_data.request.get_browser_info()?;
                let ipv4 = browser_info.get_ip_address()?;

                Ok(Self {
                    idempotency_key,
                    amount,
                    payment_method,
                    automatic_capture,
                    ipv4,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct MonerisAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
    pub(super) merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for MonerisAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                client_id: key1.to_owned(),
                client_secret: api_key.to_owned(),
                merchant_id: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MonerisAuthRequest {
    client_id: Secret<String>,
    client_secret: Secret<String>,
    grant_type: String,
}

impl TryFrom<&RefreshTokenRouterData> for MonerisAuthRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = MonerisAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
            grant_type: CLIENT_CREDENTIALS.to_string(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonerisAuthResponse {
    access_token: Secret<String>,
    token_type: String,
    expires_in: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, MonerisAuthResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MonerisAuthResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item
                    .response
                    .expires_in
                    .parse::<i64>()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MonerisPaymentStatus {
    Succeeded,
    #[default]
    Processing,
    Canceled,
    Declined,
    DeclinedRetry,
    Authorized,
}

impl From<MonerisPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: MonerisPaymentStatus) -> Self {
        match item {
            MonerisPaymentStatus::Succeeded => Self::Charged,
            MonerisPaymentStatus::Authorized => Self::Authorized,
            MonerisPaymentStatus::Canceled => Self::Voided,
            MonerisPaymentStatus::Declined | MonerisPaymentStatus::DeclinedRetry => Self::Failure,
            MonerisPaymentStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonerisPaymentsResponse {
    payment_status: MonerisPaymentStatus,
    payment_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, MonerisPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MonerisPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.payment_status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonerisPaymentsCaptureRequest {
    amount: Amount,
    idempotency_key: String,
}

impl TryFrom<&MonerisRouterData<&PaymentsCaptureRouterData>> for MonerisPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MonerisRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.router_data.request.currency,
            amount: item.amount,
        };
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        Ok(Self {
            amount,
            idempotency_key,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonerisCancelRequest {
    idempotency_key: String,
    reason: Option<String>,
}

impl TryFrom<&PaymentsCancelRouterData> for MonerisCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let reason = item.request.cancellation_reason.clone();
        Ok(Self {
            idempotency_key,
            reason,
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonerisRefundRequest {
    pub refund_amount: Amount,
    pub idempotency_key: String,
    pub reason: Option<String>,
    pub payment_id: String,
}

impl<F> TryFrom<&MonerisRouterData<&RefundsRouterData<F>>> for MonerisRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MonerisRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let refund_amount = Amount {
            currency: item.router_data.request.currency,
            amount: item.amount,
        };
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let reason = item.router_data.request.reason.clone();
        let payment_id = item.router_data.request.connector_transaction_id.clone();
        Ok(Self {
            refund_amount,
            idempotency_key,
            reason,
            payment_id,
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Succeeded,
    #[default]
    Processing,
    Declined,
    DeclinedRetry,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Declined | RefundStatus::DeclinedRetry => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_id: String,
    refund_status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.refund_status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonerisErrorResponse {
    pub status: u16,
    pub category: String,
    pub title: String,
    pub errors: Option<Vec<MonerisError>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonerisError {
    pub reason_code: String,
    pub parameter_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MonerisAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}
