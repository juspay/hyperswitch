use common_enums::enums;
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::router_data::ConnectorAuthType;
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use super::Getnetglobal;
use crate::types;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalPaymentsRequest {
    pub amount: i64,
    pub currency: String,
    pub order_id: String,
    pub customer: Customer,
    pub payment_method: PaymentMethod,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Customer {
    pub name: String,
    pub email: String,
    pub document: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentMethod {
    pub r#type: String,
    pub card: Card,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Card {
    pub number: Secret<String>,
    pub expiration_month: String,
    pub expiration_year: String,
    pub cvv: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalPaymentsResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: i64,
    pub currency: String,
    pub order_id: String,
    pub payment_method: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalCaptureRequest {
    pub amount: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalCaptureResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalCancelRequest {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalCancelResponse {
    pub transaction_id: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalRefundRequest {
    pub amount: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefundResponse {
    pub transaction_id: String,
    pub status: String,
    pub amount: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalErrorResponse {
    pub code: String,
    pub message: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalAuthType {
    pub username: Secret<String>,
    pub password: Secret<String>,
    pub merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for GetnetglobalAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                username: api_key.clone(),
                password: key1.clone(),
                merchant_id: Secret::new(String::from("")),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetnetglobalRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::RouterData<T>,
        types::PaymentsAuthorizeData,
    )> for GetnetglobalRouterData<types::RouterData<T>>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, _): (
            &types::RouterData<T>,
            types::PaymentsAuthorizeData,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.minor_amount,
            router_data: item.clone(),
        })
    }
}

impl<T>
    TryFrom<(
        &types::RouterData<T>,
        types::PaymentsCaptureData,
    )> for GetnetglobalRouterData<types::RouterData<T>>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, _): (
            &types::RouterData<T>,
            types::PaymentsCaptureData,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.minor_amount_to_capture,
            router_data: item.clone(),
        })
    }
}

impl<T>
    TryFrom<(
        &types::RouterData<T>,
        types::RefundsData,
    )> for GetnetglobalRouterData<types::RouterData<T>>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, _): (
            &types::RouterData<T>,
            types::RefundsData,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.minor_refund_amount,
            router_data: item.clone(),
        })
    }
}

impl
    TryFrom<
        &GetnetglobalRouterData<types::RouterData<types::PaymentsAuthorizeData>>,
    > for GetnetglobalPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GetnetglobalRouterData<types::RouterData<types::PaymentsAuthorizeData>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency.to_string(),
            order_id: item.router_data.connector_request_reference_id.clone(),
            customer: Customer {
                name: item.router_data.request.get_customer_name()?,
                email: item.router_data.request.email.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "customer email",
                    },
                )?,
                document: item.router_data.request.get_customer_tax_id()?,
            },
            payment_method: PaymentMethod {
                r#type: "credit_card".to_string(),
                card: Card {
                    number: item
                        .router_data
                        .request
                        .payment_method_data
                        .card
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "card",
                        })?
                        .card_number,
                    expiration_month: item
                        .router_data
                        .request
                        .payment_method_data
                        .card
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "card",
                        })?
                        .card_exp_month,
                    expiration_year: item
                        .router_data
                        .request
                        .payment_method_data
                        .card
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "card",
                        })?
                        .card_exp_year,
                    cvv: item
                        .router_data
                        .request
                        .payment_method_data
                        .card
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "card",
                        })?
                        .card_cvc,
                },
            },
        })
    }
}

impl
    TryFrom<
        &GetnetglobalRouterData<types::RouterData<types::PaymentsCaptureData>>,
    > for GetnetglobalCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GetnetglobalRouterData<types::RouterData<types::PaymentsCaptureData>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
        })
    }
}

impl
    TryFrom<
        &GetnetglobalRouterData<types::RouterData<types::RefundsData>>,
    > for GetnetglobalRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &GetnetglobalRouterData<types::RouterData<types::RefundsData>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
        })
    }
}

pub fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<GetnetglobalPaymentsResponse, errors::ConnectorError> {
    let notif: GetnetglobalPaymentsResponse = serde_json::from_slice(body)
        .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)?;
    Ok(notif)
}

pub fn get_webhook_response(
    body: &[u8],
) -> CustomResult<GetnetglobalWebhookResponse, errors::ConnectorError> {
    let response: GetnetglobalWebhookResponse = serde_json::from_slice(body)
        .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)?;
    Ok(response)
}

pub fn get_incoming_webhook_event(
    transaction_type: String,
    transaction_state: String,
) -> IncomingWebhookEvent {
    match transaction_type.as_str() {
        "PAYMENT" => match transaction_state.as_str() {
            "AUTHORIZED" => IncomingWebhookEvent::PaymentSucceeded,
            "CANCELLED" => IncomingWebhookEvent::PaymentCancelled,
            "FAILED" => IncomingWebhookEvent::PaymentFailed,
            _ => IncomingWebhookEvent::EventNotSupported,
        },
        "REFUND" => match transaction_state.as_str() {
            "COMPLETED" => IncomingWebhookEvent::RefundSucceeded,
            "FAILED" => IncomingWebhookEvent::RefundFailed,
            _ => IncomingWebhookEvent::EventNotSupported,
        },
        _ => IncomingWebhookEvent::EventNotSupported,
    }
}

pub fn is_refund_event(transaction_type: &str) -> bool {
    transaction_type == "REFUND"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalWebhookResponse {
    pub response_base64: Secret<String>,
    pub response_signature_base64: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetnetglobalWebhookPayment {
    pub transaction_id: String,
    pub transaction_type: String,
    pub transaction_state: String,
}