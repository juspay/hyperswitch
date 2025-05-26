use common_enums::enums;
use common_utils::types::StringMinorUnit;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

//TODO: Fill the struct with respective fields
pub struct SpreedlyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for SpreedlyRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Spreedly Authorize Request
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SpreedlyPaymentsRequest {
    transaction: SpreedlyTransaction,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct SpreedlyTransaction {
    credit_card: SpreedlyCreditCard,
    amount: StringMinorUnit,
    currency_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct SpreedlyCreditCard {
    number: cards::CardNumber,
    verification_value: Secret<String>,
    month: Secret<String>,
    year: Secret<String>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}

impl TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeRouterData>> for SpreedlyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let credit_card = SpreedlyCreditCard {
                    number: req_card.card_number,
                    verification_value: req_card.card_cvc,
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year,
                    first_name: req_card.card_holder_name.clone()
                        .and_then(|name| {
                            let parts: Vec<&str> = name.peek().split_whitespace().collect();
                            parts.first().map(|s| Secret::new(s.to_string()))
                        })
                        .unwrap_or_else(|| Secret::new("".to_string())),
                    last_name: req_card.card_holder_name
                        .and_then(|name| {
                            let parts: Vec<&str> = name.peek().split_whitespace().collect();
                            if parts.len() > 1 {
                                Some(Secret::new(parts[1..].join(" ")))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| Secret::new("".to_string())),
                };
                
                let transaction = SpreedlyTransaction {
                    credit_card,
                    amount: item.amount.clone(),
                    currency_code: item.router_data.request.currency.to_string(),
                };
                
                Ok(Self { transaction })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct for Spreedly HTTP Basic Auth
pub struct SpreedlyAuthType {
    pub(super) environment_key: Secret<String>,
    pub(super) access_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret: _,
            } => Ok(Self {
                environment_key: api_key.to_owned(),
                access_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// Spreedly Authorize Response
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyPaymentsResponse {
    transaction: SpreedlyTransactionResponse,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyTransactionResponse {
    token: String,
    succeeded: bool,
    transaction_type: String,
    amount: i64,
    currency_code: String,
    payment_method: Option<SpreedlyPaymentMethod>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyPaymentMethod {
    token: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, SpreedlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SpreedlyPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = if item.response.transaction.succeeded {
            common_enums::AttemptStatus::Authorized
        } else {
            common_enums::AttemptStatus::Failure
        };
        
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.token.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.transaction.payment_method
                    .as_ref()
                    .map(|pm| pm.token.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// CAPTURE:
// Type definition for Capture Request
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyCaptureRequest {
    transaction: SpreedlyCaptureTransaction,
}

#[derive(Default, Debug, Serialize)]
pub struct SpreedlyCaptureTransaction {
    amount: StringMinorUnit,
}

// Type definition for Capture Response
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyCaptureResponse {
    transaction: SpreedlyCaptureResponseTransaction,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyCaptureResponseTransaction {
    token: String,
    succeeded: bool,
    transaction_type: String,
    amount: i64,
}

// Capture Request Transformation
impl TryFrom<&SpreedlyRouterData<&hyperswitch_domain_models::types::PaymentsCaptureRouterData>>
    for SpreedlyCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&hyperswitch_domain_models::types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: SpreedlyCaptureTransaction {
                amount: item.amount.clone(),
            },
        })
    }
}

// Capture Response Transformation
impl<F, T> TryFrom<ResponseRouterData<F, SpreedlyCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, SpreedlyCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = if item.response.transaction.succeeded {
            common_enums::AttemptStatus::CaptureInitiated
        } else {
            common_enums::AttemptStatus::CaptureFailed
        };
        
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.transaction.token.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND:
// Type definition for Refund Request
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    transaction: SpreedlyRefundTransaction,
}

#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundTransaction {
    amount: StringMinorUnit,
}

impl<F> TryFrom<&SpreedlyRouterData<&RefundsRouterData<F>>> for SpreedlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SpreedlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: SpreedlyRefundTransaction {
                amount: item.amount.to_owned(),
            },
        })
    }
}

// Type definition for Refund Response
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyRefundResponse {
    transaction: SpreedlyRefundResponseTransaction,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpreedlyRefundResponseTransaction {
    token: String,
    succeeded: bool,
    transaction_type: String,
    amount: i64,
}

// SYNC:
// Type definition for Sync Response (same structure as other transaction responses)
pub type SpreedlySyncResponse = SpreedlyPaymentsResponse;

impl TryFrom<RefundsResponseRouterData<Execute, SpreedlyRefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, SpreedlyRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = if item.response.transaction.succeeded {
            enums::RefundStatus::Success
        } else {
            enums::RefundStatus::Failure
        };
        
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.token.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, SpreedlyRefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, SpreedlyRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = if item.response.transaction.succeeded {
            enums::RefundStatus::Success
        } else {
            enums::RefundStatus::Failure
        };
        
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction.token.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// WEBHOOKS:
// Spreedly webhook event structure
#[derive(Debug, Serialize, Deserialize)]
pub struct SpreedlyWebhookEvent {
    pub event_type: String,
    pub occurred_at: String,
    pub transaction: Option<SpreedlyWebhookTransaction>,
    pub gateway_transaction: Option<SpreedlyGatewayTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpreedlyWebhookTransaction {
    pub token: String,
    pub transaction_type: String,
    pub succeeded: bool,
    pub state: String,
    pub amount: i64,
    pub currency_code: String,
    pub order_id: Option<String>,
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpreedlyGatewayTransaction {
    pub token: String,
    pub gateway_type: String,
    pub name: String,
}

// Webhook signature verification
#[derive(Debug)]
pub struct SpreedlyWebhookSignature {
    pub signature: String,
    pub timestamp: String,
}
