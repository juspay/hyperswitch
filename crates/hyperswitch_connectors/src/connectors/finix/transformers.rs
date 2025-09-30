pub mod finix_common;
pub mod request;
pub mod response;
use common_enums::enums;
use common_utils::types::MinorUnit;
pub use finix_common::*;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::{
        self as flows,
        refunds::{Execute, RSync},
    },
    router_request_types::{ConnectorCustomerData, PaymentMethodTokenizationData, ResponseId},
    router_response_types::{
        ConnectorCustomerResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
pub use request::*;
pub use response::*;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::RouterData as _,
};

pub enum FinixFlow {
    CreateConnectorCustomer,
    Tokenization,
    Auth,
    Transfer,
    Void,
    Refund,
}

pub struct FinixRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for FinixRouterData<T> {
    fn from((amount, item, flow): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//------------------------
impl
    TryFrom<
        &FinixRouterData<
            &RouterData<
                flows::CreateConnectorCustomer,
                ConnectorCustomerData,
                PaymentsResponseData,
            >,
        >,
    > for FinixCreateIdentityRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            &RouterData<
                flows::CreateConnectorCustomer,
                ConnectorCustomerData,
                PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        let customer_data = &item.router_data.request;

        // Create entity data
        let entity = FinixIdentityEntity {
            phone: customer_data.phone.clone(),
            first_name: customer_data.name.clone().map(|name| {
                let binding = name.clone().expose();
                // Split name into first and last if available
                let parts: Vec<&str> = binding.split_whitespace().collect();
                if !parts.is_empty() {
                    Secret::new(parts[0].to_string())
                } else {
                    name
                }
            }),
            last_name: customer_data.name.clone().map(|name| {
                let binding = name.clone().expose();

                let parts: Vec<&str> = binding.split_whitespace().collect();
                if parts.len() > 1 {
                    Secret::new(parts[1..].join(" "))
                } else {
                    Secret::new(String::new())
                }
            }),
            email: customer_data.email.clone(),
            personal_address: customer_data
                .billing_address
                .as_ref()
                .map(FinixAddress::from),
        };

        // Create the request
        Ok(Self {
            entity,
            tags: None,
            identity_type: FinixIdentityType::PERSONAL,
        })
    }
}

// Implement response handling for Identity creation
impl<F, T> TryFrom<ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixIdentityResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new_with_customer_id(item.response.id),
            )),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct FinixCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&FinixRouterData<&PaymentsAuthorizeRouterData>> for FinixPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FinixRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(_) => {
                // Check if we have a payment instrument token already
                let source = item.router_data.get_payment_method_token()?;

                Ok(Self {
                    amount: item.amount,
                    currency: item.router_data.request.currency,
                    source: match source {
                        PaymentMethodToken::Token(token) => token,
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Stax"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Stax"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Stax"))?
                        }
                    },
                    auth_type: Some("AUTHORIZATION".to_string()),
                    merchant: None, // todo
                    tags: None,
                    three_d_secure: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported".to_string(),
            )
            .into()),
        }
    }
}

impl TryFrom<&FinixRouterData<&PaymentsCaptureRouterData>> for FinixPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FinixRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        // Check if we have a payment instrument token already
        let source = item.router_data.get_payment_method_token()?;

        Ok(Self {
            amount: item.router_data.request.minor_amount_to_capture,
            currency: item.router_data.request.currency,
            source: match source {
                PaymentMethodToken::Token(token) => token,
                PaymentMethodToken::ApplePayDecrypt(_) => Err(unimplemented_payment_method!(
                    "Apple Pay",
                    "Simplified",
                    "Stax"
                ))?,
                PaymentMethodToken::PazeDecrypt(_) => {
                    Err(unimplemented_payment_method!("Paze", "Stax"))?
                }
                PaymentMethodToken::GooglePayDecrypt(_) => {
                    Err(unimplemented_payment_method!("Google Pay", "Stax"))?
                }
            },
            auth_type: Some("AUTHORIZATION".to_string()),
            merchant: None, //to do
            tags: None,
            three_d_secure: None,
        })
    }
}

impl
    TryFrom<
        &FinixRouterData<
            &RouterData<
                flows::PaymentMethodToken,
                PaymentMethodTokenizationData,
                PaymentsResponseData,
            >,
        >,
    > for FinixCreatePaymentInstrumentRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FinixRouterData<
            &RouterData<
                flows::PaymentMethodToken,
                PaymentMethodTokenizationData,
                PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        let tokenization_data = &item.router_data.request;

        match &tokenization_data.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                // let address = item
                //     .router_data
                //     .get_billing_address()
                //     .map(FinixAddress::from);

                Ok(Self {
                    instrument_type: FinixPaymentInstrumentType::PaymentCard,
                    name: card_data.card_holder_name.clone(),
                    number: Some(Secret::new(card_data.card_number.clone().get_card_no())),
                    security_code: Some(card_data.card_cvc.clone()),
                    expiration_month: Some(Secret::new(
                        card_data
                            .card_exp_month
                            .clone()
                            .expose()
                            .parse::<i32>()
                            .unwrap_or(0),
                    )),
                    expiration_year: Some(Secret::new(
                        card_data
                            .card_exp_year
                            .clone()
                            .expose()
                            .parse::<i32>()
                            .unwrap_or(0),
                    )),
                    identity: item.router_data.get_connector_customer_id()?, // This would come from a previously created identity
                    tags: None,
                    address: None,
                    card_brand: None, // Finix determines this from the card number
                    card_type: None,  // Finix determines this from the card number
                    additional_data: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported for tokenization".to_string(),
            )
            .into()),
        }
    }
}

// Implement response handling for tokenization
impl<F, T> TryFrom<ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixInstrumentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::Charged,
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.id,
            }),
            ..item.data
        })
    }
}

// Auth Struct
pub struct FinixAuthType {
    pub finix_user_name: Secret<String>,
    pub finix_password: Secret<String>,
    pub merchant_id: Secret<String>,
}
impl TryFrom<&ConnectorAuthType> for FinixAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                finix_user_name: api_key.clone(),
                finix_password: api_secret.clone(),
                merchant_id: key1.clone(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FinixPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<FinixPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: FinixPaymentStatus) -> Self {
        match item {
            FinixPaymentStatus::Succeeded => Self::Authorized,
            FinixPaymentStatus::Failed => Self::Failure,
            FinixPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

fn get_payment_status(state: FinixState, flow: FinixFlow) -> common_enums::AttemptStatus {
    todo!()
}
//TODO: Fill the struct with respective fields

impl<F, T> TryFrom<ResponseRouterData<F, FinixPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FinixPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_payment_status(item.response.state, FinixFlow::Auth), //todo
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct FinixRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&FinixRouterData<&RefundsRouterData<F>>> for FinixRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &FinixRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
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
pub struct FinixErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}
