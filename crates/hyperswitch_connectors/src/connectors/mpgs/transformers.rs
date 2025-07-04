use common_enums::enums;
use common_utils::types::StringMinorUnit;
use api_models::enums::CaptureMethod;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData, RedirectForm},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils,
};

pub struct MpgsRouterData<T> {
    pub amount: String, // MPGS expects amount as string with 2 decimal places
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for MpgsRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount: amount.to_string(),
            router_data: item,
        }
    }
}

// MPGS Payment Request structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsPaymentsRequest {
    pub api_operation: MpgsApiOperation,
    pub order: MpgsOrder,
    pub source_of_funds: MpgsSourceOfFunds,
    pub transaction: MpgsTransaction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<MpgsCustomer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disbursement_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MpgsApiOperation {
    Pay,
    Authorize,
    Capture,
    Void,
    Refund,
    Verify,
    Disbursement,
}

#[derive(Debug, Serialize)]
pub struct MpgsOrder {
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsSourceOfFunds {
    #[serde(rename = "type")]
    pub source_type: String,
    pub provided: MpgsProvidedData,
}

#[derive(Debug, Serialize)]
pub struct MpgsProvidedData {
    pub card: MpgsCard,
}

#[derive(Debug, Serialize)]
pub struct MpgsCard {
    pub number: cards::CardNumber,
    pub expiry: MpgsExpiry,
    #[serde(rename = "securityCode")]
    pub security_code: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct MpgsExpiry {
    pub month: Secret<String>,
    pub year: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsTransaction {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_transaction_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MpgsCustomer {
    pub email: common_utils::pii::Email,
}


// Capture Request Structure
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsCaptureRequest {
    api_operation: MpgsApiOperation,
    transaction: MpgsCaptureTransaction,
}

#[derive(Debug, Serialize)]
pub struct MpgsCaptureTransaction {
    amount: String,
    currency: String,
}

impl TryFrom<&MpgsRouterData<&PaymentsAuthorizeRouterData>> for MpgsPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card) => {
                let amount = utils::to_currency_base_unit(
                    item.router_data.request.amount,
                    item.router_data.request.currency,
                )?;
                
                let formatted_amount = format!("{:.2}", amount);
                
                let api_operation = match item.router_data.request.capture_method {
                    Some(CaptureMethod::Automatic) | None => MpgsApiOperation::Pay,
                    Some(CaptureMethod::Manual) => MpgsApiOperation::Authorize,
                    _ => MpgsApiOperation::Pay,
                };
                
                Ok(Self {
                    api_operation,
                    order: MpgsOrder {
                        amount: formatted_amount,
                        currency: item.router_data.request.currency.to_string(),
                    },
                    source_of_funds: MpgsSourceOfFunds {
                        source_type: "CARD".to_string(),
                        provided: MpgsProvidedData {
                            card: MpgsCard {
                                number: card.card_number,
                                expiry: MpgsExpiry {
                                    month: card.card_exp_month,
                                    year: card.card_exp_year,
                                },
                                security_code: card.card_cvc,
                            },
                        },
                    },
                    transaction: MpgsTransaction {
                        source: "INTERNET".to_string(),
                        target_transaction_id: None,
                    },
                    customer: item.router_data.request.email.as_ref().map(|email| {
                        MpgsCustomer {
                            email: email.clone(),
                        }
                    }),
                    disbursement_type: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<&MpgsRouterData<&PaymentsCaptureRouterData>> for MpgsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_base_unit(
            item.router_data.request.amount_to_capture,
            item.router_data.request.currency,
        )?;
        
        let formatted_amount = format!("{:.2}", amount);
        
        Ok(Self {
            api_operation: MpgsApiOperation::Capture,
            transaction: MpgsCaptureTransaction {
                amount: formatted_amount,
                currency: item.router_data.request.currency.to_string(),
            },
        })
    }
}

// Auth Struct
pub struct MpgsAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_id: Secret<String>,
    pub(super) region: String,
}

impl TryFrom<&ConnectorAuthType> for MpgsAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::HeaderKey { api_key } = auth_type {
            let exposed_api_key = api_key.clone().expose();
            let auth_fields: Vec<&str> = exposed_api_key.split(':').collect();
            if auth_fields.len() == 3 {
                Ok(Self {
                    api_key: Secret::new(auth_fields[1].to_string()),
                    merchant_id: Secret::new(auth_fields[0].to_string()),
                    region: auth_fields[2].to_string(),
                })
            } else {
                Err(errors::ConnectorError::InvalidConnectorConfig {
                    config: "Invalid authentication format. Expected: merchant_id:api_password:region",
                }
                .into())
            }
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

// MPGS Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpgsPaymentsResponse {
    pub result: String,
    pub merchant: Option<String>,
    pub order: MpgsOrderResponse,
    pub transaction: MpgsTransactionResponse,
    pub response: MpgsGatewayResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<MpgsAuthenticationResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsOrderResponse {
    pub amount: f64,
    pub currency: String,
    pub id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_authorized_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_captured_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_disbursed_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_refunded_amount: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsTransactionResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquirer_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsGatewayResponse {
    pub gateway_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquirer_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquirer_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsAuthenticationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, MpgsPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, MpgsPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = match (item.response.result.as_str(), item.response.response.gateway_code.as_str()) {
            ("SUCCESS", "APPROVED") => {
                match item.response.transaction.transaction_type.as_str() {
                    "AUTHORIZATION" => common_enums::AttemptStatus::Authorized,
                    "PAYMENT" | "CAPTURE" => common_enums::AttemptStatus::Charged,
                    "VOID_AUTHORIZATION" => common_enums::AttemptStatus::Voided,
                    _ => common_enums::AttemptStatus::Pending,
                }
            },
            ("PENDING", _) => common_enums::AttemptStatus::Pending,
            (_, "AUTHENTICATION_IN_PROGRESS") => common_enums::AttemptStatus::AuthenticationPending,
            _ => common_enums::AttemptStatus::Failure,
        };

        let redirection_data = item.response.authentication
            .and_then(|auth| auth.redirect_url)
            .map(|url| {
                RedirectForm::Form {
                    endpoint: url,
                    method: common_utils::request::Method::Get,
                    form_fields: std::collections::HashMap::new(),
                }
            });

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.transaction.acquirer_reference.clone(),
                connector_response_reference_id: Some(item.response.order.id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsRefundRequest {
    api_operation: MpgsApiOperation,
    transaction: MpgsRefundTransaction,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpgsRefundTransaction {
    amount: String,
    currency: String,
}

impl<F> TryFrom<&MpgsRouterData<&RefundsRouterData<F>>> for MpgsRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &MpgsRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = utils::to_currency_base_unit(
            item.router_data.request.refund_amount,
            item.router_data.request.currency,
        )?;
        
        let formatted_amount = format!("{:.2}", amount);
        
        Ok(Self {
            api_operation: MpgsApiOperation::Refund,
            transaction: MpgsRefundTransaction {
                amount: formatted_amount,
                currency: item.router_data.request.currency.to_string(),
            },
        })
    }
}

// Type definition for Refund Response
pub type MpgsRefundResponse = MpgsPaymentsResponse;

impl TryFrom<RefundsResponseRouterData<Execute, MpgsRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, MpgsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match (item.response.result.as_str(), item.response.response.gateway_code.as_str()) {
            ("SUCCESS", "APPROVED") => enums::RefundStatus::Success,
            ("PENDING", _) => enums::RefundStatus::Pending,
            _ => enums::RefundStatus::Failure,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, MpgsRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, MpgsRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match (item.response.result.as_str(), item.response.response.gateway_code.as_str()) {
            ("SUCCESS", "APPROVED") => enums::RefundStatus::Success,
            ("PENDING", _) => enums::RefundStatus::Pending,
            _ => enums::RefundStatus::Failure,
        };

        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpgsWebhookBody {
    pub order: MpgsWebhookOrder,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpgsWebhookOrder {
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MpgsWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
}

// Error Response Structure
#[derive(Debug, Serialize, Deserialize)]
pub struct MpgsErrorResponse {
    pub error: MpgsError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MpgsError {
    pub cause: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_type: Option<String>,
}
