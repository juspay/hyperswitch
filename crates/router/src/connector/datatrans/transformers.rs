use std::fmt::Debug;

use base64::Engine;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    consts,
    core::errors,
    types::{
        self,
        api::{self, enums},
        domain,
    },
};

pub struct DatatransAuthType {
    pub(super) auth_header: String,
}

pub struct DatatransRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

#[derive(Debug, Serialize, Deserialize, Eq, Clone, PartialEq)]
pub struct DatatransPaymentsRequest {
    pub amount: i64,
    pub currency: String,
    #[serde(flatten)]
    pub card: CardType,
    pub refno: String,
    #[serde(rename = "autoSettle")]
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
    ChallengeRequired,
    ChallengeOngoing,
    Authenticated,
    Authorized,
    Settled,
    Canceled,
    Transmitted,
    Failed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SyncResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "type")]
    pub res_type: TransactionType,
    pub status: TransactionStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DataTransSyncResponse {
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

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct PlainCardDetails {
    #[serde(rename = "type")]
    pub res_type: String,
    pub number: cards::CardNumber,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: Secret<String>,
    #[serde(rename = "expiryYear")]
    pub expiry_year: Secret<String>,
}
#[derive(Debug, Serialize, Deserialize, Eq, Clone, PartialEq)]
pub enum CardType {
    #[serde(rename = "card")]
    PLAIN(PlainCardDetails),
    ALIAS,
    NetworkTOKEN,
    DeviceTOKEN,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize, PartialEq, Eq)]
pub struct DatatransError {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DatatransResponse {
    pub error: Option<DatatransError>,
    #[serde(rename = "transactionId")]
    pub transaction_id: Option<String>,
    #[serde(rename = "acquirerAuthorizationCode")]
    pub acquire_authorization_code: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DataTransSuccessResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "acquirerAuthorizationCode")]
    pub acquire_authorization_code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataTransRefundsResponse {
    Error(DatatransError),
    Success(DataTransSuccessResponse),
}

#[derive(Default, Debug, Serialize)]
pub struct DatatransRefundRequest {
    pub amount: i64,
    pub currency: String,
    pub refno: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, Clone, PartialEq)]
pub struct DataPaymentCaptureRequest {
    pub amount: i64,
    pub currency: String,
    pub refno: String,
}

impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for DatatransRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
    ) -> Result<Self, Self::Error> {
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
            domain::PaymentMethodData::Card(req_card) => {
                let card = CardType::PLAIN(PlainCardDetails {
                    res_type: "PLAIN".to_string(),
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                });

                let amount = item.amount;
                let currency = item.router_data.request.currency.to_string();
                let refno = item.router_data.connector_request_reference_id.clone();
                let auto_settle = matches!(
                    item.router_data.request.capture_method,
                    Some(enums::CaptureMethod::Automatic)
                );
                Ok(Self {
                    amount,
                    currency,
                    card,
                    refno,
                    auto_settle,
                })
            },
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
            | domain::PaymentMethodData::CardToken(_) => {
                Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into())
        }
    }   
}
}
impl TryFrom<&types::ConnectorAuthType> for DatatransAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey {
                api_key: passcode,
                key1: merchant_id,
            } => {
                let auth_key = format!("{}:{}", merchant_id.peek(), passcode.peek());
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self { auth_header })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl From<DatatransResponse> for enums::AttemptStatus {
    fn from(item: DatatransResponse) -> Self {
        if item.error.is_none() {
            Self::Charged
        } else {
            Self::Failure
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
                TransactionStatus::Initialized
                | TransactionStatus::Authenticated
                | TransactionStatus::ChallengeOngoing
                | TransactionStatus::ChallengeRequired => Self::Pending,
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
                | TransactionStatus::ChallengeOngoing
                | TransactionStatus::ChallengeRequired
                | TransactionStatus::Failed => Self::Failure,
            },
            TransactionType::Payment | TransactionType::CardCheck => Self::Failure,
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, DatatransResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, DatatransResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let resource_id = match item.response.transaction_id.as_ref() {
            Some(transaction_id) => {
                types::ResponseId::ConnectorTransactionId(transaction_id.to_string())
            }
            None => types::ResponseId::NoResponseId,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata:None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
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
            currency: item.router_data.request.currency.to_string(),
            refno: item.router_data.connector_request_reference_id.clone(),
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, DataTransRefundsResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, DataTransRefundsResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DataTransRefundsResponse::Error(error) => Ok(Self {
                response: Err(types::ErrorResponse {
                    code: error.code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: error
                        .message
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                }),
                ..item.data
            }),
            DataTransRefundsResponse::Success(response) => Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: response.transaction_id,
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, DataTransSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, DataTransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item.response {
            DataTransSyncResponse::Error(error) => Err(types::ErrorResponse {
                code: error.code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                message: error
                    .message
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                reason: None,
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            DataTransSyncResponse::Response(response) => Ok(types::RefundsResponseData {
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

impl TryFrom<types::PaymentsSyncResponseRouterData<DataTransSyncResponse>>
    for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSyncResponseRouterData<DataTransSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DataTransSyncResponse::Error(error) => {
                let response = Err(types::ErrorResponse {
                    code: error.code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                    message: error
                        .message
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    attempt_status: None,
                    connector_transaction_id: None,
                    status_code: item.http_code,
                });
                Ok(Self {
                    response,
                    ..item.data
                })
            }
            DataTransSyncResponse::Response(response) => {
                let resource_id =
                    types::ResponseId::ConnectorTransactionId(response.transaction_id.to_string());
                Ok(Self {
                    status: enums::AttemptStatus::from(response),
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id,
                        redirection_data: None,
                        mandate_reference: None,
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
        let amount = item.amount;
        let currency = item.router_data.request.currency.to_string();
        let refno = item.router_data.connector_request_reference_id.clone();
        Ok(Self {
            amount,
            currency,
            refno,
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
            DataTransCaptureResponse::Error(error) => match error
                .message
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string())
                .as_str()
            {
                "already settled" => common_enums::AttemptStatus::Charged,
                _ => common_enums::AttemptStatus::Failure,
            },
            DataTransCaptureResponse::Empty => match item.http_code {
                204 => common_enums::AttemptStatus::Charged,
                _ => common_enums::AttemptStatus::Failure,
            },
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
            DataTransCancelResponse::Empty => match item.http_code {
                204 => common_enums::AttemptStatus::Voided,
                _ => common_enums::AttemptStatus::Failure,
            },
            DataTransCancelResponse::Error(error) => {
                match error
                    .message
                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string())
                    .as_str()
                {
                    "transaction already canceled" => common_enums::AttemptStatus::Voided,
                    _ => common_enums::AttemptStatus::Failure,
                }
            }
        };
        Ok(Self {
            status,
            ..item.data
        })
    }
}
