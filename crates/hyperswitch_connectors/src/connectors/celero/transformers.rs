use common_enums::{enums, Currency};
use common_utils::{pii::Email, types::MinorUnit};
use hyperswitch_domain_models::{
    address::Address as DomainAddress,
    payment_method_data::PaymentMethodData,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        RouterData,
    },
    router_flow_types::{
        payments::Capture,
        refunds::{Execute, RSync},
    },
    router_request_types::{PaymentsCaptureData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RefundsRequestData, RouterData as _},
};

//TODO: Fill the struct with respective fields
pub struct CeleroRouterData<T> {
    pub amount: MinorUnit, // CeleroCommerce expects integer cents
    pub router_data: T,
}

impl<T> TryFrom<(MinorUnit, T)> for CeleroRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (MinorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
// CeleroCommerce Search Request for sync operations - POST /api/transaction/search
#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroSearchRequest {
    transaction_id: String,
}

impl TryFrom<&PaymentsSyncRouterData> for CeleroSearchRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let transaction_id = match &item.request.connector_transaction_id {
            ResponseId::ConnectorTransactionId(id) => id.clone(),
            ResponseId::EncodedData(id) => id.clone(),
            ResponseId::NoResponseId => {
                return Err(errors::ConnectorError::MissingConnectorTransactionID.into());
            }
        };
        Ok(Self { transaction_id })
    }
}

impl TryFrom<&RefundSyncRouterData> for CeleroSearchRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: item.request.get_connector_refund_id()?,
        })
    }
}

// CeleroCommerce Payment Request according to API specs
#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroPaymentsRequest {
    idempotency_key: String,
    #[serde(rename = "type")]
    transaction_type: TransactionType,
    amount: MinorUnit, // CeleroCommerce expects integer cents
    currency: Currency,
    payment_method: CeleroPaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_address: Option<CeleroAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_address: Option<CeleroAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    create_vault_record: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroAddress {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address_line_1: Option<Secret<String>>,
    address_line_2: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country: Option<common_enums::CountryAlpha2>,
    phone: Option<Secret<String>>,
    email: Option<Email>,
}

impl From<&DomainAddress> for CeleroAddress {
    fn from(address: &DomainAddress) -> Self {
        let address_details = address.address.as_ref();
        Self {
            first_name: address_details.and_then(|f| f.first_name.clone()),
            last_name: address_details.and_then(|l| l.last_name.clone()),
            address_line_1: address_details.and_then(|a| a.line1.clone()),
            address_line_2: address_details.and_then(|a| a.line2.clone()),
            city: address_details.and_then(|a| a.city.clone()),
            state: address_details.and_then(|a| a.state.clone()),
            postal_code: address_details.and_then(|a| a.zip.clone()),
            country: address_details.and_then(|a| a.country.clone()),
            phone: address
                .phone
                .as_ref()
                .and_then(|phone| phone.number.clone()),
            email: address.email.clone(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CeleroPaymentMethod {
    Card(CeleroCard),
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CeleroCard {
    // entry_type: String,
    number: cards::CardNumber,
    expiration_date: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<&PaymentMethodData> for CeleroPaymentMethod {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentMethodData) -> Result<Self, Self::Error> {
        match item {
            PaymentMethodData::Card(req_card) => {
                let card = CeleroCard {
                    // entry_type: "keyed".to_string(),
                    number: req_card.card_number.clone(),
                    expiration_date: Secret::new(format!(
                        "{}/{}",
                        req_card.card_exp_month.peek(),
                        req_card.card_exp_year.peek()
                    )),
                    cvc: req_card.card_cvc.clone(),
                };
                Ok(Self::Card(card))
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::MobilePayment(_) => {
                Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into())
            }
        }
    }
}

impl TryFrom<&CeleroRouterData<&PaymentsAuthorizeRouterData>> for CeleroPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CeleroRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let is_auto_capture = item.router_data.request.is_auto_capture()?;
        let transaction_type = if is_auto_capture {
            TransactionType::Sale
        } else {
            TransactionType::Authorize
        };

        let billing_address: Option<CeleroAddress> =
            item.router_data.get_optional_shipping().map(|e| e.into());

        let shipping_address: Option<CeleroAddress> =
            item.router_data.get_optional_shipping().map(|e| e.into());

        let request: CeleroPaymentsRequest = Self {
            idempotency_key: item.router_data.connector_request_reference_id.clone(),
            transaction_type,
            amount: item.amount,
            currency: item.router_data.request.currency,
            payment_method: CeleroPaymentMethod::try_from(
                &item.router_data.request.payment_method_data,
            )?,
            billing_address,
            shipping_address,
            create_vault_record: Some(false),
        };

        Ok(request)
    }
}

// Auth Struct for CeleroCommerce API key authentication
pub struct CeleroAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CeleroAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// CeleroCommerce API Response Structures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CeleroResponseStatus {
    #[serde(alias = "success", alias = "Success", alias = "SUCCESS")]
    Success,
    #[serde(alias = "error", alias = "Error", alias = "ERROR")]
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CeleroTransactionStatus {
    #[serde(alias = "approved", alias = "Approved", alias = "APPROVED")]
    Approved,
    #[serde(alias = "declined", alias = "Declined", alias = "DECLINED")]
    Declined,
    #[serde(alias = "error", alias = "Error", alias = "ERROR")]
    Error,
    #[serde(alias = "pending", alias = "Pending", alias = "PENDING")]
    Pending,
    #[serde(alias = "settled", alias = "Settled", alias = "SETTLED")]
    Settled,
    #[serde(alias = "voided", alias = "Voided", alias = "VOIDED")]
    Voided,
}

impl From<CeleroTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: CeleroTransactionStatus) -> Self {
        match item {
            CeleroTransactionStatus::Approved => Self::Authorized,
            CeleroTransactionStatus::Settled => Self::Charged,
            CeleroTransactionStatus::Declined => Self::Failure,
            CeleroTransactionStatus::Error => Self::Failure,
            CeleroTransactionStatus::Pending => Self::Pending,
            CeleroTransactionStatus::Voided => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroCardResponse {
    pub status: CeleroTransactionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor_response_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avs_response_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CeleroPaymentMethodResponse {
    Card(CeleroCardResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Sale,
    Authorize,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CeleroTransactionData {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub currency: String,
    pub response: CeleroPaymentMethodResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<CeleroAddressResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<CeleroAddressResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CeleroAddressResponse {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address_line_1: Option<Secret<String>>,
    address_line_2: Option<Secret<String>>,
    city: Option<String>,
    state: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country: Option<common_enums::CountryAlpha2>,
    phone: Option<Secret<String>>,
    email: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CeleroPaymentsResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<CeleroTransactionData>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CeleroPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CeleroPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => {
                if let Some(data) = item.response.data {
                    let response = match &data.response {
                        CeleroPaymentMethodResponse::Card(card) => card,
                    };
                    // Check if transaction itself failed despite successful API call
                    match response.status {
                        CeleroTransactionStatus::Declined | CeleroTransactionStatus::Error => {
                            // Transaction failed - create error response with transaction details
                            let error_details = CeleroErrorDetails::from_transaction_response(
                                response,
                                item.response.msg,
                            );

                            Ok(Self {
                                status: common_enums::AttemptStatus::Failure,
                                response: Err(
                                    hyperswitch_domain_models::router_data::ErrorResponse {
                                        code: error_details
                                            .error_code
                                            .unwrap_or_else(|| "TRANSACTION_FAILED".to_string()),
                                        message: error_details.error_message,
                                        reason: error_details.decline_reason,
                                        status_code: item.http_code,
                                        attempt_status: None,
                                        connector_transaction_id: Some(data.id),
                                        network_decline_code: None,
                                        network_advice_code: None,
                                        network_error_message: None,
                                    },
                                ),
                                ..item.data
                            })
                        }
                        _ => {
                            let connector_response_data =
                                convert_to_additional_payment_method_connector_response(
                                    response.avs_response_code.clone(),
                                )
                                .map(ConnectorResponseData::with_additional_payment_method_data);
                            let final_status: enums::AttemptStatus = response.status.into();
                            Ok(Self {
                                status: final_status,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(data.id),
                                    redirection_data: Box::new(None),
                                    mandate_reference: Box::new(None),
                                    connector_metadata: None,
                                    network_txn_id: None,
                                    connector_response_reference_id: response.auth_code.clone(),
                                    incremental_authorization_allowed: None,
                                    charges: None,
                                }),
                                connector_response: connector_response_data,
                                ..item.data
                            })
                        }
                    }
                } else {
                    // No transaction data in successful response
                    Ok(Self {
                        status: common_enums::AttemptStatus::Failure,
                        response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                            code: "MISSING_DATA".to_string(),
                            message: "No transaction data in response".to_string(),
                            reason: Some(item.response.msg),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: None,
                            network_decline_code: None,
                            network_advice_code: None,
                            network_error_message: None,
                        }),
                        ..item.data
                    })
                }
            }
            CeleroResponseStatus::Error => {
                // Top-level API error
                let error_details =
                    CeleroErrorDetails::from_top_level_error(item.response.msg.clone());

                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                        code: error_details
                            .error_code
                            .unwrap_or_else(|| "API_ERROR".to_string()),
                        message: error_details.error_message,
                        reason: error_details.decline_reason,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        network_decline_code: None,
                        network_advice_code: None,
                        network_error_message: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

// CAPTURE:
// Type definition for CaptureRequest
#[derive(Default, Debug, Serialize)]
pub struct CeleroCaptureRequest {
    pub amount: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

impl TryFrom<&CeleroRouterData<&PaymentsCaptureRouterData>> for CeleroCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CeleroRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            order_id: Some(item.router_data.payment_id.clone()),
        })
    }
}

// CeleroCommerce Capture Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroCaptureResponse {
    pub status: CeleroResponseStatus,
    pub msg: Option<String>,
    pub data: Option<serde_json::Value>, // Usually null for capture responses
}

impl
    TryFrom<
        ResponseRouterData<
            Capture,
            CeleroCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Capture,
            CeleroCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                status: common_enums::AttemptStatus::Charged,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "CAPTURE_FAILED".to_string(),
                    message: item
                        .response
                        .msg
                        .clone()
                        .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                    reason: None,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }),
                ..item.data
            }),
        }
    }
}

// VOID:
// Type definition for VoidRequest
#[derive(Default, Debug, Serialize)]
pub struct CeleroVoidRequest {
    // Based on API documentation, void request appears to be a simple POST without body
    // However, following the existing pattern for consistency
}

impl
    TryFrom<
        &CeleroRouterData<
            &RouterData<
                hyperswitch_domain_models::router_flow_types::payments::Void,
                hyperswitch_domain_models::router_request_types::PaymentsCancelData,
                PaymentsResponseData,
            >,
        >,
    > for CeleroVoidRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &CeleroRouterData<
            &RouterData<
                hyperswitch_domain_models::router_flow_types::payments::Void,
                hyperswitch_domain_models::router_request_types::PaymentsCancelData,
                PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            // Void request appears to be empty based on API documentation
        })
    }
}

// CeleroCommerce Void Response - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroVoidResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for void responses
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            CeleroVoidResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    >
    for RouterData<
        hyperswitch_domain_models::router_flow_types::payments::Void,
        hyperswitch_domain_models::router_request_types::PaymentsCancelData,
        PaymentsResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            CeleroVoidResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                status: common_enums::AttemptStatus::Voided,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "VOID_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }),
                ..item.data
            }),
        }
    }
}
#[derive(Default, Debug, Serialize)]
pub struct CeleroRefundRequest {
    pub amount: MinorUnit,
    pub surcharge: MinorUnit, // Required field as per API specification
}

impl<F> TryFrom<&CeleroRouterData<&RefundsRouterData<F>>> for CeleroRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CeleroRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            surcharge: MinorUnit::zero(), // Default to 0 as per API specification
        })
    }
}

// CeleroCommerce Refund Response - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroRefundResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for refund responses
}

impl TryFrom<RefundsResponseRouterData<Execute, CeleroRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CeleroRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "REFUND_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }),
                ..item.data
            }),
        }
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, CeleroRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CeleroRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CeleroResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CeleroResponseStatus::Error => Ok(Self {
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "REFUND_SYNC_FAILED".to_string(),
                    message: item.response.msg.clone(),
                    reason: Some(item.response.msg),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(
                        item.data.request.connector_transaction_id.clone(),
                    ),
                    network_decline_code: None,
                    network_advice_code: None,
                    network_error_message: None,
                }),
                ..item.data
            }),
        }
    }
}

// CeleroCommerce Error Response Structures

// Main error response structure - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroErrorResponse {
    pub status: CeleroResponseStatus,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// Error details that can be extracted from various response fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CeleroErrorDetails {
    pub error_code: Option<String>,
    pub error_message: String,
    pub processor_response_code: Option<String>,
    pub decline_reason: Option<String>,
}

impl From<CeleroErrorResponse> for CeleroErrorDetails {
    fn from(error_response: CeleroErrorResponse) -> Self {
        Self {
            error_code: Some("API_ERROR".to_string()),
            error_message: error_response.msg,
            processor_response_code: None,
            decline_reason: None,
        }
    }
}

// Function to extract error details from transaction response data
impl CeleroErrorDetails {
    pub fn from_transaction_response(response: &CeleroCardResponse, msg: String) -> Self {
        // Map specific error codes based on common response patterns
        let (error_code, decline_reason) =
            Self::map_processor_error(&response.processor_response_code, &msg);

        Self {
            error_code,
            error_message: msg,
            processor_response_code: response.processor_response_code.clone(),
            decline_reason,
        }
    }

    pub fn from_top_level_error(msg: String) -> Self {
        // Map specific error codes from top-level API errors
        let (error_code, decline_reason) = Self::map_api_error(&msg);

        Self {
            error_code,
            error_message: msg,
            processor_response_code: None,
            decline_reason,
        }
    }

    /// Map processor response codes and messages to specific Hyperswitch error codes
    fn map_processor_error(
        processor_code: &Option<String>,
        message: &str,
    ) -> (Option<String>, Option<String>) {
        let message_lower = message.to_lowercase();

        // Check message content for specific error patterns first
        if message_lower.contains("invalid card number") || message_lower.contains("invalid card") {
            return (
                Some("INVALID_CARD_DATA".to_string()),
                Some("Invalid card number".to_string()),
            );
        }

        if message_lower.contains("insufficient funds")
            || message_lower.contains("insufficient balance")
        {
            return (
                Some("INSUFFICIENT_FUNDS".to_string()),
                Some("Insufficient funds".to_string()),
            );
        }

        if message_lower.contains("expired card") || message_lower.contains("card expired") {
            return (
                Some("EXPIRED_CARD".to_string()),
                Some("Card expired".to_string()),
            );
        }

        if message_lower.contains("cvv")
            || message_lower.contains("cvc")
            || message_lower.contains("security code")
        {
            return (
                Some("INCORRECT_CVC".to_string()),
                Some("CVV mismatch".to_string()),
            );
        }

        if message_lower.contains("declined") || message_lower.contains("decline") {
            return (
                Some("TRANSACTION_DECLINED".to_string()),
                Some("Transaction declined by issuer".to_string()),
            );
        }

        // Check processor response codes if available
        if let Some(code) = processor_code {
            match code.as_str() {
                "05" => (
                    Some("TRANSACTION_DECLINED".to_string()),
                    Some("Do not honor".to_string()),
                ),
                "14" => (
                    Some("INVALID_CARD_DATA".to_string()),
                    Some("Invalid card number".to_string()),
                ),
                "51" => (
                    Some("INSUFFICIENT_FUNDS".to_string()),
                    Some("Insufficient funds".to_string()),
                ),
                "54" => (
                    Some("EXPIRED_CARD".to_string()),
                    Some("Expired card".to_string()),
                ),
                "55" => (
                    Some("INCORRECT_CVC".to_string()),
                    Some("Incorrect PIN".to_string()),
                ),
                "61" => (
                    Some("TRANSACTION_DECLINED".to_string()),
                    Some("Exceeds withdrawal amount limit".to_string()),
                ),
                "62" => (
                    Some("TRANSACTION_DECLINED".to_string()),
                    Some("Restricted card".to_string()),
                ),
                "65" => (
                    Some("TRANSACTION_DECLINED".to_string()),
                    Some("Exceeds withdrawal frequency limit".to_string()),
                ),
                "78" => (
                    Some("INVALID_CARD_DATA".to_string()),
                    Some("Invalid/nonexistent account".to_string()),
                ),
                "91" => (
                    Some("PROCESSING_ERROR".to_string()),
                    Some("Issuer or switch inoperative".to_string()),
                ),
                "96" => (
                    Some("PROCESSING_ERROR".to_string()),
                    Some("System malfunction".to_string()),
                ),
                _ => (
                    Some("TRANSACTION_FAILED".to_string()),
                    Some("Transaction failed".to_string()),
                ),
            }
        } else {
            (
                Some("TRANSACTION_FAILED".to_string()),
                Some("Transaction failed".to_string()),
            )
        }
    }

    /// Map top-level API errors to specific error codes
    fn map_api_error(message: &str) -> (Option<String>, Option<String>) {
        let message_lower = message.to_lowercase();

        if message_lower.contains("authentication") || message_lower.contains("unauthorized") {
            (
                Some("AUTHENTICATION_FAILED".to_string()),
                Some("Authentication failed".to_string()),
            )
        } else if message_lower.contains("invalid request") || message_lower.contains("bad request")
        {
            (
                Some("INVALID_REQUEST".to_string()),
                Some("Invalid request format".to_string()),
            )
        } else if message_lower.contains("timeout") {
            (
                Some("REQUEST_TIMEOUT".to_string()),
                Some("Request timeout".to_string()),
            )
        } else if message_lower.contains("rate limit")
            || message_lower.contains("too many requests")
        {
            (
                Some("RATE_LIMITED".to_string()),
                Some("Rate limit exceeded".to_string()),
            )
        } else if message_lower.contains("service unavailable")
            || message_lower.contains("server error")
        {
            (
                Some("SERVICE_UNAVAILABLE".to_string()),
                Some("Service temporarily unavailable".to_string()),
            )
        } else {
            (Some("API_ERROR".to_string()), None)
        }
    }
}

pub fn get_avs_definition(code: &str) -> Option<&'static str> {
    match code {
        "0" => Some("AVS Not Available"),
        "A" => Some("Address match only"),
        "B" => Some("Address matches, ZIP not verified"),
        "C" => Some("Incompatible format"),
        "D" => Some("Exact match"),
        "F" => Some("Exact match, UK-issued cards"),
        "G" => Some("Non-U.S. Issuer does not participate"),
        "I" => Some("Not verified"),
        "M" => Some("Exact match"),
        "N" => Some("No address or ZIP match"),
        "P" => Some("Postal Code match"),
        "R" => Some("Issuer system unavailable"),
        "S" => Some("Service not supported"),
        "U" => Some("Address unavailable"),
        "W" => Some("9-character numeric ZIP match only"),
        "X" => Some("Exact match, 9-character numeric ZIP"),
        "Y" => Some("Exact match, 5-character numeric ZIP"),
        "Z" => Some("5-character ZIP match only"),
        "L" => Some("Partial match, Name and billing postal code match"),
        "1" => Some("Cardholder name and ZIP match"),
        "2" => Some("Cardholder name, address and ZIP match"),
        "3" => Some("Cardholder name and address match"),
        "4" => Some("Cardholder name matches"),
        "5" => Some("Cardholder name incorrect, ZIP matches"),
        "6" => Some("Cardholder name incorrect, address and zip match"),
        "7" => Some("Cardholder name incorrect, address matches"),
        "8" => Some("Cardholder name, address, and ZIP do not match"),
        _ => None, // No definition found for the given code
    }
}
fn convert_to_additional_payment_method_connector_response(
    response_code: Option<String>,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    match response_code {
        None => None,
        Some(code) => {
            let description = get_avs_definition(&code);
            let payment_checks = serde_json::json!({
                "avs_result_code": code,
                "description": description
            });
            Some(AdditionalPaymentMethodConnectorResponse::Card {
                authentication_data: None,
                payment_checks: Some(payment_checks),
                card_network: None,
                domestic_network: None,
            })
        }
    }
}
