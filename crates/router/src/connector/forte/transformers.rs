use serde::{Deserialize, Serialize};
use masking::Secret;
use crate::{connector::utils::PaymentsAuthorizeRequestData,core::errors,types::{self,api, storage::enums, transformers:: ForeignTryFrom}};

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FortePaymentsRequest {
    amount: i64,
    card: ForteCard,
    card_transaction_type: ForteTxnType,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ForteCard {
    name: Secret<String>,
    number: Secret<String, common_utils::pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for FortePaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self,Self::Error> {
        let auth_mode = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => ForteTxnType::AuthOnly,
            _ => ForteTxnType::AuthCapture,
        };
        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card = ForteCard {
                    name: req_card.card_holder_name,
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.request.is_auto_capture(),
                };
                Ok(Self {
                    amount: item.request.amount,
                    card_transaction_type: auth_mode,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string(),
        )),
        }    
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForteVoidRequest {
    card_transaction_type: ForteTxnType,
    transaction_id: String,
}

impl TryFrom<&types::PaymentsCancelRouterData> for ForteVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = ForteTxnType::AuthReversal;
        let transaction_id = item.request.connector_transaction_id.to_string();
        Ok(Self {
            card_transaction_type,
            transaction_id,
        })
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForteCaptureRequest {
    card_transaction_type: ForteTxnType,
    transaction_id: String,
    amount: i64,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for ForteCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = ForteTxnType::Capture;
        let transaction_id = item.request.connector_transaction_id.to_string();
        let amount = i64 (
            item.request.amount_to_capture,
        )?;
        Ok(Self {
            card_transaction_type,
            transaction_id,
            amount: Some(amount),
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ForteAuthType {
    pub(super) api_key: String,
    pub(super) key1: String,
}

impl TryFrom<&types::ConnectorAuthType> for ForteAuthType  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key , key1} => Ok(Self {
                api_key: api_key.to_string(),
                key1: key1.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Payments Response
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ForteTxnType {
    AuthOnly,
    AuthCapture,
    AuthReversal,
    Capture,
    Refund,
}

pub enum FortePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    Pending,
    PendingMerchantReview,
}

impl ForeignTryFrom<(ForteTxnType, FortePaymentStatus )> for enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (ForteTxnType, FortePaymentStatus ,
    ) -> Result<Self, Self::Error> {
        let (item_txn_status, item_processing_status) = item;
        Ok(match item_processing_status {
            FortePaymentStatus ::Succeeded => match item_txn_status {
                ForteTxnType::AuthOnly => Self::Authorized,
                ForteTxnType::AuthReversal => Self::Voided,
                ForteTxnType::AuthCapture | ForteTxnType::Capture => Self::Charged,
                ForteTxnType::Refund => Self::Charged,
            },
            FortePaymentStatus::Pending | FortePaymentStatus::PendingMerchantReview => {
                Self::Pending
            }
            FortePaymentStatus::Failed => Self::Failure,
        })
    }
}

impl From<FortePaymentStatus> for enums::RefundStatus {
    fn from(item: FortePaymentStatus) -> Self {
        match item {
            FortePaymentStatus::Succeeded => Self::Charged,
            FortePaymentStatus::Failed => Self::Failure,
            FortePaymentStatus::Processing => Self::Authorizing,
            FortePaymentStatus::Pending => Self::Pending,
            FortePaymentStatus::PendingMerchantReview => Self::ManualReview,

        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FortePaymentsResponse {
    status: FortePaymentStatus,
    amount: i64,
    id: String,
    card_transaction_type: ForteTxnType,
}

impl<F,T> TryFrom<types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, FortePaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// An amount of money that is given back to you, especially because you have paid too much or you are not happy with a product or service.
#[derive(Default, Debug, Serialize)]
pub struct ForteRefundRequest {
    refund_transaction_id: String,
    pub amount: i64,
    reason: Option<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ForteRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
    }
}

// REFUND RESPONSE:
// This term is used when a third-party seller refunds a customer for any reason. in part or full. 

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Succeeded,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
    refund_transaction_id: i32,
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
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
 }

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ForteErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
