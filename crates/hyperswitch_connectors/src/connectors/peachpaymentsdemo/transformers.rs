use common_enums::enums;
use common_utils::types::MinorUnit;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, CardData},
};

//TODO: Fill the struct with respective fields
pub struct PeachpaymentsdemoRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PeachpaymentsdemoRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsdemoPaymentsRequest {
    charge_method: PeachPaymentMethod,
    reference_id: String,
    ecommerce_card_payment_only_transaction_data: EcommerceCardPaymentOnlyTransactionData,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PeachPaymentMethod {
    EcommerceCardPaymentOnly,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceCardPaymentOnlyTransactionData {
    merchant_information: MerchantInformation,
    routing_reference: PaymentMethodRouteData,
    card: PeachpaymentsdemoCard,
    amount: AmountDetails,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodRouteData {
    merchant_payment_method_route_id: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsdemoCard {
    pub pan: cards::CardNumber,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<Secret<String>>,
    pub expiry_year: Secret<String>,
    pub expiry_month: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmountDetails {
    pub amount: MinorUnit,
    pub currency_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInformation {
    client_merchant_reference_id: Secret<String>,
}

impl TryFrom<&PeachpaymentsdemoRouterData<&PaymentsAuthorizeRouterData>>
    for PeachpaymentsdemoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsdemoRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = PeachpaymentsdemoAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(card_data) => Ok(Self {
                charge_method: PeachPaymentMethod::EcommerceCardPaymentOnly,
                reference_id: item.router_data.connector_request_reference_id.clone(),
                ecommerce_card_payment_only_transaction_data:
                    EcommerceCardPaymentOnlyTransactionData {
                        merchant_information: MerchantInformation {
                            client_merchant_reference_id: auth.client_merchant_reference_id.clone(),
                        },
                        routing_reference: PaymentMethodRouteData {
                            merchant_payment_method_route_id: auth.routing_reference.clone(),
                        },
                        card: PeachpaymentsdemoCard {
                            pan: card_data.card_number.clone(),
                            cardholder_name: card_data.card_holder_name.clone(),
                            expiry_month: card_data.get_card_expiry_month_2_digit()?,
                            expiry_year: card_data.get_card_expiry_year_2_digit()?,
                            cvv: card_data.card_cvc.clone(),
                        },
                        amount: AmountDetails {
                            amount: item.amount,
                            currency_code: item.router_data.request.currency.to_string(),
                        },
                    },
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PeachpaymentsdemoAuthType {
    pub(super) client_merchant_reference_id: Secret<String>,
    pub(super) api_key: Secret<String>,
    pub(super) tenant_id: Secret<String>,
    pub(super) routing_reference: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PeachpaymentsdemoAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                tenant_id: key1.to_owned(),
                client_merchant_reference_id: api_secret.to_owned(),
                routing_reference: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PeachpaymentsdemoPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<PeachpaymentsdemoPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PeachpaymentsdemoPaymentStatus) -> Self {
        match item {
            PeachpaymentsdemoPaymentStatus::Succeeded => Self::Charged,
            PeachpaymentsdemoPaymentStatus::Failed => Self::Failure,
            PeachpaymentsdemoPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsdemoPaymentsResponse {
    pub transaction_id: String,
    pub response_code: Option<ResponseCode>,
    pub transaction_result: PeachpaymentsPaymentStatus,
    pub ecommerce_card_payment_only_transaction_data: Option<EcommerceCardPaymentOnlyResponseData>,
}

// Card Gateway API Response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PeachpaymentsPaymentStatus {
    Successful,
    Pending,
    Authorized,
    Approved,
    ApprovedConfirmed,
    Declined,
    Failed,
    Reversed,
    ThreedsRequired,
    Voided,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcommerceCardPaymentOnlyResponseData {
    pub amount: Option<AmountDetails>,
    pub stan: Option<Secret<String>>,
    pub rrn: Option<Secret<String>>,
    pub approval_code: Option<String>,
    pub merchant_advice_code: Option<String>,
    pub description: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ResponseCode {
    Text(String),
    Structured {
        value: String,
        description: String,
        terminal_outcome_string: Option<String>,
        receipt_string: Option<String>,
    },
}

impl<F, T>
    TryFrom<ResponseRouterData<F, PeachpaymentsdemoPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PeachpaymentsdemoPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.transaction_result);
        let response = if !is_payment_success(
            item.response
                .response_code
                .as_ref()
                .and_then(|code| code.value()),
        ) {
            Err(ErrorResponse {
                code: get_error_code(item.response.response_code.as_ref()),
                message: get_error_message(item.response.response_code.as_ref()),
                reason: item
                    .response
                    .ecommerce_card_payment_only_transaction_data
                    .and_then(|data| data.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.transaction_id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

fn get_error_code(response_code: Option<&ResponseCode>) -> String {
    response_code
        .and_then(|code| code.value())
        .map(|val| val.to_string())
        .unwrap_or(
            response_code
                .and_then(|code| code.as_text())
                .map(|text| text.to_string())
                .unwrap_or(NO_ERROR_CODE.to_string()),
        )
}

fn get_error_message(response_code: Option<&ResponseCode>) -> String {
    response_code
        .and_then(|code| code.description())
        .map(|desc| desc.to_string())
        .unwrap_or(
            response_code
                .and_then(|code| code.as_text())
                .map(|text| text.to_string())
                .unwrap_or(NO_ERROR_MESSAGE.to_string()),
        )
}

impl From<PeachpaymentsPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PeachpaymentsPaymentStatus) -> Self {
        match item {
            // PENDING means authorized but not yet captured - requires confirmation
            PeachpaymentsPaymentStatus::Pending
            | PeachpaymentsPaymentStatus::Authorized
            | PeachpaymentsPaymentStatus::Approved => Self::Authorized,
            PeachpaymentsPaymentStatus::Declined | PeachpaymentsPaymentStatus::Failed => {
                Self::Failure
            }
            PeachpaymentsPaymentStatus::Voided | PeachpaymentsPaymentStatus::Reversed => {
                Self::Voided
            }
            PeachpaymentsPaymentStatus::ThreedsRequired => Self::AuthenticationPending,
            PeachpaymentsPaymentStatus::ApprovedConfirmed
            | PeachpaymentsPaymentStatus::Successful => Self::Charged,
        }
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct PeachpaymentsdemoRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&PeachpaymentsdemoRouterData<&RefundsRouterData<F>>>
    for PeachpaymentsdemoRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PeachpaymentsdemoRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
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
pub struct PeachpaymentsdemoErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
    pub network_advice_code: Option<String>,
    pub network_decline_code: Option<String>,
    pub network_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeachpaymentsErrorResponse {
    pub error_ref: String,
    pub message: String,
}

impl ResponseCode {
    pub fn value(&self) -> Option<&String> {
        match self {
            Self::Structured { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn description(&self) -> Option<&String> {
        match self {
            Self::Structured { description, .. } => Some(description),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&String> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

fn is_payment_success(value: Option<&String>) -> bool {
    if let Some(val) = value {
        val == "00" || val == "08" || val == "X94"
    } else {
        false
    }
}
