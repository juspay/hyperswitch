use std::collections::HashMap;
use api_models::webhooks::IncomingWebhookEvent;
use cards::CardNumber;
use common_enums::{AttemptStatus, Currency};
use common_utils::types::{StringMinorUnit};
use error_stack::{report, ResultExt, Report};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{Authorize, Capture, Execute, RSync, Void},
    router_request_types::{PaymentsAuthorizeData, PaymentsCaptureData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors::ConnectorError;
use masking::{Secret, ExposeInterface};
use serde::{Deserialize, Serialize};

use crate::{
    types::{ResponseRouterData, RefundsResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

// TabaPay Router Data
#[derive(Debug, Clone)]
pub struct TabaPayRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for TabaPayRouterData<T> {
    fn from(data: (StringMinorUnit, T)) -> Self {
        Self {
            amount: data.0,
            router_data: data.1,
        }
    }
}

// Auth struct for TabaPay connector
#[derive(Debug, Clone)]
pub struct TabaPayAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for TabaPayAuthType {
    type Error = Report<ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// TabaPay Transaction Types (Pull or Push)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TransactionType {
    Pull,
    Push,
}

// TabaPay Card Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayCard {
    #[serde(rename = "accountNumber")]
    pub account_number: CardNumber,
    pub cvv: Secret<String>,
    #[serde(rename = "expiryMonth")]
    pub expiry_month: Secret<String>,
    #[serde(rename = "expiryYear")]
    pub expiry_year: Secret<String>,
}

// TabaPay Account Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayAccount {
    #[serde(rename = "accountType")]
    pub account_type: String,
    pub card: TabaPayCard,
}

// TabaPay Destination Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayDestination {
    pub id: String,
}

// TabaPay Source Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPaySource {
    #[serde(flatten)]
    pub account: TabaPayAccount,
}

// TabaPay Payment Request
#[derive(Debug, Clone, Serialize)]
pub struct TabaPayPaymentsRequest {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: StringMinorUnit,
    pub currency: Currency,
    pub source: TabaPaySource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<TabaPayDestination>,
}

// TabaPay Capture Request
#[derive(Debug, Serialize)]
pub struct TabaPayCaptureRequest {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    pub amount: StringMinorUnit,
}

impl TryFrom<&TabaPayRouterData<&PaymentsAuthorizeRouterData>> for TabaPayPaymentsRequest {
    type Error = Report<ConnectorError>;

    fn try_from(item: &TabaPayRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            Some(PaymentMethodData::Card(card)) => {
                let source = TabaPaySource {
                    account: TabaPayAccount {
                        account_type: "CARD".to_string(),
                        card: TabaPayCard {
                            account_number: card.card_number.clone(),
                            cvv: card.card_cvc.clone(),
                            expiry_month: card.card_exp_month.clone(),
                            expiry_year: card.card_exp_year.clone(),
                        },
                    },
                };

                Ok(Self {
                    reference_id: item.router_data.connector_request_reference_id.clone(),
                    transaction_type: TransactionType::Pull,
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    source,
                    destination: None,
                })
            }
            _ => Err(ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// TabaPay Payment Status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TabaPayPaymentStatus {
    Approved,
    Declined,
    Pending,
    Cancelled,
    Error,
}

impl From<TabaPayPaymentStatus> for AttemptStatus {
    fn from(status: TabaPayPaymentStatus) -> Self {
        match status {
            TabaPayPaymentStatus::Approved => Self::Charged,
            TabaPayPaymentStatus::Declined | TabaPayPaymentStatus::Error => Self::Failure,
            TabaPayPaymentStatus::Pending => Self::Pending,
            TabaPayPaymentStatus::Cancelled => Self::Voided,
        }
    }
}

// TabaPay Network Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayNetworkResponse {
    pub code: String,
    pub message: Option<String>,
}

// TabaPay Payment Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayPaymentsResponse {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub status: TabaPayPaymentStatus,
    pub network: Option<TabaPayNetworkResponse>,
    #[serde(rename = "networkId")]
    pub network_id: Option<String>,
}

impl TryFrom<ResponseRouterData<Authorize, TabaPayPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> 
    for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData> {
    type Error = Report<ConnectorError>;

    fn try_from(
        item: ResponseRouterData<Authorize, TabaPayPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status);
        
        let error_response = if status == AttemptStatus::Failure {
            Some(hyperswitch_domain_models::router_data::ErrorResponse {
                code: item.response.network.as_ref().map_or("".to_string(), |n| n.code.clone()),
                message: item.response.network.as_ref().and_then(|n| n.message.clone()).unwrap_or_default(),
                reason: None,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            })
        } else {
            None
        };

        let response = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: item.response.network_id,
            connector_response_reference_id: Some(item.response.reference_id),
            incremental_authorization_allowed: None,
            charges: None,
        };

        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(response), Err),
            ..item.data
        })
    }
}

impl TryFrom<ResponseRouterData<Capture, TabaPayPaymentsResponse, PaymentsCaptureData, PaymentsResponseData>> 
    for RouterData<Capture, PaymentsCaptureData, PaymentsResponseData> {
    type Error = Report<ConnectorError>;

    fn try_from(
        item: ResponseRouterData<Capture, TabaPayPaymentsResponse, PaymentsCaptureData, PaymentsResponseData>
    ) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from(item.response.status);
        
        let error_response = if status == AttemptStatus::Failure {
            Some(hyperswitch_domain_models::router_data::ErrorResponse {
                code: item.response.network.as_ref().map_or("".to_string(), |n| n.code.clone()),
                message: item.response.network.as_ref().and_then(|n| n.message.clone()).unwrap_or_default(),
                reason: None,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
            })
        } else {
            None
        };

        let response = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.transaction_id.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: item.response.network_id,
            connector_response_reference_id: Some(item.response.reference_id),
            incremental_authorization_allowed: None,
            charges: None,
        };

        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(response), Err),
            ..item.data
        })
    }
}

// TabaPay Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayErrorResponse {
    pub code: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
}

// Implement refund request and response types
#[derive(Debug, Clone, Serialize)]
pub struct TabaPayRefundRequest {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    pub amount: StringMinorUnit,
    #[serde(rename = "originalTransactionId")]
    pub original_transaction_id: String,
}

impl<F> TryFrom<&TabaPayRouterData<&RefundsRouterData<F>>> for TabaPayRefundRequest {
    type Error = Report<ConnectorError>;

    fn try_from(item: &TabaPayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            reference_id: item.router_data.request.refund_id.clone(),
            amount: item.amount.clone(),
            original_transaction_id: item.router_data.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabaPayRefundResponse {
    #[serde(rename = "referenceId")]
    pub reference_id: String,
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub status: TabaPayPaymentStatus,
    pub network: Option<TabaPayNetworkResponse>,
    #[serde(rename = "networkId")]
    pub network_id: Option<String>,
}