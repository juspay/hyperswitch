#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use std::str::FromStr;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    types::{MinorUnit, StringMinorUnit},
};
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
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
use time::PrimitiveDateTime;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct ChargebeeRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for ChargebeeRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ChargebeePaymentsRequest {
    amount: StringMinorUnit,
    card: ChargebeeCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ChargebeeCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ChargebeeRouterData<&PaymentsAuthorizeRouterData>> for ChargebeePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = ChargebeeCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct ChargebeeAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for ChargebeeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChargebeePaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<ChargebeePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: ChargebeePaymentStatus) -> Self {
        match item {
            ChargebeePaymentStatus::Succeeded => Self::Charged,
            ChargebeePaymentStatus::Failed => Self::Failure,
            ChargebeePaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChargebeePaymentsResponse {
    status: ChargebeePaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, ChargebeePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, ChargebeePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
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
pub struct ChargebeeRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&ChargebeeRouterData<&RefundsRouterData<F>>> for ChargebeeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ChargebeeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct ChargebeeErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeWebhookBody {
    pub content: ChargebeeWebhookContent,
    pub event_type: ChargebeeEventType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceBody {
    pub content: ChargebeeInvoiceContent,
    pub event_type: ChargebeeEventType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceContent {
    pub invoice: ChargebeeInvoiceData,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct ChargebeeWebhookContent {
    pub transaction: ChargebeeTransactionData,
    pub invoice: ChargebeeInvoiceData,
    pub customer: Option<ChargebeeCustomer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeEventType {
    PaymentSucceeded,
    PaymentFailed,
    InvoiceDeleted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceData {
    // invoice id
    pub id: String,
    pub total: MinorUnit,
    pub currency_code: enums::Currency,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeTransactionData {
    id_at_gateway: Option<String>,
    status: ChargebeeTranasactionStatus,
    error_code: Option<String>,
    error_text: Option<String>,
    gateway_account_id: Option<String>,
    currency_code: enums::Currency,
    amount: MinorUnit,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    date: Option<PrimitiveDateTime>,
    payment_method: ChargebeeTransactionPaymentMethod,
    payment_method_details: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTransactionPaymentMethod {
    Card,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeePaymentMethodDetails {
    card: ChargebeeCardDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeCardDetails {
    funding_type: ChargebeeFundingType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeFundingType {
    Credit,
    Debit,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTranasactionStatus {
    // Waiting for response from the payment gateway.
    InProgress,
    // The transaction is successful.
    Success,
    // Transaction failed.
    Failure,
    // No response received while trying to charge the card.
    Timeout,
    // Indicates that a successful payment transaction has failed now due to a late failure notification from the payment gateway,
    // typically caused by issues like insufficient funds or a closed bank account.
    LateFailure,
    // Connection with Gateway got terminated abruptly. So, status of this transaction needs to be resolved manually
    NeedsAttention,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeCustomer {
    pub payment_method: ChargebeePaymentMethod,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeePaymentMethod {
    pub reference_id: String,
    pub gateway: ChargebeeGateway,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeGateway {
    Stripe,
    Braintree,
}

impl ChargebeeWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("ChargebeeWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

impl ChargebeeInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("ChargebeeInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

impl ChargebeeCustomer {
    // the logic to find connector customer id & mandate id is different for different gateways, reference : https://apidocs.chargebee.com/docs/api/customers?prod_cat_ver=2#customer_payment_method_reference_id .
    pub fn find_connector_ids(&self) -> Result<(String, String), errors::ConnectorError> {
        match self.payment_method.gateway {
            ChargebeeGateway::Stripe | ChargebeeGateway::Braintree => {
                let mut parts = self.payment_method.reference_id.split('/');
                let customer_id = parts
                    .next()
                    .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?
                    .to_string();
                let mandate_id = parts
                    .last()
                    .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?
                    .to_string();
                Ok((customer_id, mandate_id))
            }
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<ChargebeeWebhookBody> for revenue_recovery::RevenueRecoveryAttemptData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebeeWebhookBody) -> Result<Self, Self::Error> {
        let amount = item.content.transaction.amount;
        let currency = item.content.transaction.currency_code.to_owned();
        let merchant_reference_id =
            common_utils::id_type::PaymentReferenceId::from_str(&item.content.invoice.id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let connector_transaction_id = item
            .content
            .transaction
            .id_at_gateway
            .map(common_utils::types::ConnectorTransactionId::TxnId);
        let error_code = item.content.transaction.error_code.clone();
        let error_message = item.content.transaction.error_text.clone();
        let (connector_customer_id, processor_payment_method_token) = match &item.content.customer {
            Some(customer) => {
                let (customer_id, mandate_id) = customer.find_connector_ids()?;
                (Some(customer_id), Some(mandate_id))
            }
            None => (None, None),
        };
        let connector_account_reference_id = item.content.transaction.gateway_account_id.clone();
        let transaction_created_at = item.content.transaction.date;
        let status = enums::AttemptStatus::from(item.content.transaction.status);
        let payment_method_type =
            enums::PaymentMethod::from(item.content.transaction.payment_method);
        let payment_method_details: ChargebeePaymentMethodDetails =
            serde_json::from_str(&item.content.transaction.payment_method_details)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let payment_method_sub_type =
            enums::PaymentMethodType::from(payment_method_details.card.funding_type);
        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            connector_transaction_id,
            error_code,
            error_message,
            processor_payment_method_token,
            connector_customer_id,
            connector_account_reference_id,
            transaction_created_at,
            status,
            payment_method_type,
            payment_method_sub_type,
        })
    }
}

impl From<ChargebeeTranasactionStatus> for enums::AttemptStatus {
    fn from(status: ChargebeeTranasactionStatus) -> Self {
        match status {
            ChargebeeTranasactionStatus::InProgress
            | ChargebeeTranasactionStatus::NeedsAttention => Self::Pending,
            ChargebeeTranasactionStatus::Success => Self::Charged,
            ChargebeeTranasactionStatus::Failure
            | ChargebeeTranasactionStatus::Timeout
            | ChargebeeTranasactionStatus::LateFailure => Self::Pending,
        }
    }
}

impl From<ChargebeeTransactionPaymentMethod> for enums::PaymentMethod {
    fn from(payment_method: ChargebeeTransactionPaymentMethod) -> Self {
        match payment_method {
            ChargebeeTransactionPaymentMethod::Card => Self::Card,
        }
    }
}

impl From<ChargebeeFundingType> for enums::PaymentMethodType {
    fn from(funding_type: ChargebeeFundingType) -> Self {
        match funding_type {
            ChargebeeFundingType::Credit => Self::Credit,
            ChargebeeFundingType::Debit => Self::Debit,
        }
    }
}
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl From<ChargebeeEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(event: ChargebeeEventType) -> Self {
        match event {
            ChargebeeEventType::PaymentSucceeded => Self::RecoveryPaymentSuccess,
            ChargebeeEventType::PaymentFailed => Self::RecoveryPaymentFailure,
            ChargebeeEventType::InvoiceDeleted => Self::RecoveryInvoiceCancel,
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<ChargebeeInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebeeInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            common_utils::id_type::PaymentReferenceId::from_str(&item.content.invoice.id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Self {
            amount: item.content.invoice.total,
            currency: item.content.invoice.currency_code,
            merchant_reference_id,
        })
    }
}
