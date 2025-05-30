use common_enums::enums;
use common_utils::types::StringMinorUnit;
use masking::{ExposeInterface, Secret};
use cards::CardNumber;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct AuthipayRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for AuthipayRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

// Authipay payment request structure
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayPaymentsRequest {
    request_type: String,
    transaction_amount: AuthipayAmount,
    payment_method: AuthipayPaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_transaction_id: Option<String>,
    store_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored_credentials: Option<AuthipayStoredCredentials>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AuthipayAmount {
    total: StringMinorUnit,
    currency: String,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayPaymentMethod {
    payment_card: AuthipayCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AuthipayCard {
    number: CardNumber,
    security_code: Secret<String>,
    expiry_date: AuthipayExpiryDate,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AuthipayExpiryDate {
    month: Secret<String>,
    year: Secret<String>,
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct AuthipayStoredCredentials {
    sequence: String,
    scheduled: bool,
}

impl TryFrom<&AuthipayRouterData<&PaymentsAuthorizeRouterData>> for AuthipayPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AuthipayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                // Determine if it's a sale or pre-auth transaction based on capture method
                let request_type = if item.router_data.request.is_auto_capture()? {
                    "PaymentCardSaleTransaction"
                } else {
                    "PaymentCardPreAuthTransaction"
                }.to_string();

                let expiry_date = AuthipayExpiryDate {
                    month: req_card.card_exp_month,
                    year: req_card.card_exp_year.clone(),
                };

                let card = AuthipayCard {
                    number: req_card.card_number,
                    security_code: req_card.card_cvc,
                    expiry_date,
                };

                let payment_method = AuthipayPaymentMethod {
                    payment_card: card,
                };

                let auth = AuthipayAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

                Ok(Self {
                    request_type,
                    transaction_amount: AuthipayAmount {
                        total: item.amount.clone(),
                        currency: item.router_data.request.currency.to_string(),
                    },
                    payment_method,
                    merchant_transaction_id: Some(item.router_data.payment_id.clone()),
                    store_id: auth.store_id.clone(),
                    stored_credentials: None,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct AuthipayAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) api_secret: Secret<String>,
    pub(super) store_id: String,
}

impl TryFrom<&ConnectorAuthType> for AuthipayAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey { api_key, key1, api_secret } => Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                store_id: key1.clone().expose().to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse Status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayPaymentStatus {
    AUTHORIZED,
    CAPTURED,
    DECLINED,
    VOIDED,
    #[default]
    PENDING,
}

impl From<AuthipayPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: AuthipayPaymentStatus) -> Self {
        match item {
            AuthipayPaymentStatus::AUTHORIZED => Self::Authorized,
            AuthipayPaymentStatus::CAPTURED => Self::Charged,
            AuthipayPaymentStatus::DECLINED => Self::Failure,
            AuthipayPaymentStatus::VOIDED => Self::Voided,
            AuthipayPaymentStatus::PENDING => Self::Pending,
        }
    }
}

// Authipay Payment Response structure
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthipayPaymentsResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "ipgTransactionId")]
    ipg_transaction_id: String,
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "transactionTime")]
    transaction_time: i64,
    #[serde(rename = "transactionState")]
    transaction_state: AuthipayPaymentStatus,
    #[serde(rename = "paymentType")]
    payment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "transactionOrigin")]
    transaction_origin: Option<String>,
    amount: AuthipayAmount,
    #[serde(rename = "storeId")]
    store_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AuthipayPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.ipg_transaction_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct AuthipayRefundRequest {
    request_type: String,
    transaction_amount: AuthipayAmount,
    store_id: String,
}

impl<F> TryFrom<&AuthipayRouterData<&RefundsRouterData<F>>> for AuthipayRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AuthipayRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = AuthipayAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        Ok(Self {
            request_type: "ReturnTransaction".to_string(),
            transaction_amount: AuthipayAmount {
                total: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
            },
            store_id: auth.store_id.clone(),
        })
    }
}

// Type definition for Refund Response
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuthipayRefundStatus {
    RETURNED,
    DECLINED,
    #[default]
    PENDING,
}

impl From<AuthipayRefundStatus> for enums::RefundStatus {
    fn from(item: AuthipayRefundStatus) -> Self {
        match item {
            AuthipayRefundStatus::RETURNED => Self::Success,
            AuthipayRefundStatus::DECLINED => Self::Failure,
            AuthipayRefundStatus::PENDING => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(rename = "clientRequestId")]
    client_request_id: String,
    #[serde(rename = "apiTraceId")]
    api_trace_id: String,
    #[serde(rename = "ipgTransactionId")]
    ipg_transaction_id: String,
    #[serde(rename = "orderId")]
    order_id: String,
    #[serde(rename = "transactionTime")]
    transaction_time: i64,
    #[serde(rename = "transactionState")]
    transaction_state: AuthipayRefundStatus,
    #[serde(rename = "paymentType")]
    payment_type: String,
    amount: AuthipayAmount,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_state),
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
                connector_refund_id: item.response.ipg_transaction_id,
                refund_status: enums::RefundStatus::from(item.response.transaction_state),
            }),
            ..item.data
        })
    }
}

// Authipay Error Response structure
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorResponse {
    #[serde(rename = "clientRequestId")]
    pub client_request_id: String,
    #[serde(rename = "apiTraceId")]
    pub api_trace_id: String,
    pub error: AuthipayErrorDetails,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default)]
    pub details: Option<Vec<AuthipayErrorDetail>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthipayErrorDetail {
    pub location: String,
    pub message: String,
    #[serde(rename = "locationType")]
    pub location_type: String,
}
