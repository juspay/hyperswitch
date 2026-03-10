#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use std::str::FromStr;

use api_models::subscription as api;
use common_enums::{connector_enums, enums};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    id_type::{CustomerId, InvoiceId, SubscriptionId},
    pii::{self, Email},
    types::MinorUnit,
};
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{subscriptions::SubscriptionAutoCollection, ResponseId},
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            self, GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionInvoiceData, SubscriptionLineItem, SubscriptionPauseResponse,
            SubscriptionResumeResponse, SubscriptionStatus,
        },
        ConnectorCustomerResponseData, PaymentsResponseData, RefundsResponseData,
    },
    types::{
        GetSubscriptionEstimateRouterData, InvoiceRecordBackRouterData,
        PaymentsAuthorizeRouterData, RefundsRouterData, SubscriptionCancelRouterData,
        SubscriptionPauseRouterData, SubscriptionResumeRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    convert_connector_response_to_domain_response,
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};

// SubscriptionCreate structures
#[derive(Debug, Serialize)]
pub struct ChargebeeSubscriptionCreateRequest {
    #[serde(rename = "id")]
    pub subscription_id: SubscriptionId,
    #[serde(rename = "subscription_items[item_price_id][0]")]
    pub item_price_id: String,
    #[serde(rename = "subscription_items[quantity][0]")]
    pub quantity: Option<u32>,
    #[serde(rename = "billing_address[line1]")]
    pub billing_address_line1: Option<Secret<String>>,
    #[serde(rename = "billing_address[city]")]
    pub billing_address_city: Option<String>,
    #[serde(rename = "billing_address[state]")]
    pub billing_address_state: Option<Secret<String>>,
    #[serde(rename = "billing_address[zip]")]
    pub billing_address_zip: Option<Secret<String>>,
    #[serde(rename = "billing_address[country]")]
    pub billing_address_country: Option<common_enums::CountryAlpha2>,
    pub auto_collection: ChargebeeAutoCollection,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeAutoCollection {
    On,
    Off,
}

impl From<SubscriptionAutoCollection> for ChargebeeAutoCollection {
    fn from(auto_collection: SubscriptionAutoCollection) -> Self {
        match auto_collection {
            SubscriptionAutoCollection::On => Self::On,
            SubscriptionAutoCollection::Off => Self::Off,
        }
    }
}

impl TryFrom<&ChargebeeRouterData<&hyperswitch_domain_models::types::SubscriptionCreateRouterData>>
    for ChargebeeSubscriptionCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&hyperswitch_domain_models::types::SubscriptionCreateRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;

        let first_item =
            req.subscription_items
                .first()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "subscription_items",
                })?;

        Ok(Self {
            subscription_id: req.subscription_id.clone(),
            item_price_id: first_item.item_price_id.clone(),
            quantity: first_item.quantity,
            billing_address_line1: item.router_data.get_optional_billing_line1(),
            billing_address_city: item.router_data.get_optional_billing_city(),
            billing_address_state: item.router_data.get_optional_billing_state(),
            billing_address_zip: item.router_data.get_optional_billing_zip(),
            billing_address_country: item.router_data.get_optional_billing_country(),
            auto_collection: req.auto_collection.clone().into(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeSubscriptionCreateResponse {
    pub subscription: ChargebeeSubscriptionDetails,
    pub invoice: Option<ChargebeeInvoiceData>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeSubscriptionDetails {
    pub id: SubscriptionId,
    pub status: ChargebeeSubscriptionStatus,
    pub customer_id: CustomerId,
    pub currency_code: enums::Currency,
    pub total_dues: Option<MinorUnit>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub pause_date: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    cancelled_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeSubscriptionStatus {
    Future,
    #[serde(rename = "in_trial")]
    InTrial,
    Active,
    #[serde(rename = "non_renewing")]
    NonRenewing,
    Paused,
    Cancelled,
    Transferred,
}

impl From<ChargebeeSubscriptionStatus> for SubscriptionStatus {
    fn from(status: ChargebeeSubscriptionStatus) -> Self {
        match status {
            ChargebeeSubscriptionStatus::Future => Self::Pending,
            ChargebeeSubscriptionStatus::InTrial => Self::Trial,
            ChargebeeSubscriptionStatus::Active => Self::Active,
            ChargebeeSubscriptionStatus::NonRenewing => Self::Onetime,
            ChargebeeSubscriptionStatus::Paused => Self::Paused,
            ChargebeeSubscriptionStatus::Cancelled => Self::Cancelled,
            ChargebeeSubscriptionStatus::Transferred => Self::Cancelled,
        }
    }
}

convert_connector_response_to_domain_response!(
    ChargebeeSubscriptionCreateResponse,
    SubscriptionCreateResponse,
    |item: ResponseRouterData<_, ChargebeeSubscriptionCreateResponse, _, _>| {
        let subscription = &item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionCreateResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                customer_id: subscription.customer_id.clone(),
                currency_code: subscription.currency_code,
                total_amount: subscription.total_dues.unwrap_or(MinorUnit::new(0)),
                next_billing_at: subscription.next_billing_at,
                created_at: subscription.created_at,
                invoice_details: item.response.invoice.map(SubscriptionInvoiceData::from),
            }),
            ..item.data
        })
    }
);

//TODO: Fill the struct with respective fields
pub struct ChargebeeRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for ChargebeeRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct ChargebeePaymentsRequest {
    amount: MinorUnit,
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
                    amount: item.amount,
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Auth Struct
pub struct ChargebeeAuthType {
    pub(super) full_access_key_v1: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChargebeeMetadata {
    pub(super) site: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for ChargebeeMetadata {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl TryFrom<&ConnectorAuthType> for ChargebeeAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                full_access_key_v1: api_key.clone(),
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

convert_connector_response_to_domain_response!(
    ChargebeePaymentsResponse,
    PaymentsResponseData,
    |item: ResponseRouterData<_, ChargebeePaymentsResponse, _, _>| {
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
);

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct ChargebeeRefundRequest {
    pub amount: MinorUnit,
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChargebeeErrorResponse {
    pub api_error_code: String,
    pub message: String,
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
    pub subscription: Option<ChargebeeSubscriptionData>,
}

#[derive(Serialize, Deserialize, Debug)]

pub struct ChargebeeWebhookContent {
    pub transaction: ChargebeeTransactionData,
    pub invoice: ChargebeeInvoiceData,
    pub customer: Option<ChargebeeCustomer>,
    pub subscription: Option<ChargebeeSubscriptionData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeSubscriptionData {
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub current_term_start: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeEventType {
    PaymentSucceeded,
    PaymentFailed,
    InvoiceDeleted,
    InvoiceGenerated,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoiceData {
    // invoice id
    pub id: InvoiceId,
    pub total: MinorUnit,
    pub currency_code: enums::Currency,
    pub status: Option<ChargebeeInvoiceStatus>,
    pub billing_address: Option<ChargebeeInvoiceBillingAddress>,
    pub linked_payments: Option<Vec<ChargebeeInvoicePayments>>,
    pub customer_id: CustomerId,
    pub subscription_id: SubscriptionId,
    pub first_invoice: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoicePayments {
    pub txn_status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChargebeeTransactionData {
    id_at_gateway: Option<String>,
    status: ChargebeeTranasactionStatus,
    error_code: Option<String>,
    error_text: Option<String>,
    gateway_account_id: String,
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
    brand: common_enums::CardNetwork,
    iin: String,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChargebeeInvoiceBillingAddress {
    pub line1: Option<Secret<String>>,
    pub line2: Option<Secret<String>>,
    pub line3: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub country: Option<enums::CountryAlpha2>,
    pub zip: Option<Secret<String>>,
    pub city: Option<String>,
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
// Structure to extract MIT payment data from invoice_generated webhook
#[derive(Debug, Clone)]
pub struct ChargebeeMitPaymentData {
    pub invoice_id: InvoiceId,
    pub amount_due: MinorUnit,
    pub currency_code: enums::Currency,
    pub status: Option<ChargebeeInvoiceStatus>,
    pub customer_id: CustomerId,
    pub subscription_id: SubscriptionId,
    pub first_invoice: bool,
}

impl TryFrom<ChargebeeInvoiceBody> for ChargebeeMitPaymentData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(webhook_body: ChargebeeInvoiceBody) -> Result<Self, Self::Error> {
        let invoice = webhook_body.content.invoice;

        Ok(Self {
            invoice_id: invoice.id,
            amount_due: invoice.total,
            currency_code: invoice.currency_code,
            status: invoice.status,
            customer_id: invoice.customer_id,
            subscription_id: invoice.subscription_id,
            first_invoice: invoice.first_invoice.unwrap_or(false),
        })
    }
}
pub struct ChargebeeMandateDetails {
    pub customer_id: String,
    pub mandate_id: String,
}

impl ChargebeeCustomer {
    // the logic to find connector customer id & mandate id is different for different gateways, reference : https://apidocs.chargebee.com/docs/api/customers?prod_cat_ver=2#customer_payment_method_reference_id .
    pub fn find_connector_ids(&self) -> Result<ChargebeeMandateDetails, errors::ConnectorError> {
        match self.payment_method.gateway {
            ChargebeeGateway::Stripe | ChargebeeGateway::Braintree => {
                let mut parts = self.payment_method.reference_id.split('/');
                let customer_id = parts
                    .next()
                    .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?
                    .to_string();
                let mandate_id = parts
                    .next_back()
                    .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?
                    .to_string();
                Ok(ChargebeeMandateDetails {
                    customer_id,
                    mandate_id,
                })
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
        let merchant_reference_id = common_utils::id_type::PaymentReferenceId::from_str(
            item.content.invoice.id.get_string_repr(),
        )
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let connector_transaction_id = item
            .content
            .transaction
            .id_at_gateway
            .map(common_utils::types::ConnectorTransactionId::TxnId);
        let error_code = item.content.transaction.error_code.clone();
        let error_message = item.content.transaction.error_text.clone();
        let connector_mandate_details = item
            .content
            .customer
            .as_ref()
            .map(|customer| customer.find_connector_ids())
            .transpose()?
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_mandate_details",
            })?;
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
        // Chargebee retry count will always be less than u16 always. Chargebee can have maximum 12 retry attempts
        #[allow(clippy::as_conversions)]
        let retry_count = item
            .content
            .invoice
            .linked_payments
            .map(|linked_payments| linked_payments.len() as u16);
        let invoice_next_billing_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.next_billing_at);
        let invoice_billing_started_at_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.current_term_start);
        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            connector_transaction_id,
            error_code,
            error_message,
            processor_payment_method_token: connector_mandate_details.mandate_id,
            connector_customer_id: connector_mandate_details.customer_id,
            connector_account_reference_id,
            transaction_created_at,
            status,
            payment_method_type,
            payment_method_sub_type,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            retry_count,
            invoice_next_billing_time,
            invoice_billing_started_at_time,
            // This field is none because it is specific to stripebilling.
            charge_id: None,
            // Need to populate these card info field
            card_info: api_models::payments::AdditionalCardInfo {
                card_network: Some(payment_method_details.card.brand),
                card_isin: Some(payment_method_details.card.iin),
                card_issuer: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                last4: None,
                card_extended_bin: None,
                card_exp_month: None,
                card_exp_year: None,
                card_holder_name: None,
                payment_checks: None,
                authentication_data: None,
                is_regulated: None,
                signature_network: None,
            },
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
            | ChargebeeTranasactionStatus::LateFailure => Self::Failure,
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
            ChargebeeEventType::InvoiceGenerated => Self::InvoiceGenerated,
        }
    }
}

#[cfg(feature = "v1")]
impl From<ChargebeeEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(event: ChargebeeEventType) -> Self {
        match event {
            ChargebeeEventType::PaymentSucceeded => Self::PaymentIntentSuccess,
            ChargebeeEventType::PaymentFailed => Self::PaymentIntentFailure,
            ChargebeeEventType::InvoiceDeleted => Self::EventNotSupported,
            ChargebeeEventType::InvoiceGenerated => Self::InvoiceGenerated,
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<ChargebeeInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: ChargebeeInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id = common_utils::id_type::PaymentReferenceId::from_str(
            item.content.invoice.id.get_string_repr(),
        )
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        // The retry count will never exceed u16 limit in a billing connector. It can have maximum of 12 in case of charge bee so its ok to suppress this
        #[allow(clippy::as_conversions)]
        let retry_count = item
            .content
            .invoice
            .linked_payments
            .as_ref()
            .map(|linked_payments| linked_payments.len() as u16);
        let invoice_next_billing_time = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.next_billing_at);
        let billing_started_at = item
            .content
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.current_term_start);
        Ok(Self {
            amount: item.content.invoice.total,
            currency: item.content.invoice.currency_code,
            merchant_reference_id,
            billing_address: Some(api_models::payments::Address::from(item.content.invoice)),
            retry_count,
            next_billing_at: invoice_next_billing_time,
            billing_started_at,
            metadata: None,
            // TODO! This field should be handled for billing connnector integrations
            enable_partial_authorization: None,
        })
    }
}

impl From<ChargebeeInvoiceData> for api_models::payments::Address {
    fn from(item: ChargebeeInvoiceData) -> Self {
        Self {
            address: item
                .billing_address
                .map(api_models::payments::AddressDetails::from),
            phone: None,
            email: None,
        }
    }
}

impl From<ChargebeeInvoiceBillingAddress> for api_models::payments::AddressDetails {
    fn from(item: ChargebeeInvoiceBillingAddress) -> Self {
        Self {
            city: item.city,
            country: item.country,
            state: item.state,
            zip: item.zip,
            line1: item.line1,
            line2: item.line2,
            line3: item.line3,
            first_name: None,
            last_name: None,
            origin_zip: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ChargebeeRecordPaymentRequest {
    #[serde(rename = "transaction[amount]")]
    pub amount: MinorUnit,
    #[serde(rename = "transaction[payment_method]")]
    pub payment_method: ChargebeeRecordPaymentMethod,
    #[serde(rename = "transaction[id_at_gateway]")]
    pub connector_payment_id: Option<String>,
    #[serde(rename = "transaction[status]")]
    pub status: ChargebeeRecordStatus,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeRecordPaymentMethod {
    Other,
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeRecordStatus {
    Success,
    Failure,
}

impl TryFrom<&ChargebeeRouterData<&InvoiceRecordBackRouterData>> for ChargebeeRecordPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ChargebeeRouterData<&InvoiceRecordBackRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;
        Ok(Self {
            amount: req.amount,
            payment_method: ChargebeeRecordPaymentMethod::Other,
            connector_payment_id: req
                .connector_transaction_id
                .as_ref()
                .map(|connector_payment_id| connector_payment_id.get_id().to_string()),
            status: ChargebeeRecordStatus::try_from(req.attempt_status)?,
        })
    }
}

impl TryFrom<enums::AttemptStatus> for ChargebeeRecordStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(status: enums::AttemptStatus) -> Result<Self, Self::Error> {
        match status {
            enums::AttemptStatus::Charged
            | enums::AttemptStatus::PartialCharged
            | enums::AttemptStatus::PartialChargedAndChargeable => Ok(Self::Success),
            enums::AttemptStatus::Failure
            | enums::AttemptStatus::CaptureFailed
            | enums::AttemptStatus::RouterDeclined => Ok(Self::Failure),
            enums::AttemptStatus::AuthenticationFailed
            | enums::AttemptStatus::Started
            | enums::AttemptStatus::AuthenticationPending
            | enums::AttemptStatus::AuthenticationSuccessful
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::PartiallyAuthorized
            | enums::AttemptStatus::AuthorizationFailed
            | enums::AttemptStatus::Authorizing
            | enums::AttemptStatus::CodInitiated
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::VoidedPostCharge
            | enums::AttemptStatus::VoidInitiated
            | enums::AttemptStatus::CaptureInitiated
            | enums::AttemptStatus::VoidFailed
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::Unresolved
            | enums::AttemptStatus::Pending
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::ConfirmationAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending
            | enums::AttemptStatus::IntegrityFailure
            | enums::AttemptStatus::Expired => Err(errors::ConnectorError::NotSupported {
                message: "Record back flow is only supported for terminal status".to_string(),
                connector: "chargebee",
            }
            .into()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeRecordbackResponse {
    pub invoice: ChargebeeRecordbackInvoice,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeRecordbackInvoice {
    pub id: common_utils::id_type::PaymentReferenceId,
}

convert_connector_response_to_domain_response!(
    ChargebeeRecordbackResponse,
    InvoiceRecordBackResponse,
    |item: ResponseRouterData<_, ChargebeeRecordbackResponse, _, _>| {
        let merchant_reference_id = item.response.invoice.id;
        Ok(Self {
            response: Ok(InvoiceRecordBackResponse {
                merchant_reference_id,
            }),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeListPlansResponse {
    pub list: Vec<ChargebeeItemList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeItemList {
    pub item: ChargebeeItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub plan_type: String,
    pub is_giftable: bool,
    pub enabled_for_checkout: bool,
    pub enabled_in_portal: bool,
    pub metered: bool,
    pub deleted: bool,
    pub description: Option<String>,
}

convert_connector_response_to_domain_response!(
    SubscriptionEstimateResponse,
    GetSubscriptionEstimateResponse,
    |item: ResponseRouterData<_, SubscriptionEstimateResponse, _, _>| {
        let estimate = item.response.estimate;
        Ok(Self {
            response: Ok(GetSubscriptionEstimateResponse {
                sub_total: estimate.invoice_estimate.sub_total,
                total: estimate.invoice_estimate.total,
                amount_paid: Some(estimate.invoice_estimate.amount_paid),
                amount_due: Some(estimate.invoice_estimate.amount_due),
                currency: estimate.subscription_estimate.currency_code,
                next_billing_at: estimate.subscription_estimate.next_billing_at,
                credits_applied: Some(estimate.invoice_estimate.credits_applied),
                customer_id: Some(estimate.invoice_estimate.customer_id),
                line_items: estimate
                    .invoice_estimate
                    .line_items
                    .into_iter()
                    .map(|line_item| SubscriptionLineItem {
                        item_id: line_item.entity_id,
                        item_type: line_item.entity_type,
                        description: line_item.description,
                        amount: line_item.amount,
                        currency: estimate.invoice_estimate.currency_code,
                        unit_amount: Some(line_item.unit_amount),
                        quantity: line_item.quantity,
                        pricing_model: Some(line_item.pricing_model),
                    })
                    .collect(),
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeListPlansResponse,
    GetSubscriptionPlansResponse,
    |item: ResponseRouterData<_, ChargebeeListPlansResponse, _, _>| {
        let plans = item
            .response
            .list
            .into_iter()
            .map(|plan| subscriptions::SubscriptionPlans {
                subscription_provider_plan_id: plan.item.id,
                name: plan.item.name,
                description: plan.item.description,
            })
            .collect();
        Ok(Self {
            response: Ok(GetSubscriptionPlansResponse { list: plans }),
            ..item.data
        })
    }
);

#[derive(Debug, Serialize)]
pub struct ChargebeeCustomerCreateRequest {
    #[serde(rename = "id")]
    pub customer_id: CustomerId,
    #[serde(rename = "first_name")]
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    #[serde(rename = "billing_address[first_name]")]
    pub billing_address_first_name: Option<Secret<String>>,
    #[serde(rename = "billing_address[last_name]")]
    pub billing_address_last_name: Option<Secret<String>>,
    #[serde(rename = "billing_address[line1]")]
    pub billing_address_line1: Option<Secret<String>>,
    #[serde(rename = "billing_address[city]")]
    pub billing_address_city: Option<String>,
    #[serde(rename = "billing_address[state]")]
    pub billing_address_state: Option<Secret<String>>,
    #[serde(rename = "billing_address[zip]")]
    pub billing_address_zip: Option<Secret<String>>,
    #[serde(rename = "billing_address[country]")]
    pub billing_address_country: Option<String>,
}

impl TryFrom<&ChargebeeRouterData<&hyperswitch_domain_models::types::ConnectorCustomerRouterData>>
    for ChargebeeCustomerCreateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &ChargebeeRouterData<&hyperswitch_domain_models::types::ConnectorCustomerRouterData>,
    ) -> Result<Self, Self::Error> {
        let req = &item.router_data.request;

        Ok(Self {
            customer_id: req
                .customer_id
                .as_ref()
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                })?
                .clone(),
            name: req.name.clone(),
            email: req.email.clone(),
            billing_address_first_name: req
                .billing_address
                .as_ref()
                .and_then(|address| address.first_name.clone()),
            billing_address_last_name: req
                .billing_address
                .as_ref()
                .and_then(|address| address.last_name.clone()),
            billing_address_line1: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.line1.clone()),
            billing_address_city: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.city.clone()),
            billing_address_country: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.country.map(|country| country.to_string())),
            billing_address_state: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.state.clone()),
            billing_address_zip: req
                .billing_address
                .as_ref()
                .and_then(|addr| addr.zip.clone()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCustomerCreateResponse {
    pub customer: ChargebeeCustomerDetails,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCustomerDetails {
    pub id: String,
    #[serde(rename = "first_name")]
    pub name: Option<Secret<String>>,
    pub email: Option<Email>,
    pub billing_address: Option<api_models::payments::AddressDetails>,
}

convert_connector_response_to_domain_response!(
    ChargebeeCustomerCreateResponse,
    PaymentsResponseData,
    |item: ResponseRouterData<_, ChargebeeCustomerCreateResponse, _, _>| {
        let customer_response = &item.response.customer;

        Ok(Self {
            response: Ok(PaymentsResponseData::ConnectorCustomerResponse(
                ConnectorCustomerResponseData::new(
                    customer_response.id.clone(),
                    customer_response
                        .name
                        .as_ref()
                        .map(|name| name.clone().expose()),
                    customer_response
                        .email
                        .as_ref()
                        .map(|email| email.clone().expose().expose()),
                    customer_response.billing_address.clone(),
                ),
            )),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeSubscriptionEstimateRequest {
    #[serde(rename = "subscription_items[item_price_id][0]")]
    pub price_id: String,
}

impl TryFrom<&GetSubscriptionEstimateRouterData> for ChargebeeSubscriptionEstimateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &GetSubscriptionEstimateRouterData) -> Result<Self, Self::Error> {
        let price_id = item.request.price_id.to_owned();
        Ok(Self { price_id })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeGetPlanPricesResponse {
    pub list: Vec<ChargebeeGetPlanPriceList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeGetPlanPriceList {
    pub item_price: ChargebeePlanPriceItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeePlanPriceItem {
    pub id: String,
    pub name: String,
    pub currency_code: common_enums::Currency,
    pub free_quantity: i64,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub created_at: Option<PrimitiveDateTime>,
    pub deleted: bool,
    pub item_id: Option<String>,
    pub period: i64,
    pub period_unit: ChargebeePeriodUnit,
    pub trial_period: Option<i64>,
    pub trial_period_unit: Option<ChargebeeTrialPeriodUnit>,
    pub price: MinorUnit,
    pub pricing_model: ChargebeePricingModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeePricingModel {
    FlatFee,
    PerUnit,
    Tiered,
    Volume,
    Stairstep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeePeriodUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeTrialPeriodUnit {
    Day,
    Month,
}

convert_connector_response_to_domain_response!(
    ChargebeeGetPlanPricesResponse,
    GetSubscriptionPlanPricesResponse,
    |item: ResponseRouterData<_, ChargebeeGetPlanPricesResponse, _, _>| {
        let plan_prices = item
            .response
            .list
            .into_iter()
            .map(|prices| subscriptions::SubscriptionPlanPrices {
                price_id: prices.item_price.id,
                plan_id: prices.item_price.item_id,
                amount: prices.item_price.price,
                currency: prices.item_price.currency_code,
                interval: match prices.item_price.period_unit {
                    ChargebeePeriodUnit::Day => subscriptions::PeriodUnit::Day,
                    ChargebeePeriodUnit::Week => subscriptions::PeriodUnit::Week,
                    ChargebeePeriodUnit::Month => subscriptions::PeriodUnit::Month,
                    ChargebeePeriodUnit::Year => subscriptions::PeriodUnit::Year,
                },
                interval_count: prices.item_price.period,
                trial_period: prices.item_price.trial_period,
                trial_period_unit: match prices.item_price.trial_period_unit {
                    Some(ChargebeeTrialPeriodUnit::Day) => Some(subscriptions::PeriodUnit::Day),
                    Some(ChargebeeTrialPeriodUnit::Month) => Some(subscriptions::PeriodUnit::Month),
                    None => None,
                },
            })
            .collect();
        Ok(Self {
            response: Ok(GetSubscriptionPlanPricesResponse { list: plan_prices }),
            ..item.data
        })
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEstimateResponse {
    pub estimate: ChargebeeEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargebeeEstimate {
    pub created_at: i64,
    /// type of the object will be `estimate`
    pub object: String,
    pub subscription_estimate: SubscriptionEstimate,
    pub invoice_estimate: InvoiceEstimate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionEstimate {
    pub status: String,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub next_billing_at: Option<PrimitiveDateTime>,
    /// type of the object will be `subscription_estimate`
    pub object: String,
    pub currency_code: enums::Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceEstimate {
    pub recurring: bool,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date: Option<PrimitiveDateTime>,
    pub price_type: String,
    pub sub_total: MinorUnit,
    pub total: MinorUnit,
    pub credits_applied: MinorUnit,
    pub amount_paid: MinorUnit,
    pub amount_due: MinorUnit,
    /// type of the object will be `invoice_estimate`
    pub object: String,
    pub customer_id: CustomerId,
    pub line_items: Vec<LineItem>,
    pub currency_code: enums::Currency,
    pub round_off_amount: MinorUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub id: String,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date_from: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::timestamp::option")]
    pub date_to: Option<PrimitiveDateTime>,
    pub unit_amount: MinorUnit,
    pub quantity: i64,
    pub amount: MinorUnit,
    pub pricing_model: String,
    pub is_taxed: bool,
    pub tax_amount: MinorUnit,
    /// type of the object will be `line_item`
    pub object: String,
    pub customer_id: String,
    pub description: String,
    pub entity_type: String,
    pub entity_id: String,
    pub discount_amount: MinorUnit,
    pub item_level_discount_amount: MinorUnit,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ChargebeeInvoiceStatus {
    Paid,
    Posted,
    PaymentDue,
    NotPaid,
    Voided,
    #[serde(other)]
    Pending,
}

impl From<ChargebeeInvoiceData> for SubscriptionInvoiceData {
    fn from(item: ChargebeeInvoiceData) -> Self {
        Self {
            billing_address: Some(api_models::payments::Address::from(item.clone())),
            id: item.id,
            total: item.total,
            currency_code: item.currency_code,
            status: item.status.map(connector_enums::InvoiceStatus::from),
        }
    }
}

impl From<ChargebeeInvoiceStatus> for connector_enums::InvoiceStatus {
    fn from(status: ChargebeeInvoiceStatus) -> Self {
        match status {
            ChargebeeInvoiceStatus::Paid => Self::InvoicePaid,
            ChargebeeInvoiceStatus::Posted => Self::PaymentPendingTimeout,
            ChargebeeInvoiceStatus::PaymentDue => Self::PaymentPending,
            ChargebeeInvoiceStatus::NotPaid => Self::PaymentFailed,
            ChargebeeInvoiceStatus::Voided => Self::Voided,
            ChargebeeInvoiceStatus::Pending => Self::InvoiceCreated,
        }
    }
}

// Pause Subscription structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeePauseSubscriptionRequest {
    #[serde(rename = "pause_option")]
    pub pause_option: Option<api::PauseOption>,
    #[serde(rename = "resume_date", skip_serializing_if = "Option::is_none")]
    pub resume_date: Option<i64>,
}

impl From<&SubscriptionPauseRouterData> for ChargebeePauseSubscriptionRequest {
    fn from(req: &SubscriptionPauseRouterData) -> Self {
        Self {
            pause_option: req.request.pause_option.clone(),
            resume_date: req
                .request
                .pause_date
                .map(|date| date.assume_utc().unix_timestamp()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeePauseSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

// Resume Subscription structures
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeeResumeSubscriptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_option: Option<api::ResumeOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_date: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charges_handling: Option<api::ChargesHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpaid_invoices_handling: Option<api::UnpaidInvoicesHandling>,
}

impl From<&SubscriptionResumeRouterData> for ChargebeeResumeSubscriptionRequest {
    fn from(req: &SubscriptionResumeRouterData) -> Self {
        Self {
            resume_option: req.request.resume_option.clone(),
            resume_date: req
                .request
                .resume_date
                .map(|date| date.assume_utc().unix_timestamp()),
            charges_handling: req.request.charges_handling.clone(),
            unpaid_invoices_handling: req.request.unpaid_invoices_handling.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeResumeSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

// Cancel Subscription structures
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ChargebeeCancelSubscriptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_option: Option<api::CancelOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unbilled_charges_option: Option<api::UnbilledChargesOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_option_for_current_term_charges: Option<api::CreditOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_receivables_handling: Option<api::AccountReceivablesHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refundable_credits_handling: Option<api::RefundableCreditsHandling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_reason_code: Option<String>,
}

impl From<&SubscriptionCancelRouterData> for ChargebeeCancelSubscriptionRequest {
    fn from(req: &SubscriptionCancelRouterData) -> Self {
        Self {
            cancel_at: req
                .request
                .cancel_date
                .map(|date| date.assume_utc().unix_timestamp()),
            cancel_option: req.request.cancel_option.clone(),
            unbilled_charges_option: req.request.unbilled_charges_option.clone(),
            credit_option_for_current_term_charges: req
                .request
                .credit_option_for_current_term_charges
                .clone(),
            account_receivables_handling: req.request.account_receivables_handling.clone(),
            refundable_credits_handling: req.request.refundable_credits_handling.clone(),
            cancel_reason_code: req.request.cancel_reason_code.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChargebeeCancelSubscriptionResponse {
    pub subscription: ChargebeeSubscriptionDetails,
}

convert_connector_response_to_domain_response!(
    ChargebeePauseSubscriptionResponse,
    SubscriptionPauseResponse,
    |item: ResponseRouterData<_, ChargebeePauseSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionPauseResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                paused_at: subscription.pause_date,
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeResumeSubscriptionResponse,
    SubscriptionResumeResponse,
    |item: ResponseRouterData<_, ChargebeeResumeSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionResumeResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                next_billing_at: subscription.next_billing_at,
            }),
            ..item.data
        })
    }
);

convert_connector_response_to_domain_response!(
    ChargebeeCancelSubscriptionResponse,
    SubscriptionCancelResponse,
    |item: ResponseRouterData<_, ChargebeeCancelSubscriptionResponse, _, _>| {
        let subscription = item.response.subscription;
        Ok(Self {
            response: Ok(SubscriptionCancelResponse {
                subscription_id: subscription.id.clone(),
                status: subscription.status.clone().into(),
                cancelled_at: subscription.cancelled_at,
            }),
            ..item.data
        })
    }
);
