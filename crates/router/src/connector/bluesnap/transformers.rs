use serde::{Deserialize, Serialize};
use crate::{
    core::errors,
    pii::{self, PeekInterface, Secret},
    types::{self, api, storage::enums},
};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsRequest {
    amount: String,
    credit_card: Card,
    currency: String,
    soft_descriptor: Option<String>,
    card_transaction_type: BluesnapPaymentStatus,
    card_holder_info: CardHolderInfo,
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapVoidRequest {
    card_transaction_type: BluesnapPaymentStatus,
    transaction_id: String
}

impl TryFrom<&types::PaymentsCancelRouterData> for BluesnapVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = BluesnapPaymentStatus::AuthReversal;
        let transaction_id = String::from(&item.request.connector_transaction_id);

        Ok(Self {
            card_transaction_type,
            transaction_id,
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCaptureRequest {
    card_transaction_type: BluesnapPaymentStatus,
    transaction_id: String,
    amount: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for BluesnapCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = BluesnapPaymentStatus::Capture;
        let transaction_id = String::from(&item.request.connector_transaction_id);
        match item.request.amount_to_capture {
            Some(amount_capture) => Ok(Self {
                    card_transaction_type,
                    transaction_id,
                    amount: Some(amount_capture.to_string()),
                }),
            _ => Ok(Self {
                card_transaction_type,
                transaction_id,
                amount: None,
            }),
        }
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    card_number: Secret<String, pii::CardNumber>,
    expiration_month: String,
    expiration_year: String,
    security_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardHolderInfo {
    first_name: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BluesnapPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let capture_method = if let Some(capture_method) = item.request.capture_method {
                    capture_method.to_string()
                } else {
                    "automatic".to_string()
                };
                let auth_mode = if capture_method.to_lowercase() == "manual" {
                    BluesnapPaymentStatus::AuthOnly
                } else {
                    BluesnapPaymentStatus::AuthCapture
                };
                let payment_request = Self {
                    amount: item.request.amount.to_string(),
                    credit_card: Card {
                        card_number: ccard.card_number.clone(),
                        expiration_month: ccard.card_exp_month.peek().clone(),
                        expiration_year: ccard.card_exp_year.peek().clone(),
                        security_code: ccard.card_cvc.peek().clone(),
                    },
                    currency: item.request.currency.to_string(),
                    soft_descriptor: item.description.clone(),
                    card_transaction_type: auth_mode,
                    card_holder_info: CardHolderInfo {
                        first_name: ccard.card_holder_name.peek().clone(),
                    },
                };
                Ok(payment_request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
        }
    }
}

// Auth Struct
pub struct BluesnapAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BluesnapAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapPaymentStatus {
    #[default]
    AuthOnly,
    AuthCapture,
    AuthReversal,
    Capture,
}

impl From<BluesnapPaymentStatus> for enums::AttemptStatus {
    fn from(item: BluesnapPaymentStatus) -> Self {
        match item {
            BluesnapPaymentStatus::AuthOnly => Self::Authorized,
            BluesnapPaymentStatus::AuthCapture => Self::Charged,
            BluesnapPaymentStatus::AuthReversal => Self::Voided,
            BluesnapPaymentStatus::Capture => Self::CaptureInitiated,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsResponse {
    processing_info: BluesnapPaymentsProcessingInfoResponse,
    transaction_id: String,
    refunds: Option<RefundObj>,
    card_transaction_type: BluesnapPaymentStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundObj {
    balance_amount: String,
    refund: Vec<Refund>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    refund_transaction_id: String,
    amount: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsProcessingInfoResponse {
    processing_status: String,
    cvv_response_code: String,
    authorization_code: String,
    avs_response_code_zip: String,
    avs_response_code_address: String,
    avs_response_code_name:String,
}


impl<F,T> TryFrom<types::ResponseRouterData<F, BluesnapPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: types::ResponseRouterData<F, BluesnapPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.card_transaction_type),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.transaction_id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Eq, PartialEq, Serialize)]
pub struct BluesnapRefundRequestData {
    amount: String,
    reason: String,
}

#[derive(Default, Debug, Serialize)]
pub struct BluesnapRefundRequest {
    refund: BluesnapRefundRequestData,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BluesnapRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            refund: BluesnapRefundRequestData {
                reason: item.request.reason.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "item.request.reason",
                    },
                )?,
                amount: item.request.amount.to_string(),
            },
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    #[default]
    Succeeded,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            //TODO: Review mapping
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_transaction_id: i32,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundSyncResponse {
    refunds: RefundObject,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundObject {
    refund: Vec<RefundResponse>,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_item = match item.response.refunds.refund.first() {
            Some(refund_item) => refund_item,
            _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: refund_item.refund_transaction_id.to_string(),
                refund_status: enums::RefundStatus::Success,
            }),
            ..item.data
        })
    }
}


impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
       
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_transaction_id.to_string(),
                refund_status: enums::RefundStatus::Success,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BluesnapErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
}
