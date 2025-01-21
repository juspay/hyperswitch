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
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, types::MinorUnit};
use error_stack::ResultExt;

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
                charge_id: None,
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

pub struct ChargebeeWebhookContent {
    pub transaction: Option<ChargebeeTransactionData>,
    pub invoice: ChargebeeInvoiceData,
    pub customer: Option<ChargebeeCustomer>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChargebeeEventType {
    PaymentSucceeded,
    PaymentFailed,
    InvoiceDeleted
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeInvoiceData {
    // invoice id
    pub id: String,
    pub total: MinorUnit,
    pub currency_code : enums::Currency,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeTransactionData {
    // gateway transaction id
    id_at_gateway: String,
    // transaction status
    status: ChargebeeTranasactionStatus,
    error_code: Option<String>,
    error_text: Option<String>,
    // The gateway account reference used for this transaction
    gateway_account_id: Option<String>,
    currency_code: enums::Currency,
    amount: MinorUnit,
    // #[serde(with = "common_utils::custom_serde::timestamp")]
    // date: PrimitiveDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChargebeeTranasactionStatus {
    InProgress,
    Success,
    Failure,
    Timeout,
    LateFailure,
    NeedsAttention,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeCustomer{
    pub payment_method : ChargebeePaymentMethod,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeePaymentMethod{
    pub reference_id: String,
    pub gateway: ChargebeeGateway,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ChargebeeGateway{
    Stripe,
    Braintree,
}

impl ChargebeeWebhookBody {
    pub fn get_webhook_object_from_body(
        body: &[u8],
    ) -> CustomResult<ChargebeeWebhookBody, errors::ConnectorError> {
        let webhook_body: ChargebeeWebhookBody = body
            .parse_struct("ChargebeeWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        router_env::logger::debug!("$$$$$$ webhook_body {:?}",webhook_body);
        Ok(webhook_body)
    }
}

impl ChargebeeCustomer{
    // the logic to find connector customer id & mandate id is different for different gateways, reference : https://apidocs.chargebee.com/docs/api/customers?prod_cat_ver=2#customer_payment_method_reference_id .
    pub fn find_connector_ids(&self) -> Result<(String, String), errors::ConnectorError> {
        match self.payment_method.gateway {
            ChargebeeGateway::Stripe |  ChargebeeGateway::Braintree => {
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

impl TryFrom<ChargebeeWebhookBody> for hyperswitch_interfaces::recovery::RecoveryPayload{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ChargebeeWebhookBody,
    ) -> Result<Self,Self::Error>{
        let amount = item.content.transaction.as_ref().map_or(item.content.invoice.total.clone(),|trans|trans.amount.clone());
        let currency = item.content.transaction.as_ref().map_or(item.content.invoice.currency_code.clone(),|trans|trans.currency_code.clone());
        let merchant_reference_id = item.content.invoice.id.clone();
        let connector_transaction_id = item.content.transaction.as_ref().map(|trans| trans.id_at_gateway.clone());        
        let error_code = item.content.transaction.as_ref().and_then(|trans| trans.error_code.clone());
        let error_message = item.content.transaction.as_ref().and_then(|trans| trans.error_text.clone());
        let (connector_customer_id, connector_mandate_id) = match &item.content.customer {
            Some(customer) =>{
                let (customer_id,mandate_id) =customer.find_connector_ids()?;
                (Some(customer_id),Some(mandate_id))
            },
            None => (None, None),
        };
        let connector_account_reference_id = item.content.transaction.as_ref().and_then(|trans|trans.gateway_account_id.clone());
        // let created_at = item.content.transaction.as_ref().map(|trans| trans.date.clone());
        let created_at = Some(common_utils::date_time::now());
        Ok(hyperswitch_interfaces::recovery::RecoveryPayload{
            amount,
            currency,
            merchant_reference_id,
            error_code,
            error_message,
            connector_customer_id,
            connector_mandate_id,
            connector_transaction_id,
            connector_account_reference_id,
            created_at
        })
    }
}
