use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::CardData as _,
};

//TODO: Fill the struct with respective fields
pub struct FiservemeaRouterData<T> {
    pub amount: StringMajorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for FiservemeaRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FiservemeaTransactionAmount {
    total: StringMajorUnit,
    currency: common_enums::Currency,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaOrder {
    order_id: String,
}

#[derive(Debug, Serialize)]
pub enum FiservemeaRequestType {
    PaymentCardSaleTransaction,
    PaymentCardPreAuthTransaction,
    PostAuthTransaction,
    VoidPreAuthTransactions,
    ReturnTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentCard {
    number: cards::CardNumber,
    expiry_date: FiservemeaExpiryDate,
    security_code: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FiservemeaPaymentMethods {
    PaymentCard(FiservemeaPaymentCard),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentsRequest {
    request_type: FiservemeaRequestType,
    merchant_transaction_id: String,
    transaction_amount: FiservemeaTransactionAmount,
    order: FiservemeaOrder,
    payment_method: FiservemeaPaymentMethods,
}

impl TryFrom<&FiservemeaRouterData<&PaymentsAuthorizeRouterData>> for FiservemeaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FiservemeaRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = FiservemeaPaymentCard {
                    number: req_card.card_number.clone(),
                    expiry_date: FiservemeaExpiryDate {
                        month: req_card.card_exp_month.clone(),
                        year: req_card.get_card_expiry_year_2_digit()?,
                    },
                    security_code: req_card.card_cvc,
                };
                let request_type = if matches!(
                    item.router_data.request.capture_method,
                    Some(enums::CaptureMethod::Automatic)
                        | Some(enums::CaptureMethod::SequentialAutomatic)
                ) {
                    FiservemeaRequestType::PaymentCardSaleTransaction
                } else {
                    FiservemeaRequestType::PaymentCardPreAuthTransaction
                };

                Ok(Self {
                    request_type,
                    merchant_transaction_id: item
                        .router_data
                        .request
                        .merchant_order_reference_id
                        .clone()
                        .unwrap_or(item.router_data.connector_request_reference_id.clone()),
                    transaction_amount: FiservemeaTransactionAmount {
                        total: item.amount.clone(),
                        currency: item.router_data.request.currency,
                    },
                    order: FiservemeaOrder {
                        order_id: item.router_data.connector_request_reference_id.clone(),
                    },
                    payment_method: FiservemeaPaymentMethods::PaymentCard(card),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Selected payment method through fiservemea".to_string(),
            )
            .into()),
        }
    }
}

// Auth Struct
#[derive(Clone)]
pub struct FiservemeaAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) secret_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FiservemeaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                secret_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
#[derive(Debug, Serialize, Deserialize)]
pub enum ResponseType {
    BadRequest,
    Unauthenticated,
    Unauthorized,
    NotFound,
    GatewayDeclined,
    EndpointDeclined,
    ServerError,
    EndpointCommunicationError,
    UnsupportedMediaType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FiservemeaTransactionType {
    Sale,
    Preauth,
    Credit,
    ForcedTicket,
    Void,
    Return,
    Postauth,
    PayerAuth,
    Disbursement,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservemeaTransactionOrigin {
    Ecom,
    Moto,
    Mail,
    Phone,
    Retail,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FiservemeaPaymentStatus {
    Approved,
    Waiting,
    Partial,
    ValidationFailed,
    ProcessingFailed,
    Declined,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FiservemeaPaymentResult {
    Approved,
    Declined,
    Failed,
    Waiting,
    Partial,
    Fraud,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentCardResponse {
    expiry_date: Option<FiservemeaExpiryDate>,
    bin: Option<String>,
    last4: Option<String>,
    brand: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentMethodDetails {
    payment_card: Option<FiservemeaPaymentCardResponse>,
    payment_method_type: Option<String>,
    payment_method_brand: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    subtotal: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmountDetails {
    total: Option<f64>,
    currency: Option<common_enums::Currency>,
    components: Option<Components>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvsResponse {
    street_match: Option<String>,
    postal_code_match: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Processor {
    reference_number: Option<String>,
    authorization_code: Option<String>,
    response_code: Option<String>,
    response_message: Option<String>,
    avs_response: Option<AvsResponse>,
    security_code_response: Option<String>,
}

fn map_status(
    fiservemea_status: Option<FiservemeaPaymentStatus>,
    fiservemea_result: Option<FiservemeaPaymentResult>,
    transaction_type: FiservemeaTransactionType,
) -> common_enums::AttemptStatus {
    match fiservemea_status {
        Some(status) => match status {
            FiservemeaPaymentStatus::Approved => match transaction_type {
                FiservemeaTransactionType::Preauth => common_enums::AttemptStatus::Authorized,
                FiservemeaTransactionType::Void => common_enums::AttemptStatus::Voided,
                FiservemeaTransactionType::Sale | FiservemeaTransactionType::Postauth => {
                    common_enums::AttemptStatus::Charged
                }
                FiservemeaTransactionType::Credit
                | FiservemeaTransactionType::ForcedTicket
                | FiservemeaTransactionType::Return
                | FiservemeaTransactionType::PayerAuth
                | FiservemeaTransactionType::Disbursement => common_enums::AttemptStatus::Failure,
            },
            FiservemeaPaymentStatus::Waiting => common_enums::AttemptStatus::Pending,
            FiservemeaPaymentStatus::Partial => common_enums::AttemptStatus::PartialCharged,
            FiservemeaPaymentStatus::ValidationFailed
            | FiservemeaPaymentStatus::ProcessingFailed
            | FiservemeaPaymentStatus::Declined => common_enums::AttemptStatus::Failure,
        },
        None => match fiservemea_result {
            Some(result) => match result {
                FiservemeaPaymentResult::Approved => match transaction_type {
                    FiservemeaTransactionType::Preauth => common_enums::AttemptStatus::Authorized,
                    FiservemeaTransactionType::Void => common_enums::AttemptStatus::Voided,
                    FiservemeaTransactionType::Sale | FiservemeaTransactionType::Postauth => {
                        common_enums::AttemptStatus::Charged
                    }
                    FiservemeaTransactionType::Credit
                    | FiservemeaTransactionType::ForcedTicket
                    | FiservemeaTransactionType::Return
                    | FiservemeaTransactionType::PayerAuth
                    | FiservemeaTransactionType::Disbursement => {
                        common_enums::AttemptStatus::Failure
                    }
                },
                FiservemeaPaymentResult::Waiting => common_enums::AttemptStatus::Pending,
                FiservemeaPaymentResult::Partial => common_enums::AttemptStatus::PartialCharged,
                FiservemeaPaymentResult::Declined
                | FiservemeaPaymentResult::Failed
                | FiservemeaPaymentResult::Fraud => common_enums::AttemptStatus::Failure,
            },
            None => common_enums::AttemptStatus::Pending,
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentsResponse {
    response_type: Option<ResponseType>,
    #[serde(rename = "type")]
    fiservemea_type: Option<String>,
    client_request_id: Option<String>,
    api_trace_id: Option<String>,
    ipg_transaction_id: String,
    order_id: Option<String>,
    transaction_type: FiservemeaTransactionType,
    transaction_origin: Option<FiservemeaTransactionOrigin>,
    payment_method_details: Option<FiservemeaPaymentMethodDetails>,
    country: Option<Secret<String>>,
    terminal_id: Option<String>,
    merchant_id: Option<String>,
    merchant_transaction_id: Option<String>,
    transaction_time: Option<i64>,
    approved_amount: Option<AmountDetails>,
    transaction_amount: Option<AmountDetails>,
    transaction_status: Option<FiservemeaPaymentStatus>, // FiservEMEA Docs mention that this field is deprecated. We are using it for now because transaction_result is not present in the response.
    transaction_result: Option<FiservemeaPaymentResult>,
    approval_code: Option<String>,
    error_message: Option<String>,
    transaction_state: Option<String>,
    scheme_transaction_id: Option<String>,
    processor: Option<Processor>,
}

impl<F, T> TryFrom<ResponseRouterData<F, FiservemeaPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FiservemeaPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: map_status(
                item.response.transaction_status,
                item.response.transaction_result,
                item.response.transaction_type,
            ),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ipg_transaction_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaCaptureRequest {
    request_type: FiservemeaRequestType,
    transaction_amount: FiservemeaTransactionAmount,
}

impl TryFrom<&FiservemeaRouterData<&PaymentsCaptureRouterData>> for FiservemeaCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FiservemeaRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: FiservemeaRequestType::PostAuthTransaction,
            transaction_amount: FiservemeaTransactionAmount {
                total: item.amount.clone(),
                currency: item.router_data.request.currency,
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaVoidRequest {
    request_type: FiservemeaRequestType,
}

impl TryFrom<&PaymentsCancelRouterData> for FiservemeaVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: FiservemeaRequestType::VoidPreAuthTransactions,
        })
    }
}

// REFUND :
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaRefundRequest {
    request_type: FiservemeaRequestType,
    transaction_amount: FiservemeaTransactionAmount,
}

impl<F> TryFrom<&FiservemeaRouterData<&RefundsRouterData<F>>> for FiservemeaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FiservemeaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            request_type: FiservemeaRequestType::ReturnTransaction,
            transaction_amount: FiservemeaTransactionAmount {
                total: item.amount.clone(),
                currency: item.router_data.request.currency,
            },
        })
    }
}

fn map_refund_status(
    fiservemea_status: Option<FiservemeaPaymentStatus>,
    fiservemea_result: Option<FiservemeaPaymentResult>,
) -> Result<enums::RefundStatus, errors::ConnectorError> {
    match fiservemea_status {
        Some(status) => match status {
            FiservemeaPaymentStatus::Approved => Ok(enums::RefundStatus::Success),
            FiservemeaPaymentStatus::Partial | FiservemeaPaymentStatus::Waiting => {
                Ok(enums::RefundStatus::Pending)
            }
            FiservemeaPaymentStatus::ValidationFailed
            | FiservemeaPaymentStatus::ProcessingFailed
            | FiservemeaPaymentStatus::Declined => Ok(enums::RefundStatus::Failure),
        },
        None => match fiservemea_result {
            Some(result) => match result {
                FiservemeaPaymentResult::Approved => Ok(enums::RefundStatus::Success),
                FiservemeaPaymentResult::Partial | FiservemeaPaymentResult::Waiting => {
                    Ok(enums::RefundStatus::Pending)
                }
                FiservemeaPaymentResult::Declined
                | FiservemeaPaymentResult::Failed
                | FiservemeaPaymentResult::Fraud => Ok(enums::RefundStatus::Failure),
            },
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "transactionResult",
            }),
        },
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, FiservemeaPaymentsResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, FiservemeaPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: map_refund_status(
                    item.response.transaction_status,
                    item.response.transaction_result,
                )?,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FiservemeaPaymentsResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FiservemeaPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: map_refund_status(
                    item.response.transaction_status,
                    item.response.transaction_result,
                )?,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub field: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiservemeaError {
    pub code: Option<String>,
    pub message: Option<String>,
    pub details: Option<Vec<ErrorDetails>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiservemeaErrorResponse {
    #[serde(rename = "type")]
    fiservemea_type: Option<String>,
    client_request_id: Option<String>,
    api_trace_id: Option<String>,
    pub response_type: Option<String>,
    pub error: Option<FiservemeaError>,
}
