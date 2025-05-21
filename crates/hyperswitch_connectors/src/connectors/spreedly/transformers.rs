use common_enums::{enums, PaymentMethodType};
use common_utils::types::StringMinorUnit;
use error_stack::Report;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{payments::Authorize, refunds::{Execute, RSync}},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, MandateReference, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
    types::{self, RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData},
};

// Router Data
pub struct SpreedlyRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for SpreedlyRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

// Card details module to handle card specific data
pub mod cards {
    use masking::Secret;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    pub struct CardNumber(pub Secret<String>);

    #[derive(Debug, Serialize)]
    pub struct CVV(pub Secret<String>);

    impl From<Secret<String>> for CardNumber {
        fn from(card_number: Secret<String>) -> Self {
            Self(card_number)
        }
    }

    impl From<Secret<String>> for CVV {
        fn from(cvv: Secret<String>) -> Self {
            Self(cvv)
        }
    }
}

// Request Structs
#[derive(Debug, Serialize)]
pub struct SpreedlyPaymentsRequest {
    pub transaction: SpreedlyTransactionRequest,
}

#[derive(Debug, Serialize)]
pub struct SpreedlyTransactionRequest {
    pub credit_card: SpreedlyCardDetails,
    pub amount: i64,
    pub currency_code: String,
}

#[derive(Debug, Serialize)]
pub struct SpreedlyCardDetails {
    pub number: cards::CardNumber,
    #[serde(rename = "verification_value")]
    pub cvv: cards::CVV,
    pub month: String,
    pub year: String,
    #[serde(rename = "first_name")]
    pub first_name: Option<String>,
    #[serde(rename = "last_name")]
    pub last_name: Option<String>,
}

impl TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeRouterData>> for SpreedlyPaymentsRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(
        item: &SpreedlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = SpreedlyCardDetails {
                    number: req_card.card_number.into(),
                    cvv: req_card.card_cvc.into(),
                    month: req_card.card_exp_month.clone().expose(),
                    year: req_card.card_exp_year.clone().expose(),
                    first_name: req_card.card_holder_name.clone().map(|name| {
                        let name_str = name.expose();
                        name_str.split_whitespace()
                            .next()
                            .unwrap_or(&name_str)
                            .to_string()
                    }),
                    last_name: req_card.card_holder_name.clone().map(|name| {
                        let name_str = name.expose();
                        let parts: Vec<&str> = name_str.split_whitespace().collect();
                        if parts.len() > 1 {
                            Some(parts[1..].join(" "))
                        } else {
                            None
                        }
                    }).flatten(),
                };
                
                // Get the amount as i64
                let amount = item.amount.to_owned().get_amount_as_i64().unwrap_or(0);
                
                Ok(Self {
                    transaction: SpreedlyTransactionRequest {
                        credit_card: card,
                        amount,
                        currency_code: item.router_data.request.currency.to_string(),
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
#[derive(Debug, Serialize)]
pub struct SpreedlyAuthType {
    pub environment_key: String,
    pub access_secret: String,
}

impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
    type Error = Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey {
                api_key,
                api_secret,
                ..
            } => Ok(Self {
                environment_key: api_key.expose(),
                access_secret: api_secret.expose(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpreedlyPaymentsResponse {
    pub transaction: SpreedlyTransactionResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpreedlyTransactionResponse {
    pub token: String,
    pub succeeded: bool,
    #[serde(rename = "transaction_type")]
    pub transaction_type: String,
    pub amount: i64,
    #[serde(rename = "currency_code")]
    pub currency: Option<String>,
    pub payment_method: Option<SpreedlyPaymentMethod>,
    pub state: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpreedlyPaymentMethod {
    pub token: String,
    pub card_type: Option<String>,
    pub last_four_digits: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpreedlyPaymentsResponseWrapper {
    token: String,
    succeeded: bool,
    transaction_type: String,
    amount: i64,
}

impl From<SpreedlyPaymentsResponse> for SpreedlyPaymentsResponseWrapper {
    fn from(resp: SpreedlyPaymentsResponse) -> Self {
        Self {
            token: resp.transaction.token.clone(),
            succeeded: resp.transaction.succeeded,
            transaction_type: resp.transaction.transaction_type.clone(),
            amount: resp.transaction.amount,
        }
    }
}

// REFUND Types
#[derive(Default, Debug, Serialize)]
pub struct SpreedlyRefundRequest {
    #[serde(rename = "transaction_token")]
    pub transaction_token: String,
    pub amount: Option<i64>,
}

impl<F> TryFrom<&SpreedlyRouterData<&RefundsRouterData<F>>> for SpreedlyRefundRequest {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: &SpreedlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        // Get the amount as i64
        let amount = item.amount.to_owned().get_amount_as_i64();
        
        Ok(Self {
            transaction_token: item.router_data.request.connector_transaction_id.clone(),
            amount,
        })
    }
}

// Refund Response
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub status: RefundStatus,
    pub amount: Option<i64>,
    pub currency: Option<String>,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = Report<errors::ConnectorError>;
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
    type Error = Report<errors::ConnectorError>;
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

// Error Response
#[derive(Debug, Deserialize, Serialize)]
pub struct SpreedlyErrorResponse {
    pub errors: Option<Vec<SpreedlyError>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpreedlyError {
    pub key: Option<String>,
    pub message: Option<String>,
}

// Response Data Transformation
impl TryFrom<ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeRouterData, PaymentsResponseData>> for PaymentsAuthorizeRouterData {
    type Error = Report<errors::ConnectorError>;
    fn try_from(item: ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeRouterData, PaymentsResponseData>) -> Result<Self, Self::Error> {
        let mut router_data = item.data;
        
        // Set the session token
        let token = item.response.transaction.token.clone();
        router_data.response.session_token = Some(token.clone());
        
        let transaction_status = match item.response.transaction.succeeded {
            true => match item.response.transaction.state.clone().unwrap_or_default().as_str() {
                "succeeded" => enums::AttemptStatus::Charged,
                _ => enums::AttemptStatus::Authorized,
            },
            false => enums::AttemptStatus::Failure,
        };
        
        router_data.response.transaction_id = Some(token);
        
        router_data.response.connector_response_reference_id = item
            .response
            .transaction
            .payment_method
            .as_ref()
            .and_then(|pm| Some(pm.token.clone()));
            
        router_data.response.status = transaction_status;
        
        // Set the redirection and mandate data properly
        router_data.response.redirection_data = Box::new(None);
        router_data.response.mandate_reference = Box::new(None);
        
        Ok(router_data)
    }
}
