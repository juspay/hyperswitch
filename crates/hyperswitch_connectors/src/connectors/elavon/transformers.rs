use crate::{
    types::{
        PaymentsCaptureResponseRouterData, PaymentsSyncResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        CardData, ForeignFrom, PaymentsAuthorizeRequestData, RefundsRequestData, RouterData as _,
    },
};
use cards::CardNumber;
use common_enums::enums;
use common_utils::{pii::Email, types::StringMajorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::consts;
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

pub struct ElavonRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for ElavonRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    CcSale,
    CcAuthOnly,
    CcComplete,
    CcReturn,
    TxnQuery,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum SyncTransactionType {
    Sale,
    AuthOnly,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ElavonPaymentsRequest {
    Card(CardPaymentRequest),
}
#[derive(Debug, Serialize)]
pub struct CardPaymentRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_card_number: CardNumber,
    pub ssl_exp_date: Secret<String>,
    pub ssl_cvv2cvc2: Secret<String>,
    pub ssl_email: Email,
}

impl TryFrom<&ElavonRouterData<&PaymentsAuthorizeRouterData>> for ElavonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Ok(Self::Card(CardPaymentRequest {
                ssl_transaction_type: match item.router_data.request.is_auto_capture()? {
                    true => TransactionType::CcSale,
                    false => TransactionType::CcAuthOnly,
                },
                ssl_account_id: auth.account_id.clone(),
                ssl_user_id: auth.user_id.clone(),
                ssl_pin: auth.pin.clone(),
                ssl_amount: item.amount.clone(),
                ssl_card_number: req_card.card_number.clone(),
                ssl_exp_date: req_card.get_expiry_date_as_mmyy()?,
                ssl_cvv2cvc2: req_card.card_cvc,
                ssl_email: item.router_data.get_billing_email()?,
            })),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct ElavonAuthType {
    pub(super) account_id: Secret<String>,
    pub(super) user_id: Secret<String>,
    pub(super) pin: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ElavonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                account_id: api_key.to_owned(),
                user_id: key1.to_owned(),
                pin: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum SslIssuerResponse {
    #[serde(rename = "00")]
    Success,
    #[serde(rename = "85")]
    NoDeclineReason,
    #[serde(rename = "10")]
    PartialSuccess10,
    #[serde(rename = "11")]
    PartialSuccess11,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum SslResult {
    #[serde(rename = "0")]
    ImportedBatchFile,
    #[serde(other)]
    DeclineOrUnauthorized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElavonPaymentsResponse {
    #[serde(rename = "txn")]
    Success(PaymentResponse),
    #[serde(rename = "txn")]
    Error(ElavonErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElavonErrorResponse {
    error_code: String,
    error_message: String,
    error_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    ssl_issuer_response: Option<SslIssuerResponse>,
    ssl_result: SslResult,
    ssl_txn_id: String,
    ssl_result_message: Option<String>,
}

impl<F>
    TryFrom<
        ResponseRouterData<F, ElavonPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            ElavonPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::foreign_from((
            &item.response,
            item.data.request.is_auto_capture()?,
        ));

        let response = match &item.response {
            ElavonPaymentsResponse::Error(error) => Err(ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            ElavonPaymentsResponse::Success(response) => {
                if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        code: response
                            .ssl_result_message
                            .clone()
                            .unwrap_or_else(|| "NO_ERROR_CODE".to_string()),
                        message: response
                            .ssl_result_message
                            .clone()
                            .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                        reason: Some(
                            response
                                .ssl_result_message
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                        ),
                        attempt_status: None,
                        connector_transaction_id: Some(response.ssl_txn_id.clone()),
                        status_code: item.http_code,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response.ssl_txn_id.clone(),
                        ),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    })
                }
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionSyncStatus {
    Pen, // Pended
    Opn, // Unpended / release / open
    Rev, // Review
    Stl, // Settled
    Pst, // Failed due to post-auth rule
    Fpr, // Failed due to fraud prevention rules
    Pre, // Failed due to pre-auth rule
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct PaymentsCaptureRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_txn_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct PaymentsVoidRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_txn_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct ElavonRefundRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_amount: StringMajorUnit,
    pub ssl_txn_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename = "txn")]
pub struct SyncRequest {
    pub ssl_transaction_type: TransactionType,
    pub ssl_account_id: Secret<String>,
    pub ssl_user_id: Secret<String>,
    pub ssl_pin: Secret<String>,
    pub ssl_txn_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "txn")]
pub struct ElavonSyncResponse {
    pub ssl_trans_status: TransactionSyncStatus,
    pub ssl_transaction_type: SyncTransactionType,
    pub ssl_txn_id: String,
}
impl TryFrom<&RefundSyncRouterData> for SyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.request.get_connector_refund_id()?,
            ssl_transaction_type: TransactionType::TxnQuery,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}
impl TryFrom<&PaymentsSyncRouterData> for SyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item
                .request
                .connector_transaction_id
                .get_connector_transaction_id()
                .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            ssl_transaction_type: TransactionType::TxnQuery,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}
impl<F> TryFrom<&ElavonRouterData<&RefundsRouterData<F>>> for ElavonRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ElavonRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.router_data.request.connector_transaction_id.clone(),
            ssl_amount: item.amount.clone(),
            ssl_transaction_type: TransactionType::CcReturn,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}

impl TryFrom<&ElavonRouterData<&PaymentsCaptureRouterData>> for PaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ElavonRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = ElavonAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            ssl_txn_id: item.router_data.request.connector_transaction_id.clone(),
            ssl_amount: item.amount.clone(),
            ssl_transaction_type: TransactionType::CcComplete,
            ssl_account_id: auth.account_id.clone(),
            ssl_user_id: auth.user_id.clone(),
            ssl_pin: auth.pin.clone(),
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<ElavonSyncResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<ElavonSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(&item.response),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ssl_txn_id),
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
impl TryFrom<RefundsResponseRouterData<RSync, ElavonSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, ElavonSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ssl_txn_id.clone(),
                refund_status: enums::RefundStatus::from(&item.response),
            }),
            ..item.data
        })
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<ElavonPaymentsResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<ElavonPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status =
            enums::AttemptStatus::foreign_from((&item.response, enums::AttemptStatus::Charged));
        let response = match &item.response {
            ElavonPaymentsResponse::Error(error) => Err(ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            ElavonPaymentsResponse::Success(response) => {
                if status == enums::AttemptStatus::Failure {
                    Err(ErrorResponse {
                        code: response
                            .ssl_result_message
                            .clone()
                            .unwrap_or_else(|| "NO_ERROR_CODE".to_string()),
                        message: response
                            .ssl_result_message
                            .clone()
                            .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                        reason: Some(
                            response
                                .ssl_result_message
                                .clone()
                                .unwrap_or_else(|| "NO_ERROR_MESSAGE".to_string()),
                        ),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(
                            response.ssl_txn_id.clone(),
                        ),
                        redirection_data: None,
                        mandate_reference: None,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charge_id: None,
                    })
                }
            }
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}
impl TryFrom<RefundsResponseRouterData<Execute, ElavonPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, ElavonPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let status = enums::RefundStatus::from(&item.response);
        let response = match &item.response {
            ElavonPaymentsResponse::Error(error) => Err(ErrorResponse {
                code: error.error_code.clone(),
                message: error.error_message.clone(),
                reason: Some(error.error_message.clone()),
                attempt_status: None,
                connector_transaction_id: None,
                status_code: item.http_code,
            }),
            ElavonPaymentsResponse::Success(response) => {
                if status == enums::RefundStatus::Failure {
                    Err(ErrorResponse {
                        code: consts::NO_ERROR_CODE.to_owned(),
                        message: consts::NO_ERROR_MESSAGE.to_owned(),
                        reason: Some(consts::NO_ERROR_MESSAGE.to_owned()),
                        attempt_status: None,
                        connector_transaction_id: None,
                        status_code: item.http_code,
                    })
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: response.ssl_txn_id.clone(),
                        refund_status: enums::RefundStatus::from(&item.response),
                    })
                }
            }
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

trait ElavonResponseValidator {
    fn is_successful(&self) -> bool;
}
impl ElavonResponseValidator for ElavonPaymentsResponse {
    fn is_successful(&self) -> bool {
        match self {
            Self::Error(_) => false,
            Self::Success(response) => {
                if !matches!(response.ssl_result, SslResult::ImportedBatchFile) {
                    return false;
                }

                matches!(
                    response.ssl_issuer_response,
                    None | Some(SslIssuerResponse::Success)
                        | Some(SslIssuerResponse::NoDeclineReason)
                        | Some(SslIssuerResponse::PartialSuccess10)
                        | Some(SslIssuerResponse::PartialSuccess11)
                )
            }
        }
    }
}

impl ForeignFrom<(&ElavonPaymentsResponse, enums::AttemptStatus)> for enums::AttemptStatus {
    fn foreign_from(
        (item, success_status): (&ElavonPaymentsResponse, enums::AttemptStatus),
    ) -> Self {
        if item.is_successful() {
            success_status
        } else {
            Self::Failure
        }
    }
}

impl From<&ElavonPaymentsResponse> for enums::RefundStatus {
    fn from(item: &ElavonPaymentsResponse) -> Self {
        if item.is_successful() {
            Self::Success
        } else {
            Self::Failure
        }
    }
}
impl From<&ElavonSyncResponse> for enums::RefundStatus {
    fn from(item: &ElavonSyncResponse) -> Self {
        match item.ssl_trans_status {
            TransactionSyncStatus::Rev
            | TransactionSyncStatus::Opn
            | TransactionSyncStatus::Pen => Self::Pending,
            TransactionSyncStatus::Stl => Self::Success,
            TransactionSyncStatus::Pst
            | TransactionSyncStatus::Fpr
            | TransactionSyncStatus::Pre => Self::Failure,
        }
    }
}
impl From<&ElavonSyncResponse> for enums::AttemptStatus {
    fn from(item: &ElavonSyncResponse) -> Self {
        match item.ssl_trans_status {
            TransactionSyncStatus::Rev
            | TransactionSyncStatus::Opn
            | TransactionSyncStatus::Pen => Self::Pending,
            TransactionSyncStatus::Stl => match item.ssl_transaction_type {
                SyncTransactionType::Sale => Self::Charged,
                SyncTransactionType::AuthOnly => Self::Authorized,
            },
            TransactionSyncStatus::Pst
            | TransactionSyncStatus::Fpr
            | TransactionSyncStatus::Pre => Self::Failure,
        }
    }
}

impl ForeignFrom<(&ElavonPaymentsResponse, bool)> for enums::AttemptStatus {
    fn foreign_from((item, is_auto_capture): (&ElavonPaymentsResponse, bool)) -> Self {
        if item.is_successful() {
            if is_auto_capture {
                Self::Charged
            } else {
                Self::Authorized
            }
        } else {
            Self::Failure
        }
    }
}
