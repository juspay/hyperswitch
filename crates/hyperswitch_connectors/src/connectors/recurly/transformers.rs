#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use std::str::FromStr;

use common_enums::enums;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use common_utils::types::{ConnectorTransactionId, FloatMajorUnitForConnector};
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    id_type,
    types::{FloatMajorUnit, StringMinorUnit},
};
use error_stack::ResultExt;
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
    router_data_v2::flow_common_types as recovery_flow_common_types,
    router_flow_types::revenue_recovery as recovery_router_flows,
    router_request_types::revenue_recovery as recovery_request_types,
    router_response_types::revenue_recovery as recovery_response_types,
    types as recovery_router_data_types,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use crate::utils;
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData, ResponseRouterDataV2},
    utils::PaymentsAuthorizeRequestData,
};

//TODO: Fill the struct with respective fields
pub struct RecurlyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for RecurlyRouterData<T> {
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
pub struct RecurlyPaymentsRequest {
    amount: StringMinorUnit,
    card: RecurlyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct RecurlyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&RecurlyRouterData<&PaymentsAuthorizeRouterData>> for RecurlyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RecurlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = RecurlyCard {
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
pub struct RecurlyAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RecurlyAuthType {
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
pub enum RecurlyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RecurlyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: RecurlyPaymentStatus) -> Self {
        match item {
            RecurlyPaymentStatus::Succeeded => Self::Charged,
            RecurlyPaymentStatus::Failed => Self::Failure,
            RecurlyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurlyPaymentsResponse {
    status: RecurlyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>,
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
pub struct RecurlyRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&RecurlyRouterData<&RefundsRouterData<F>>> for RecurlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RecurlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct RecurlyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecurlyWebhookBody {
    // Transaction uuid
    pub uuid: String,
    pub event_type: RecurlyPaymentEventType,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum RecurlyPaymentEventType {
    #[serde(rename = "succeeded")]
    PaymentSucceeded,
    #[serde(rename = "failed")]
    PaymentFailed,
}

impl RecurlyWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("RecurlyWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum RecurlyChargeStatus {
    #[serde(rename = "success")]
    Succeeded,
    #[serde(rename = "declined")]
    Failed,
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyFundingTypes {
    Credit,
    Debit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyPaymentObject {
    CreditCard,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecurlyRecoveryDetailsData {
    pub amount: FloatMajorUnit,
    pub currency: common_enums::Currency,
    pub id: String,
    pub status_code: Option<String>,
    pub status_message: Option<String>,
    pub account: Account,
    pub invoice: Invoice,
    pub payment_method: PaymentMethod,
    pub payment_gateway: PaymentGateway,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    pub status: RecurlyChargeStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub gateway_token: String,
    pub funding_source: RecurlyFundingTypes,
    pub object: RecurlyPaymentObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentGateway {
    pub id: String,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterDataV2<
            recovery_router_flows::BillingConnectorPaymentsSync,
            RecurlyRecoveryDetailsData,
            recovery_flow_common_types::BillingConnectorPaymentsSyncFlowData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    > for recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterDataV2<
            recovery_router_flows::BillingConnectorPaymentsSync,
            RecurlyRecoveryDetailsData,
            recovery_flow_common_types::BillingConnectorPaymentsSyncFlowData,
            recovery_request_types::BillingConnectorPaymentsSyncRequest,
            recovery_response_types::BillingConnectorPaymentsSyncResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            id_type::PaymentReferenceId::from_str(&item.response.invoice.id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let connector_transaction_id = Some(ConnectorTransactionId::from(item.response.id));

        Ok(Self {
            response: Ok(
                recovery_response_types::BillingConnectorPaymentsSyncResponse {
                    status: item.response.status.into(),
                    amount: utils::convert_back_amount_to_minor_units(
                        &FloatMajorUnitForConnector,
                        item.response.amount,
                        item.response.currency,
                    )?,
                    currency: item.response.currency,
                    merchant_reference_id,
                    connector_account_reference_id: item.response.payment_gateway.id,
                    connector_transaction_id,
                    error_code: item.response.status_code,
                    error_message: item.response.status_message,
                    processor_payment_method_token: item.response.payment_method.gateway_token,
                    connector_customer_id: item.response.account.id,
                    transaction_created_at: Some(item.response.created_at),
                    payment_method_sub_type: common_enums::PaymentMethodType::from(
                        item.response.payment_method.funding_source,
                    ),
                    payment_method_type: common_enums::PaymentMethod::from(
                        item.response.payment_method.object,
                    ),
                },
            ),
            ..item.data
        })
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<RecurlyChargeStatus> for enums::AttemptStatus {
    fn from(status: RecurlyChargeStatus) -> Self {
        match status {
            RecurlyChargeStatus::Succeeded => Self::Charged,
            RecurlyChargeStatus::Failed => Self::Failure,
        }
    }
}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<RecurlyFundingTypes> for common_enums::PaymentMethodType {
    fn from(funding: RecurlyFundingTypes) -> Self {
        match funding {
            RecurlyFundingTypes::Credit => Self::Credit,
            RecurlyFundingTypes::Debit => Self::Debit,
        }
    }
}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl From<RecurlyPaymentObject> for common_enums::PaymentMethod {
    fn from(funding: RecurlyPaymentObject) -> Self {
        match funding {
            RecurlyPaymentObject::CreditCard => Self::Card,
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyRecordStatus {
    Success,
    Failure,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl TryFrom<enums::AttemptStatus> for RecurlyRecordStatus {
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
            | enums::AttemptStatus::AuthorizationFailed
            | enums::AttemptStatus::Authorizing
            | enums::AttemptStatus::CodInitiated
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::VoidInitiated
            | enums::AttemptStatus::CaptureInitiated
            | enums::AttemptStatus::VoidFailed
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::Unresolved
            | enums::AttemptStatus::Pending
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::ConfirmationAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Record back flow is only supported for terminal status".to_string(),
                    connector: "recurly",
                }
                .into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecurlyRecordBackResponse {
    // Invoice id
    pub id: id_type::PaymentReferenceId,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterDataV2<
            recovery_router_flows::RecoveryRecordBack,
            RecurlyRecordBackResponse,
            recovery_flow_common_types::RevenueRecoveryRecordBackData,
            recovery_request_types::RevenueRecoveryRecordBackRequest,
            recovery_response_types::RevenueRecoveryRecordBackResponse,
        >,
    > for recovery_router_data_types::RevenueRecoveryRecordBackRouterDataV2
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterDataV2<
            recovery_router_flows::RecoveryRecordBack,
            RecurlyRecordBackResponse,
            recovery_flow_common_types::RevenueRecoveryRecordBackData,
            recovery_request_types::RevenueRecoveryRecordBackRequest,
            recovery_response_types::RevenueRecoveryRecordBackResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let merchant_reference_id = item.response.id;
        Ok(Self {
            response: Ok(recovery_response_types::RevenueRecoveryRecordBackResponse {
                merchant_reference_id,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecurlyInvoiceSyncResponse {
    pub id: String,
    pub total: FloatMajorUnit,
    pub currency: common_enums::Currency,
    pub address: Option<RecurlyInvoiceBillingAdddress>,
    pub line_items: Vec<RecurlyLineItems>,
    pub transactions: Vec<RecurlyInvoiceTransactionsStatus>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecurlyInvoiceBillingAdddress {
    pub street1: Option<Secret<String>>,
    pub street2: Option<Secret<String>>,
    pub region: Option<Secret<String>>,
    pub country: Option<enums::CountryAlpha2>,
    pub postal_code: Option<Secret<String>>,
    pub city: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecurlyLineItems {
    #[serde(rename = "type")]
    pub invoice_type: RecurlyInvoiceLineItemType,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub start_date: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub end_date: PrimitiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyInvoiceLineItemType {
    Credit,
    Charge,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RecurlyInvoiceTransactionsStatus {
    pub status: String,
}

impl
    TryFrom<
        ResponseRouterDataV2<
            recovery_router_flows::BillingConnectorInvoiceSync,
            RecurlyInvoiceSyncResponse,
            recovery_flow_common_types::BillingConnectorInvoiceSyncFlowData,
            recovery_request_types::BillingConnectorInvoiceSyncRequest,
            recovery_response_types::BillingConnectorInvoiceSyncResponse,
        >,
    > for recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterDataV2<
            recovery_router_flows::BillingConnectorInvoiceSync,
            RecurlyInvoiceSyncResponse,
            recovery_flow_common_types::BillingConnectorInvoiceSyncFlowData,
            recovery_request_types::BillingConnectorInvoiceSyncRequest,
            recovery_response_types::BillingConnectorInvoiceSyncResponse,
        >,
    ) -> Result<Self, Self::Error> {
        #[allow(clippy::as_conversions)]
        // No of retries never exceeds u16 in recurly. So its better to supress the clippy warning
        let retry_count = item.response.transactions.len() as u16;
        let merchant_reference_id = id_type::PaymentReferenceId::from_str(&item.response.id)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Self {
            response: Ok(
                recovery_response_types::BillingConnectorInvoiceSyncResponse {
                    amount: utils::convert_back_amount_to_minor_units(
                        &FloatMajorUnitForConnector,
                        item.response.total,
                        item.response.currency,
                    )?,
                    currency: item.response.currency,
                    merchant_reference_id,
                    retry_count: Some(retry_count),
                    billing_address: Some(api_models::payments::Address {
                        address: Some(api_models::payments::AddressDetails {
                            city: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.city),
                            state: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.region),
                            country: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.country),
                            line1: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.street1),
                            line2: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.street2),
                            line3: None,
                            zip: item
                                .response
                                .address
                                .clone()
                                .and_then(|address| address.postal_code),
                            first_name: None,
                            last_name: None,
                        }),
                        phone: None,
                        email: None,
                    }),
                    created_at: item.response.line_items.first().map(|line| line.start_date),
                    ends_at: item.response.line_items.first().map(|line| line.end_date),
                },
            ),
            ..item.data
        })
    }
}
