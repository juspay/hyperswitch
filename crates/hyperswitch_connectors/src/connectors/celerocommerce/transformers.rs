use common_enums::enums;
use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
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
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, RefundsRequestData},
};

//TODO: Fill the struct with respective fields
pub struct CelerocommerceRouterData<T> {
    pub amount: i64, // CeleroCommerce expects integer cents
    pub router_data: T,
}

impl<T> TryFrom<(StringMinorUnit, T)> for CelerocommerceRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (StringMinorUnit, T)) -> Result<Self, Self::Error> {
        // Convert StringMinorUnit to i64
        let amount_str =
            serde_json::to_string(&amount).change_context(errors::ConnectorError::ParsingFailed)?;
        // Remove quotes from serialized string
        let amount_str = amount_str.trim_matches('"');
        let amount_i64 = amount_str
            .parse::<i64>()
            .change_context(errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            amount: amount_i64,
            router_data: item,
        })
    }
}

// CeleroCommerce Search Request for sync operations - POST /api/transaction/search
#[derive(Debug, Serialize, PartialEq)]
pub struct CelerocommerceSearchRequest {
    transaction_id: String,
}

impl TryFrom<&PaymentsSyncRouterData> for CelerocommerceSearchRequest {
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

impl TryFrom<&RefundSyncRouterData> for CelerocommerceSearchRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: item.request.get_connector_refund_id()?,
        })
    }
}

// CeleroCommerce Payment Request according to API specs
#[derive(Debug, Serialize, PartialEq)]
pub struct CelerocommercePaymentsRequest {
    idempotency_key: String,
    #[serde(rename = "type")]
    transaction_type: String,
    amount: i64, // CeleroCommerce expects integer cents
    currency: String,
    payment_method: CelerocommercePaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    create_vault_record: Option<bool>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CelerocommercePaymentMethod {
    card: CelerocommerceCard,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CelerocommerceCard {
    entry_type: String,
    number: cards::CardNumber,
    expiration_date: Secret<String>,
    cvc: Secret<String>,
}

impl TryFrom<&CelerocommerceRouterData<&PaymentsAuthorizeRouterData>>
    for CelerocommercePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CelerocommerceRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match &item.router_data.request.payment_method_data {
            PaymentMethodData::Card(req_card) => {
                let card = CelerocommerceCard {
                    entry_type: "keyed".to_string(),
                    number: req_card.card_number.clone(),
                    expiration_date: Secret::new(format!(
                        "{}/{}",
                        req_card.card_exp_month.peek(),
                        req_card.card_exp_year.peek()
                    )),
                    cvc: req_card.card_cvc.clone(),
                };

                let is_auto_capture = item.router_data.request.is_auto_capture()?;
                let transaction_type = if is_auto_capture {
                    "sale".to_string()
                } else {
                    "authorize".to_string()
                };

                let request = Self {
                    idempotency_key: item.router_data.connector_request_reference_id.clone(),
                    transaction_type,
                    amount: item.amount, // Already converted to i64 in CelerocommerceRouterData
                    currency: item.router_data.request.currency.to_string(),
                    payment_method: CelerocommercePaymentMethod { card },
                    create_vault_record: Some(false), // Set based on setup_future_usage if needed
                };

                Ok(request)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct for CeleroCommerce API key authentication
pub struct CelerocommerceAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CelerocommerceAuthType {
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
pub enum CelerocommerceResponseStatus {
    #[serde(alias = "success", alias = "Success", alias = "SUCCESS")]
    Success,
    #[serde(alias = "error", alias = "Error", alias = "ERROR")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CelerocommerceTransactionStatus {
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

impl From<CelerocommerceTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: CelerocommerceTransactionStatus) -> Self {
        match item {
            CelerocommerceTransactionStatus::Approved => Self::Authorized,
            CelerocommerceTransactionStatus::Settled => Self::Charged,
            CelerocommerceTransactionStatus::Declined => Self::Failure,
            CelerocommerceTransactionStatus::Error => Self::Failure,
            CelerocommerceTransactionStatus::Pending => Self::Pending,
            CelerocommerceTransactionStatus::Voided => Self::Voided,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceTransactionResponse {
    pub status: CelerocommerceTransactionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor_response_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceTransactionData {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: i64,
    pub currency: String,
    pub response: CelerocommerceTransactionResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommercePaymentsResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    pub data: Option<CelerocommerceTransactionData>,
}

// Separate response struct for search API that returns an array of transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceSearchResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    pub data: Option<Vec<CelerocommerceTransactionData>>,
    pub total_count: Option<i32>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CelerocommerceSearchResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CelerocommerceSearchResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => {
                if let Some(transactions) = item.response.data {
                    if let Some(data) = transactions.first() {
                        // Found transaction - process it like a regular payment response
                        match data.response.status {
                            CelerocommerceTransactionStatus::Declined
                            | CelerocommerceTransactionStatus::Error => {
                                // Transaction failed
                                let error_details =
                                    CelerocommerceErrorDetails::from_transaction_response(
                                        &data.response,
                                        item.response.msg,
                                    );

                                Ok(Self {
                                    status: common_enums::AttemptStatus::Failure,
                                    response: Err(
                                        hyperswitch_domain_models::router_data::ErrorResponse {
                                            code: error_details.error_code.unwrap_or_else(|| {
                                                "TRANSACTION_FAILED".to_string()
                                            }),
                                            message: error_details.error_message,
                                            reason: error_details.decline_reason,
                                            status_code: item.http_code,
                                            attempt_status: None,
                                            connector_transaction_id: Some(data.id.clone()),
                                            network_decline_code: error_details
                                                .processor_response_code
                                                .clone(),
                                            network_advice_code: None,
                                            network_error_message: error_details
                                                .processor_response_code,
                                        },
                                    ),
                                    ..item.data
                                })
                            }
                            _ => {
                                // Transaction successful - determine final status based on transaction type
                                let transaction_type_lower = data.transaction_type.to_lowercase();
                                let final_status = match transaction_type_lower.as_str() {
                                    "authorize" => match data.response.status {
                                        CelerocommerceTransactionStatus::Approved => {
                                            common_enums::AttemptStatus::Authorized
                                        }
                                        CelerocommerceTransactionStatus::Pending => {
                                            common_enums::AttemptStatus::Pending
                                        }
                                        CelerocommerceTransactionStatus::Settled => {
                                            common_enums::AttemptStatus::Authorized
                                        }
                                        CelerocommerceTransactionStatus::Voided => {
                                            common_enums::AttemptStatus::Voided
                                        }
                                        _ => common_enums::AttemptStatus::Failure,
                                    },
                                    "sale" => match data.response.status {
                                        CelerocommerceTransactionStatus::Approved
                                        | CelerocommerceTransactionStatus::Settled => {
                                            common_enums::AttemptStatus::Charged
                                        }
                                        CelerocommerceTransactionStatus::Pending => {
                                            common_enums::AttemptStatus::Pending
                                        }
                                        CelerocommerceTransactionStatus::Voided => {
                                            common_enums::AttemptStatus::Voided
                                        }
                                        _ => common_enums::AttemptStatus::Failure,
                                    },
                                    _ => common_enums::AttemptStatus::from(
                                        data.response.status.clone(),
                                    ),
                                };

                                Ok(Self {
                                    status: final_status,
                                    response: Ok(PaymentsResponseData::TransactionResponse {
                                        resource_id: ResponseId::ConnectorTransactionId(
                                            data.id.clone(),
                                        ),
                                        redirection_data: Box::new(None),
                                        mandate_reference: Box::new(None),
                                        connector_metadata: None,
                                        network_txn_id: None,
                                        connector_response_reference_id: data
                                            .response
                                            .auth_code
                                            .clone(),
                                        incremental_authorization_allowed: None,
                                        charges: None,
                                    }),
                                    ..item.data
                                })
                            }
                        }
                    } else {
                        // Empty transactions array
                        Ok(Self {
                            status: common_enums::AttemptStatus::Failure,
                            response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                                code: "TRANSACTION_NOT_FOUND".to_string(),
                                message: "Transaction not found".to_string(),
                                reason: Some(
                                    "No matching transaction found in search results".to_string(),
                                ),
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
            CelerocommerceResponseStatus::Error => {
                // Top-level API error
                let error_details =
                    CelerocommerceErrorDetails::from_top_level_error(item.response.msg.clone());

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
                        network_decline_code: error_details.processor_response_code.clone(),
                        network_advice_code: None,
                        network_error_message: error_details.processor_response_code,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, CelerocommercePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CelerocommercePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => {
                if let Some(data) = item.response.data {
                    // Check if transaction itself failed despite successful API call
                    match data.response.status {
                        CelerocommerceTransactionStatus::Declined
                        | CelerocommerceTransactionStatus::Error => {
                            // Transaction failed - create error response with transaction details
                            let error_details =
                                CelerocommerceErrorDetails::from_transaction_response(
                                    &data.response,
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
                                        network_decline_code: error_details
                                            .processor_response_code
                                            .clone(),
                                        network_advice_code: None,
                                        network_error_message: error_details
                                            .processor_response_code,
                                    },
                                ),
                                ..item.data
                            })
                        }
                        _ => {
                            // Transaction successful - determine final status based on transaction type
                            // Use case-insensitive matching for transaction type to handle variations
                            let transaction_type_lower = data.transaction_type.to_lowercase();
                            let final_status = match transaction_type_lower.as_str() {
                                "authorize" => {
                                    match data.response.status {
                                        CelerocommerceTransactionStatus::Approved => {
                                            common_enums::AttemptStatus::Authorized
                                        }
                                        CelerocommerceTransactionStatus::Pending => {
                                            common_enums::AttemptStatus::Pending
                                        }
                                        CelerocommerceTransactionStatus::Settled => {
                                            common_enums::AttemptStatus::Authorized
                                            // Authorize that got settled
                                        }
                                        CelerocommerceTransactionStatus::Voided => {
                                            common_enums::AttemptStatus::Voided
                                        }
                                        _ => common_enums::AttemptStatus::Failure,
                                    }
                                }
                                "sale" => match data.response.status {
                                    CelerocommerceTransactionStatus::Approved
                                    | CelerocommerceTransactionStatus::Settled => {
                                        common_enums::AttemptStatus::Charged
                                    }
                                    CelerocommerceTransactionStatus::Pending => {
                                        common_enums::AttemptStatus::Pending
                                    }
                                    CelerocommerceTransactionStatus::Voided => {
                                        common_enums::AttemptStatus::Voided
                                    }
                                    _ => common_enums::AttemptStatus::Failure,
                                },
                                _ => {
                                    // For unknown transaction types, use the status from enum mapping as fallback
                                    common_enums::AttemptStatus::from(data.response.status.clone())
                                }
                            };

                            Ok(Self {
                                status: final_status,
                                response: Ok(PaymentsResponseData::TransactionResponse {
                                    resource_id: ResponseId::ConnectorTransactionId(data.id),
                                    redirection_data: Box::new(None),
                                    mandate_reference: Box::new(None),
                                    connector_metadata: None,
                                    network_txn_id: None,
                                    connector_response_reference_id: data.response.auth_code,
                                    incremental_authorization_allowed: None,
                                    charges: None,
                                }),
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
            CelerocommerceResponseStatus::Error => {
                // Top-level API error
                let error_details =
                    CelerocommerceErrorDetails::from_top_level_error(item.response.msg.clone());

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
                        network_decline_code: error_details.processor_response_code.clone(),
                        network_advice_code: None,
                        network_error_message: error_details.processor_response_code,
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
pub struct CelerocommerceCaptureRequest {
    pub amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_amount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub po_number: Option<String>,
}

impl TryFrom<&CelerocommerceRouterData<&PaymentsCaptureRouterData>>
    for CelerocommerceCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CelerocommerceRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            tax_amount: Some(0),      // Default to 0 as per API specs
            shipping_amount: Some(0), // Default to 0 as per API specs
            order_id: Some(item.router_data.payment_id.clone()),
            po_number: None, // Optional field
        })
    }
}

// CeleroCommerce Capture Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceCaptureResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for capture responses
}

impl
    TryFrom<
        ResponseRouterData<
            Capture,
            CelerocommerceCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Capture,
            CelerocommerceCaptureResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => Ok(Self {
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
            CelerocommerceResponseStatus::Error => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(hyperswitch_domain_models::router_data::ErrorResponse {
                    code: "CAPTURE_FAILED".to_string(),
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

// VOID:
// Type definition for VoidRequest
#[derive(Default, Debug, Serialize)]
pub struct CelerocommerceVoidRequest {
    // Based on API documentation, void request appears to be a simple POST without body
    // However, following the existing pattern for consistency
}

impl
    TryFrom<
        &CelerocommerceRouterData<
            &RouterData<
                hyperswitch_domain_models::router_flow_types::payments::Void,
                hyperswitch_domain_models::router_request_types::PaymentsCancelData,
                PaymentsResponseData,
            >,
        >,
    > for CelerocommerceVoidRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        _item: &CelerocommerceRouterData<
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
pub struct CelerocommerceVoidResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for void responses
}

impl
    TryFrom<
        ResponseRouterData<
            hyperswitch_domain_models::router_flow_types::payments::Void,
            CelerocommerceVoidResponse,
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
            CelerocommerceVoidResponse,
            hyperswitch_domain_models::router_request_types::PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => Ok(Self {
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
            CelerocommerceResponseStatus::Error => Ok(Self {
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct CelerocommerceRefundRequest {
    pub amount: i64,
    pub surcharge: i64, // Required field as per API specification
}

impl<F> TryFrom<&CelerocommerceRouterData<&RefundsRouterData<F>>> for CelerocommerceRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CelerocommerceRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            surcharge: 0, // Default to 0 as per API specification
        })
    }
}

// CeleroCommerce Refund Response - matches API spec format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceRefundResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    pub data: Option<serde_json::Value>, // Usually null for refund responses
}

impl TryFrom<RefundsResponseRouterData<Execute, CelerocommerceRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CelerocommerceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CelerocommerceResponseStatus::Error => Ok(Self {
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

impl TryFrom<RefundsResponseRouterData<RSync, CelerocommerceRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CelerocommerceRefundResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response.status {
            CelerocommerceResponseStatus::Success => Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: enums::RefundStatus::Success,
                }),
                ..item.data
            }),
            CelerocommerceResponseStatus::Error => Ok(Self {
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
pub struct CelerocommerceErrorResponse {
    pub status: CelerocommerceResponseStatus,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// Error details that can be extracted from various response fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CelerocommerceErrorDetails {
    pub error_code: Option<String>,
    pub error_message: String,
    pub processor_response_code: Option<String>,
    pub decline_reason: Option<String>,
}

impl From<CelerocommerceErrorResponse> for CelerocommerceErrorDetails {
    fn from(error_response: CelerocommerceErrorResponse) -> Self {
        Self {
            error_code: Some("API_ERROR".to_string()),
            error_message: error_response.msg,
            processor_response_code: None,
            decline_reason: None,
        }
    }
}

// Function to extract error details from transaction response data
impl CelerocommerceErrorDetails {
    pub fn from_transaction_response(
        response: &CelerocommerceTransactionResponse,
        msg: String,
    ) -> Self {
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
