use common_enums::enums;
use common_utils::{pii, request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsSyncRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{self, RefundsResponseRouterData},
    utils::{PaymentsAuthorizeRequestData, PaymentsSyncRequestData, RouterData as _},
};

const NO_REFUND_REASON: &str = "No reason provided";

//TODO: Fill the struct with respective fields
pub struct PayjustnowRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PayjustnowRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowPaymentsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    payjustnow: PayjustnowRequest,
    checkout_total_cents: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowRequest {
    merchant_order_reference: String,
    order_amount_cents: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    order_items: Option<Vec<OrderItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer: Option<Customer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_address: Option<Address>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_address: Option<Address>,
    confirm_redirect_url: String,
    cancel_redirect_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderItem {
    name: String,
    sku: String,
    quantity: u32,
    price_cents: MinorUnit,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    #[serde(skip_serializing_if = "Option::is_none")]
    first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_name: Option<Secret<String>>,
    email: pii::Email,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone_number: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    address_line1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address_line2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    province: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<Secret<String>>,
}

impl Address {
    fn is_empty(&self) -> bool {
        self.address_line1.is_none()
            && self.address_line2.is_none()
            && self.city.is_none()
            && self.province.is_none()
            && self.postal_code.is_none()
    }
}

impl TryFrom<&PayjustnowRouterData<&PaymentsAuthorizeRouterData>> for PayjustnowPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayjustnowRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = item.router_data;
        let order_items = router_data
            .request
            .order_details
            .as_ref()
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|order| {
                        Ok(OrderItem {
                            name: order.product_name.clone(),
                            sku: order.product_id.clone().unwrap_or_default(),
                            quantity: u32::from(order.quantity),
                            price_cents: order.amount,
                        })
                    })
                    .collect::<Result<Vec<OrderItem>, errors::ConnectorError>>()
            })
            .transpose()?;

        let customer = router_data
            .get_optional_billing_email()
            .or_else(|| item.router_data.request.email.clone())
            .map(|email| Customer {
                first_name: router_data.get_optional_billing_first_name(),
                last_name: router_data.get_optional_billing_last_name(),
                email,
                phone_number: router_data.get_optional_billing_phone_number(),
            });

        let billing_address = {
            let addr = Address {
                address_line1: router_data.get_optional_billing_line1(),
                address_line2: router_data.get_optional_billing_line2(),
                city: router_data.get_optional_billing_city(),
                province: router_data.get_optional_billing_state(),
                postal_code: item.router_data.get_optional_billing_zip(),
            };

            if addr.is_empty() {
                None
            } else {
                Some(addr)
            }
        };

        let shipping_address = {
            let addr = Address {
                address_line1: item.router_data.get_optional_shipping_line1(),
                address_line2: item.router_data.get_optional_shipping_line2(),
                city: item.router_data.get_optional_shipping_city(),
                province: item.router_data.get_optional_shipping_state(),
                postal_code: item.router_data.get_optional_shipping_zip(),
            };

            if addr.is_empty() {
                None
            } else {
                Some(addr)
            }
        };

        let router_return_url = item.router_data.request.get_router_return_url()?;

        let payjustnow_request = PayjustnowRequest {
            merchant_order_reference: item
                .router_data
                .request
                .merchant_order_reference_id
                .clone()
                .unwrap_or(item.router_data.payment_id.clone()),
            order_amount_cents: item.amount,
            order_items,
            customer,
            billing_address,
            shipping_address,
            confirm_redirect_url: router_return_url.clone(),
            cancel_redirect_url: router_return_url,
        };

        Ok(Self {
            request_id: Some(item.router_data.connector_request_reference_id.clone()),
            payjustnow: payjustnow_request,
            checkout_total_cents: item.amount,
        })
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct PayjustnowAuthType {
    pub(super) merchant_account_id: Secret<String>,
    pub(super) signing_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayjustnowAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                signing_key: api_key.to_owned(),
                merchant_account_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowPaymentsResponse {
    payment_url: String,
    checkout_token: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, PayjustnowPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayjustnowPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = url::Url::parse(&item.response.payment_url.clone())
            .ok()
            .map(|url| RedirectForm::from((url, Method::Get)));
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.checkout_token),
                redirection_data: Box::new(redirection_data),
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowSyncRequest {
    checkout_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowCancelRequest {
    checkout_token: String,
}

impl TryFrom<&PaymentsCancelRouterData> for PayjustnowCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            checkout_token: item.request.connector_transaction_id.clone(),
        })
    }
}

impl TryFrom<&PaymentsSyncRouterData> for PayjustnowSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let checkout_token = item.request.get_connector_transaction_id()?;
        Ok(Self { checkout_token })
    }
}

impl TryFrom<&RefundsRouterData<RSync>> for PayjustnowSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<RSync>) -> Result<Self, Self::Error> {
        Ok(Self {
            checkout_token: item.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayjustnowCheckoutStatus {
    PendingOrder,
    PendingPayment,
    PaidPendingCallback,
    Paid,
    CancelledByMerchant,
    CancelledByConsumer,
    Expired,
}

impl From<PayjustnowCheckoutStatus> for enums::AttemptStatus {
    fn from(item: PayjustnowCheckoutStatus) -> Self {
        match item {
            PayjustnowCheckoutStatus::Paid | PayjustnowCheckoutStatus::PaidPendingCallback => {
                Self::Charged
            }
            PayjustnowCheckoutStatus::PendingOrder | PayjustnowCheckoutStatus::PendingPayment => {
                Self::AuthenticationPending
            }
            PayjustnowCheckoutStatus::CancelledByMerchant
            | PayjustnowCheckoutStatus::CancelledByConsumer => Self::Voided,
            PayjustnowCheckoutStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowSyncResponse {
    checkout_token: String,
    payment_url: Option<String>,
    checkout_payment_status: PayjustnowCheckoutStatus,
    payment_reference: Option<i64>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PayjustnowSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PayjustnowSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.checkout_payment_status);
        let redirection_data = item.response.payment_url.and_then(|url_string| {
            url::Url::parse(&url_string)
                .ok()
                .map(|url| RedirectForm::from((url, Method::Get)))
        });

        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.checkout_token),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item
                    .response
                    .payment_reference
                    .map(|id| id.to_string()),
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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowRefundRequest {
    request_id: Option<String>,
    checkout_token: String,
    merchant_refund_reference: String,
    refund_amount_cents: MinorUnit,
    refund_description: String,
}

impl<F> TryFrom<&RefundsRouterData<F>> for PayjustnowRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Some(item.connector_request_reference_id.clone()),
            checkout_token: item.request.connector_transaction_id.clone(),
            merchant_refund_reference: item.request.refund_id.clone(),
            refund_amount_cents: item.request.minor_refund_amount,
            refund_description: item
                .request
                .reason
                .clone()
                .unwrap_or(NO_REFUND_REASON.to_string()),
        })
    }
}

// Type definition for Refund Response

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Success,
    Failed,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowRefundResponse {
    request_id: String,
    refunded_amount_cents: MinorUnit,
    refund_status: RefundStatus,
    refund_status_at: String,
    refund_status_description: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, PayjustnowRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PayjustnowRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.request_id,
                refund_status: enums::RefundStatus::from(item.response.refund_status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, PayjustnowRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, PayjustnowRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.request_id,
                refund_status: enums::RefundStatus::from(item.response.refund_status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayjustnowRsyncStatus {
    FullyRefunded,
    PartiallyRefunded,
    PaidPendingCallback,
    Paid,
}

impl From<PayjustnowRsyncStatus> for enums::RefundStatus {
    fn from(item: PayjustnowRsyncStatus) -> Self {
        match item {
            PayjustnowRsyncStatus::FullyRefunded | PayjustnowRsyncStatus::PartiallyRefunded => {
                Self::Success
            }
            PayjustnowRsyncStatus::Paid | PayjustnowRsyncStatus::PaidPendingCallback => {
                Self::Failure
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowRsyncResponse {
    request_id: String,
    checkout_payment_status: PayjustnowRsyncStatus,
}

impl TryFrom<RefundsResponseRouterData<RSync, PayjustnowRsyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, PayjustnowRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.request_id.clone(),
                refund_status: enums::RefundStatus::from(item.response.checkout_payment_status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PayjustnowError {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PayjustnowErrorResponse {
    Structured(PayjustnowError),
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayjustnowWebhookStatus {
    PaidPendingCallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayjustnowWebhookDetails {
    pub checkout_token: String,
    pub checkout_payment_status: PayjustnowWebhookStatus,
}
