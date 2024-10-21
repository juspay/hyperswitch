use std::fmt::Debug;

use common_utils::types::MinorUnit;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::{
        utils as connector_utils,
        utils::{CardData, PaymentsAuthorizeRequestData},
    },
    core::errors,
    types::{
        self,
        api::{self, enums},
        domain,
        transformers::ForeignFrom,
    },
};

const TRANSACTION_ALREADY_CANCELLED: &str = "transaction already canceled";
const TRANSACTION_ALREADY_SETTLED: &str = "already settled";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DatatransErrorResponse {
    pub error: DatatransError,
}
pub struct DatatransAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) passcode: Secret<String>,
}

pub struct DatatransRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DatatransPaymentsRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub card: PlainCardDetails,
    pub refno: String,
    pub auto_settle: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Payment,
    Credit,
    CardCheck,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Initialized,
    Authenticated,
    Authorized,
    Settled,
    Canceled,
    Transmitted,
    Failed,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DatatransSyncResponse {
    Error(DatatransError),
    Response(SyncResponse),
}
#[derive(Debug, Deserialize, Serialize)]
pub enum DataTransCaptureResponse {
    Error(DatatransError),
    Empty,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DataTransCancelResponse {
    Error(DatatransError),
    Empty,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub res_type: TransactionType,
    pub status: TransactionStatus,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlainCardDetails {
    #[serde(rename = "type")]
    pub res_type: String,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize)]
pub struct DatatransError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DatatransResponse {
    TransactionResponse(DatatransSuccessResponse),
    ErrorResponse(DatatransError),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatatransSuccessResponse {
    pub transaction_id: String,
    pub acquirer_authorization_code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DatatransRefundsResponse {
    Success(DatatransSuccessResponse),
    Error(DatatransError),
}

#[derive(Default, Debug, Serialize)]
pub struct DatatransRefundRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub refno: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DataPaymentCaptureRequest {
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    pub refno: String,
}

impl<T> TryFrom<(MinorUnit, T)> for DatatransRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl TryFrom<&DatatransRouterData<&types::PaymentsAuthorizeRouterData>>
    for DatatransPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            domain::PaymentMethodData::Card(req_card) => Ok(Self {
                amount: item.amount,
                currency: item.router_data.request.currency,
                card: PlainCardDetails {
                    res_type: "PLAIN".to_string(),
                    number: req_card.card_number.clone(),
                    expiry_month: req_card.card_exp_month.clone(),
                    expiry_year: req_card.get_card_expiry_year_2_digit()?,
                },
                refno: item.router_data.connector_request_reference_id.clone(),
                auto_settle: matches!(
                    item.router_data.request.capture_method,
                    Some(enums::CaptureMethod::Automatic)
                ),
            }),
            domain::PaymentMethodData::Wallet(_)
            | domain::PaymentMethodData::PayLater(_)
            | domain::PaymentMethodData::BankRedirect(_)
            | domain::PaymentMethodData::BankDebit(_)
            | domain::PaymentMethodData::BankTransfer(_)
            | domain::PaymentMethodData::Crypto(_)
            | domain::PaymentMethodData::MandatePayment
            | domain::PaymentMethodData::Reward
            | domain::PaymentMethodData::RealTimePayment(_)
            | domain::PaymentMethodData::Upi(_)
            | domain::PaymentMethodData::CardRedirect(_)
            | domain::PaymentMethodData::Voucher(_)
            | domain::PaymentMethodData::GiftCard(_)
            | domain::PaymentMethodData::OpenBanking(_)
            | domain::PaymentMethodData::CardToken(_)
            | domain::PaymentMethodData::NetworkToken(_)
            | domain::PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    connector_utils::get_unimplemented_payment_method_error_message("Datatrans"),
                ))?
            }
        }
    }
}
impl TryFrom<&types::ConnectorAuthType> for DatatransAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_id: key1.clone(),
                passcode: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl ForeignFrom<(&DatatransResponse, bool)> for enums::AttemptStatus {
    fn foreign_from((item, is_auto_capture): (&DatatransResponse, bool)) -> Self {
        match item {
            DatatransResponse::ErrorResponse(_) => Self::Failure,
            DatatransResponse::TransactionResponse(_) => {
                if is_auto_capture {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
        }
    }
}
impl From<SyncResponse> for enums::AttemptStatus {
    fn from(item: SyncResponse) -> Self {
        match item.res_type {
            TransactionType::Payment => match item.status {
                TransactionStatus::Authorized => Self::Authorized,
                TransactionStatus::Settled | TransactionStatus::Transmitted => Self::Charged,
                TransactionStatus::Canceled => Self::Voided,
                TransactionStatus::Failed => Self::Failure,
                TransactionStatus::Initialized | TransactionStatus::Authenticated => Self::Pending,
            },
            TransactionType::Credit | TransactionType::CardCheck => Self::Failure,
        }
    }
}

impl From<SyncResponse> for enums::RefundStatus {
    fn from(item: SyncResponse) -> Self {
        match item.res_type {
            TransactionType::Credit => match item.status {
                TransactionStatus::Settled | TransactionStatus::Transmitted => Self::Success,
                TransactionStatus::Initialized
                | TransactionStatus::Authenticated
                | TransactionStatus::Authorized
                | TransactionStatus::Canceled
                | TransactionStatus::Failed => Self::Failure,
            },
            TransactionType::Payment | TransactionType::CardCheck => Self::Failure,
        }
    }
}

impl<F>
    TryFrom<
        types::ResponseRouterData<
            F,
            DatatransResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            DatatransResponse,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::foreign_from((
            &item.response,
            item.data.request.is_auto_capture()?,
        ));
        let response = match &item.response {
            DatatransResponse::ErrorResponse(error) => Err(types::ErrorResponse {
                code: error.code.clone(),
                message: error.message.clone(),
                reason: Some(error.message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DatatransResponse::TransactionResponse(response) => {
                Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        response.transaction_id.clone(),
                    ),
                    redirection_data: None,
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charge_id: None,
                })
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F> TryFrom<&DatatransRouterData<&types::RefundsRouterData<F>>> for DatatransRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency,
            refno: item.router_data.request.refund_id.clone(),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, DatatransRefundsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, DatatransRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DatatransRefundsResponse::Error(error) => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: error.code.clone(),
                    message: error.message.clone(),
                    reason: Some(error.message),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
            DatatransRefundsResponse::Success(response) => Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: response.transaction_id,
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, DatatransSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, DatatransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            DatatransSyncResponse::Error(error) => Err(types::ErrorResponse {
                code: error.code.clone(),
                message: error.message.clone(),
                reason: Some(error.message),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DatatransSyncResponse::Response(response) => Ok(types::RefundsResponseData {
                connector_refund_id: response.transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(response),
            }),
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsSyncResponseRouterData<DatatransSyncResponse>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<DatatransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DatatransSyncResponse::Error(error) => {
                let response = Err(types::ErrorResponse {
                    code: error.code.clone(),
                    message: error.message.clone(),
                    reason: Some(error.message),
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                });
                Ok(Self {
                    response,
                    ..item.data
                })
            }
            DatatransSyncResponse::Response(response) => {
                let resource_id =
                    types::ResponseId::ConnectorTransactionId(response.transaction_id.to_string());
                Ok(Self {
                    status: enums::AttemptStatus::from(response),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id,
                        redirection_data: None,
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&DatatransRouterData<&types::PaymentsCaptureRouterData>>
    for DataPaymentCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DatatransRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            refno: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl TryFrom<types::PaymentsCaptureResponseRouterData<DataTransCaptureResponse>>
    for types::PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::PaymentsCaptureResponseRouterData<DataTransCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response {
            DataTransCaptureResponse::Error(error) => {
                if error.message == *TRANSACTION_ALREADY_SETTLED {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
            // Datatrans http code 204 implies Successful Capture
            //https://api-reference.datatrans.ch/#tag/v1transactions/operation/settle
            DataTransCaptureResponse::Empty => {
                if item.http_code == 204 {
                    common_enums::AttemptStatus::Charged
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
        };
        Ok(Self {
            status,
            ..item.data
        })
    }
}

impl TryFrom<types::PaymentsCancelResponseRouterData<DataTransCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: types::PaymentsCancelResponseRouterData<DataTransCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status = match item.response {
            // Datatrans http code 204 implies Successful Cancellation
            //https://api-reference.datatrans.ch/#tag/v1transactions/operation/cancel
            DataTransCancelResponse::Empty => {
                if item.http_code == 204 {
                    common_enums::AttemptStatus::Voided
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
            DataTransCancelResponse::Error(error) => {
                if error.message == *TRANSACTION_ALREADY_CANCELLED {
                    common_enums::AttemptStatus::Voided
                } else {
                    common_enums::AttemptStatus::Failure
                }
            }
        };
        Ok(Self {
            status,
            ..item.data
        })
    }
}
