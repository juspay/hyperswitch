use common_utils::pii::Email;
use error_stack::IntoReport;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{CardData, PaymentsAuthorizeRequestData, RouterData},
    core::errors,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequestMetaData {
    tax: i64,
}

#[derive(Debug, Serialize)]
pub struct StaxPaymentsRequest {
    payment_method_id: Secret<String>,
    total: i64,
    is_refundable: bool,
    pre_auth: bool,
    meta: StaxPaymentsRequestMetaData,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for StaxPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(_) => {
                let pre_auth = !item.request.is_auto_capture()?;
                Ok(Self {
                    meta: StaxPaymentsRequestMetaData { tax: 0 },
                    total: item.request.amount,
                    is_refundable: true,
                    pre_auth,
                    payment_method_id: Secret::new(item.get_payment_method_token()?),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct StaxAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StaxAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: Secret::new(api_key.to_string()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StaxCustomerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    firstname: Option<String>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for StaxCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: item.request.email.to_owned(),
            firstname: item.request.name.to_owned(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxCustomerResponse {
    id: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxCustomerResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct StaxTokenizeData {
    person_name: Secret<String>,
    card_number: cards::CardNumber,
    card_exp: Secret<String>,
    card_cvv: Secret<String>,
    customer_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum StaxTokenRequest {
    Card(StaxTokenizeData),
}

impl TryFrom<&types::TokenizationRouterData> for StaxTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        let customer_id = item.get_connector_customer_id()?;
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(card_data) => {
                let stax_card_data = StaxTokenizeData {
                    card_exp: card_data
                        .get_card_expiry_month_year_2_digit_with_delimiter("".to_string()),
                    person_name: card_data.card_holder_name,
                    card_number: card_data.card_number,
                    card_cvv: card_data.card_cvc,
                    customer_id: Secret::new(customer_id),
                };
                Ok(Self::Card(stax_card_data))
            }
            api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward(_)
            | api::PaymentMethodData::Upi(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))
            .into_report(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaxTokenResponse {
    id: Secret<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.id.peek().to_string(),
            }),
            ..item.data
        })
    }
}

// PaymentsResponse
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StaxPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<StaxPaymentStatus> for enums::AttemptStatus {
    fn from(item: StaxPaymentStatus) -> Self {
        match item {
            StaxPaymentStatus::Succeeded => Self::Charged,
            StaxPaymentStatus::Failed => Self::Failure,
            StaxPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxPaymentResponseTypes {
    Charge,
    PreAuth,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaxChildCapture {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaxPaymentsResponse {
    success: bool,
    id: String,
    is_captured: i8,
    is_voided: bool,
    child_captures: Vec<StaxChildCapture>,
    #[serde(rename = "type")]
    payment_response_type: StaxPaymentResponseTypes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StaxMetaData {
    pub id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StaxPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let mut connector_metadata = None;
        let mut status = match item.response.success {
            true => match item.response.payment_response_type {
                StaxPaymentResponseTypes::Charge => enums::AttemptStatus::Charged,
                StaxPaymentResponseTypes::PreAuth => match item.response.is_captured {
                    0 => enums::AttemptStatus::Authorized,
                    _ => {
                        connector_metadata =
                            item.response.child_captures.first().map(|child_captures| {
                                serde_json::json!(StaxMetaData {
                                    id: child_captures.id.clone()
                                })
                            });
                        enums::AttemptStatus::Charged
                    }
                },
            },
            false => enums::AttemptStatus::Failure,
        };
        if item.response.is_voided {
            status = enums::AttemptStatus::Voided;
        }

        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaxCaptureRequest {
    total: Option<i64>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for StaxCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let total = item.request.amount_to_capture;
        Ok(Self { total: Some(total) })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
pub struct StaxRefundRequest {
    pub total: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for StaxRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            total: item.request.refund_amount,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChildTransactionsInResponse {
    id: String,
    success: bool,
}
#[derive(Debug, Clone, Deserialize)]
pub struct RefundResponse {
    id: String,
    success: bool,
    child_transactions: Vec<ChildTransactionsInResponse>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let (connector_refund_id, refund_status) = item.response.child_transactions.first().map_or(
            ("".to_string(), enums::RefundStatus::Failure),
            |child_transaction| {
                (
                    child_transaction.id.clone(),
                    match child_transaction.success {
                        true => enums::RefundStatus::Success,
                        false => enums::RefundStatus::Failure,
                    },
                )
            },
        );

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.success {
            true => enums::RefundStatus::Success,
            false => enums::RefundStatus::Failure,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaxErrorResponseTypes {
    Validation(Vec<String>),
    CardCvv(Vec<String>),
    Id(Vec<String>),
    Total(Vec<String>),
    Authorization(Vec<String>),
    Error(Vec<String>),
    Firstname(Vec<String>),
    Lastname(Vec<String>),
    Email(Vec<String>),
    Company(Vec<String>),
    CardNumber(Vec<String>),
    PersonName(Vec<String>),
    CustomerId(Vec<String>),
    Meta(Vec<String>),
    PaymentMethodId(Vec<String>),
    TokenInvalid(Vec<String>),
}
