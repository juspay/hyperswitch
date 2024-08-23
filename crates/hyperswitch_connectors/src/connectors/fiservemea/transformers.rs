use common_enums::enums;
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

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

#[derive(Debug, Serialize)]
pub enum FiservemeaRequestType {
    PaymentCardSaleTransaction,
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
                    number: req_card.card_number,
                    expiry_date: FiservemeaExpiryDate {
                        month: req_card.card_exp_month,
                        year: req_card.card_exp_year,
                    },
                    security_code: req_card.card_cvc,
                };
                Ok(Self {
                    request_type: FiservemeaRequestType::PaymentCardSaleTransaction,
                    merchant_transaction_id: item
                        .router_data
                        .connector_request_reference_id
                        .clone(),
                    transaction_amount: FiservemeaTransactionAmount {
                        total: item.amount.clone(),
                        currency: item.router_data.request.currency,
                    },
                    payment_method: FiservemeaPaymentMethods::PaymentCard(card),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FiservemeaResponseType {
    TransactionResponse,
}

#[derive(Debug, Serialize, Deserialize)]
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

impl From<FiservemeaPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: FiservemeaPaymentStatus) -> Self {
        match item {
            FiservemeaPaymentStatus::Approved => Self::Charged,
            FiservemeaPaymentStatus::Waiting => Self::Pending,
            FiservemeaPaymentStatus::Partial => Self::PartialCharged,
            FiservemeaPaymentStatus::ValidationFailed
            | FiservemeaPaymentStatus::ProcessingFailed
            | FiservemeaPaymentStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservemeaPaymentsResponse {
    response_type: Option<ResponseType>,
    #[serde(rename = "type")]
    fiservemea_type: Option<FiservemeaResponseType>,
    client_request_id: Option<String>,
    api_trace_id: Option<String>,
    ipg_transaction_id: String,
    order_id: Option<String>,
    transaction_type: Option<FiservemeaTransactionType>,
    transaction_origin: Option<FiservemeaTransactionOrigin>,
    payment_method_details: Option<FiservemeaPaymentMethodDetails>,
    country: Option<Secret<String>>,
    terminal_id: Option<String>,
    merchant_id: Option<String>,
    merchant_transaction_id: Option<String>,
    transaction_time: Option<i64>,
    approved_amount: Option<AmountDetails>,
    transaction_amount: Option<AmountDetails>,
    transaction_status: FiservemeaPaymentStatus, // FiservEMEA Docs mention that this field is deprecated. We are using it for now because transaction_result is not present in the response.
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
            status: common_enums::AttemptStatus::from(item.response.transaction_status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ipg_transaction_id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.merchant_transaction_id,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct FiservemeaRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&FiservemeaRouterData<&RefundsRouterData<F>>> for FiservemeaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FiservemeaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

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
            RefundStatus::Succeeded => Self::Success,
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
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct FiservemeaErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
