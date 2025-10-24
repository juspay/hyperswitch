#[cfg(feature = "v2")]
use std::str::FromStr;

use common_enums::enums;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use common_utils::id_type;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, types::StringMinorUnit};
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
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{
    router_flow_types::revenue_recovery as recovery_router_flows,
    router_request_types::revenue_recovery as recovery_request_types,
    router_response_types::revenue_recovery as recovery_response_types,
    types as recovery_router_data_types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{convert_uppercase, PaymentsAuthorizeRequestData},
};
pub mod auth_headers {
    pub const STRIPE_API_VERSION: &str = "stripe-version";
    pub const STRIPE_VERSION: &str = "2022-11-15";
}

//TODO: Fill the struct with respective fields
pub struct StripebillingRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for StripebillingRouterData<T> {
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
pub struct StripebillingPaymentsRequest {
    amount: StringMinorUnit,
    card: StripebillingCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StripebillingCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&StripebillingRouterData<&PaymentsAuthorizeRouterData>>
    for StripebillingPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = StripebillingCard {
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
pub struct StripebillingAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for StripebillingAuthType {
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StripebillingPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<StripebillingPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: StripebillingPaymentStatus) -> Self {
        match item {
            StripebillingPaymentStatus::Succeeded => Self::Charged,
            StripebillingPaymentStatus::Failed => Self::Failure,
            StripebillingPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StripebillingPaymentsResponse {
    status: StripebillingPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>,
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
pub struct StripebillingRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&StripebillingRouterData<&RefundsRouterData<F>>> for StripebillingRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, Copy)]
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
pub struct StripebillingErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingWebhookBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingWebhookData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingInvoiceData,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingEventType {
    #[serde(rename = "invoice.paid")]
    PaymentSucceeded,
    #[serde(rename = "invoice.payment_failed")]
    PaymentFailed,
    #[serde(rename = "invoice.voided")]
    InvoiceDeleted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingInvoiceData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
    pub charge: String,
    pub payment_intent: String,
    pub customer_address: Option<StripebillingInvoiceBillingAddress>,
    pub attempt_count: u16,
    pub lines: StripebillingWebhookLinesObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLinesObject {
    pub data: Vec<StripebillingWebhookLinesData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLinesData {
    pub period: StripebillingWebhookLineDataPeriod,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookLineDataPeriod {
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub end: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub start: PrimitiveDateTime,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingInvoiceBillingAddress {
    pub country: Option<enums::CountryAlpha2>,
    pub city: Option<String>,
    pub address_line1: Option<Secret<String>>,
    pub address_line2: Option<Secret<String>>,
    pub zip_code: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
    pub attempt_count: Option<u16>,
}

impl StripebillingWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body: Self = body
            .parse_struct::<Self>("StripebillingWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(webhook_body)
    }
}

impl StripebillingInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("StripebillingInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

impl From<StripebillingInvoiceBillingAddress> for api_models::payments::Address {
    fn from(item: StripebillingInvoiceBillingAddress) -> Self {
        Self {
            address: Some(api_models::payments::AddressDetails::from(item)),
            phone: None,
            email: None,
        }
    }
}

impl From<StripebillingInvoiceBillingAddress> for api_models::payments::AddressDetails {
    fn from(item: StripebillingInvoiceBillingAddress) -> Self {
        Self {
            city: item.city,
            state: item.state,
            country: item.country,
            zip: item.zip_code,
            line1: item.address_line1,
            line2: item.address_line2,
            line3: None,
            first_name: None,
            last_name: None,
            origin_zip: None,
        }
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<StripebillingInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: StripebillingInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            id_type::PaymentReferenceId::from_str(&item.data.object.invoice_id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let next_billing_at = item
            .data
            .object
            .lines
            .data
            .first()
            .map(|linedata| linedata.period.end);
        let billing_started_at = item
            .data
            .object
            .lines
            .data
            .first()
            .map(|linedata| linedata.period.start);
        Ok(Self {
            amount: item.data.object.amount,
            currency: item.data.object.currency,
            merchant_reference_id,
            billing_address: item
                .data
                .object
                .customer_address
                .map(api_models::payments::Address::from),
            retry_count: Some(item.data.object.attempt_count),
            next_billing_at,
            billing_started_at,
            metadata: None,
            // TODO! This field should be handled for billing connnector integrations
            enable_partial_authorization: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripebillingRecoveryDetailsData {
    #[serde(rename = "id")]
    pub charge_id: String,
    pub status: StripebillingChargeStatus,
    pub amount: common_utils::types::MinorUnit,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    pub payment_method: String,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub payment_method_details: StripePaymentMethodDetails,
    #[serde(rename = "invoice")]
    pub invoice_id: String,
    pub payment_intent: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripePaymentMethodDetails {
    #[serde(rename = "type")]
    pub type_of_payment_method: StripebillingPaymentMethod,
    #[serde(rename = "card")]
    pub card_details: StripeBillingCardDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingPaymentMethod {
    Card,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripeBillingCardDetails {
    pub network: StripebillingCardNetwork,
    pub funding: StripebillingFundingTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingCardNetwork {
    Visa,
    Mastercard,
    AmericanExpress,
    JCB,
    DinersClub,
    Discover,
    CartesBancaires,
    UnionPay,
    Interac,
    RuPay,
    Maestro,
    Star,
    Pulse,
    Accel,
    Nyce,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename = "snake_case")]
pub enum StripebillingFundingTypes {
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "debit")]
    Debit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingChargeStatus {
    Succeeded,
    Failed,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
// This is the default hard coded mca Id to find the stripe account associated with the stripe biliing
// Context : Since we dont have the concept of connector_reference_id in stripebilling because payments always go through stripe.
// While creating stripebilling we will hard code the stripe account id to string "stripebilling" in mca featrue metadata. So we have to pass the same as account_reference_id here in response.
const MCA_ID_IDENTIFIER_FOR_STRIPE_IN_STRIPEBILLING_MCA_FEAATURE_METADATA: &str = "stripebilling";

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            recovery_router_flows::BillingConnectorPaymentsSync,
            StripebillingRecoveryDetailsData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    > for recovery_router_data_types::BillingConnectorPaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            recovery_router_flows::BillingConnectorPaymentsSync,
            StripebillingRecoveryDetailsData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let charge_details = item.response;
        let merchant_reference_id =
            id_type::PaymentReferenceId::from_str(charge_details.invoice_id.as_str())
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "invoice_id",
                })?;
        let connector_transaction_id = Some(common_utils::types::ConnectorTransactionId::from(
            charge_details.payment_intent,
        ));

        Ok(Self {
            response: Ok(
                recovery_response_types::BillingConnectorPaymentsSyncResponse {
                    status: charge_details.status.into(),
                    amount: charge_details.amount,
                    currency: charge_details.currency,
                    merchant_reference_id,
                    connector_account_reference_id:
                        MCA_ID_IDENTIFIER_FOR_STRIPE_IN_STRIPEBILLING_MCA_FEAATURE_METADATA
                            .to_string(),
                    connector_transaction_id,
                    error_code: charge_details.failure_code,
                    error_message: charge_details.failure_message,
                    processor_payment_method_token: charge_details.payment_method,
                    connector_customer_id: charge_details.customer,
                    transaction_created_at: Some(charge_details.created),
                    payment_method_sub_type: common_enums::PaymentMethodType::from(
                        charge_details.payment_method_details.card_details.funding,
                    ),
                    payment_method_type: common_enums::PaymentMethod::from(
                        charge_details.payment_method_details.type_of_payment_method,
                    ),
                    // Todo: Fetch Card issuer details. Generally in the other billing connector we are getting card_issuer using the card bin info. But stripe dosent provide any such details. We should find a way for stripe billing case
                    charge_id: Some(charge_details.charge_id.clone()),
                    // Need to populate these card info field
                    card_info: api_models::payments::AdditionalCardInfo {
                        card_network: Some(common_enums::CardNetwork::from(
                            charge_details.payment_method_details.card_details.network,
                        )),
                        card_isin: None,
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
                },
            ),
            ..item.data
        })
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingChargeStatus> for enums::AttemptStatus {
    fn from(status: StripebillingChargeStatus) -> Self {
        match status {
            StripebillingChargeStatus::Succeeded => Self::Charged,
            StripebillingChargeStatus::Failed => Self::Failure,
        }
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingFundingTypes> for common_enums::PaymentMethodType {
    fn from(funding: StripebillingFundingTypes) -> Self {
        match funding {
            StripebillingFundingTypes::Credit => Self::Credit,
            StripebillingFundingTypes::Debit => Self::Debit,
        }
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<StripebillingPaymentMethod> for common_enums::PaymentMethod {
    fn from(method: StripebillingPaymentMethod) -> Self {
        match method {
            StripebillingPaymentMethod::Card => Self::Card,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StripebillingRecordBackResponse {
    pub id: String,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            recovery_router_flows::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            recovery_request_types::InvoiceRecordBackRequest,
            recovery_response_types::InvoiceRecordBackResponse,
        >,
    > for recovery_router_data_types::InvoiceRecordBackRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            recovery_router_flows::InvoiceRecordBack,
            StripebillingRecordBackResponse,
            recovery_request_types::InvoiceRecordBackRequest,
            recovery_response_types::InvoiceRecordBackResponse,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(recovery_response_types::InvoiceRecordBackResponse {
                merchant_reference_id: id_type::PaymentReferenceId::from_str(
                    item.response.id.as_str(),
                )
                .change_context(errors::ConnectorError::MissingRequiredField {
                    field_name: "invoice_id in the response",
                })?,
            }),
            ..item.data
        })
    }
}

impl From<StripebillingCardNetwork> for enums::CardNetwork {
    fn from(item: StripebillingCardNetwork) -> Self {
        match item {
            StripebillingCardNetwork::Visa => Self::Visa,
            StripebillingCardNetwork::Mastercard => Self::Mastercard,
            StripebillingCardNetwork::AmericanExpress => Self::AmericanExpress,
            StripebillingCardNetwork::JCB => Self::JCB,
            StripebillingCardNetwork::DinersClub => Self::DinersClub,
            StripebillingCardNetwork::Discover => Self::Discover,
            StripebillingCardNetwork::CartesBancaires => Self::CartesBancaires,
            StripebillingCardNetwork::UnionPay => Self::UnionPay,
            StripebillingCardNetwork::Interac => Self::Interac,
            StripebillingCardNetwork::RuPay => Self::RuPay,
            StripebillingCardNetwork::Maestro => Self::Maestro,
            StripebillingCardNetwork::Star => Self::Star,
            StripebillingCardNetwork::Pulse => Self::Pulse,
            StripebillingCardNetwork::Accel => Self::Accel,
            StripebillingCardNetwork::Nyce => Self::Nyce,
        }
    }
}
