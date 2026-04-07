use common_enums::{enums, CountryAlpha2, Currency};
use common_utils::{pii, request::Method, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PayLaterData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        RouterData as OtherRouterData,
    },
};
pub struct AffirmRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for AffirmRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AffirmPaymentsRequest {
    pub merchant: Merchant,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping: Option<Shipping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing: Option<Billing>,
    pub total: MinorUnit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AffirmCompleteAuthorizeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub transaction_id: String,
    pub currency: Currency,
}

#[derive(Debug, Deserialize)]
pub struct AffirmRedirectResponse {
    pub checkout_token: String,
}

impl TryFrom<&PaymentsCompleteAuthorizeRouterData> for AffirmCompleteAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let payload_data = item.request.get_redirect_response_payload()?.expose();
        let redirection_response: AffirmRedirectResponse = serde_json::from_value(payload_data)
            .change_context(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "checkout_token",
            })?;
        let transaction_id = redirection_response.checkout_token;
        let order_id = item.connector_request_reference_id.clone();
        Ok(Self {
            transaction_id,
            order_id: Some(order_id),
            currency: item.request.currency,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Merchant {
    pub public_api_key: Secret<String>,
    pub user_confirmation_url: String,
    pub user_cancel_url: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shipping {
    pub name: Name,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
}
#[derive(Debug, Serialize)]
pub struct Billing {
    pub name: Name,
    pub address: Address,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<pii::Email>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zipcode: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryAlpha2>,
}

fn validate_payment_currency(
    currency: Currency,
) -> Result<(), error_stack::Report<errors::ConnectorError>> {
    if matches!(currency, Currency::USD | Currency::CAD | Currency::GBP) {
        Ok(())
    } else {
        Err(errors::ConnectorError::NotSupported {
            message: format!("{} payments", currency),
            connector: "affirm",
        }
        .into())
    }
}

fn build_billing(item: &AffirmRouterData<&PaymentsAuthorizeRouterData>) -> Option<Billing> {
    Some(Billing {
        name: Name {
            first: item.router_data.get_optional_billing_first_name(),
            last: item.router_data.get_optional_billing_last_name(),
            full: item.router_data.get_optional_billing_full_name(),
        },
        address: Address {
            line1: item.router_data.get_optional_billing_line1(),
            line2: item.router_data.get_optional_billing_line2(),
            city: item.router_data.get_optional_billing_city(),
            state: item.router_data.get_optional_billing_state(),
            zipcode: item.router_data.get_optional_billing_zip(),
            country: item.router_data.get_optional_billing_country(),
        },
        phone_number: item.router_data.get_optional_billing_phone_number(),
        email: item.router_data.get_optional_billing_email(),
    })
}

fn build_shipping(item: &AffirmRouterData<&PaymentsAuthorizeRouterData>) -> Option<Shipping> {
    Some(Shipping {
        name: Name {
            first: item.router_data.get_optional_shipping_first_name(),
            last: item.router_data.get_optional_shipping_last_name(),
            full: item.router_data.get_optional_shipping_full_name(),
        },
        address: Address {
            line1: item.router_data.get_optional_shipping_line1(),
            line2: item.router_data.get_optional_shipping_line2(),
            city: item.router_data.get_optional_shipping_city(),
            state: item.router_data.get_optional_shipping_state(),
            zipcode: item.router_data.get_optional_shipping_zip(),
            country: item.router_data.get_optional_shipping_country(),
        },
        phone_number: item.router_data.get_optional_shipping_phone_number(),
        email: item.router_data.get_optional_shipping_email(),
    })
}

impl TryFrom<&AffirmRouterData<&PaymentsAuthorizeRouterData>> for AffirmPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &AffirmRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let router_data = &item.router_data;
        let request = &router_data.request;
        let billing = build_billing(item);
        let shipping = build_shipping(item);

        match request.payment_method_data.clone() {
            PaymentMethodData::PayLater(PayLaterData::AffirmRedirect {}) => {
                let auth_type = AffirmAuthType::try_from(&item.router_data.connector_auth_type)
                    .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
                let public_api_key = auth_type.public_key;
                let merchant = Merchant {
                    public_api_key,
                    user_confirmation_url: request.get_complete_authorize_url()?,
                    user_cancel_url: request.get_router_return_url()?,
                };

                validate_payment_currency(item.router_data.request.currency)?;

                Ok(Self {
                    merchant,
                    shipping,
                    billing,
                    total: item.amount,
                    order_id: Some(item.router_data.connector_request_reference_id.clone()),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}
pub struct AffirmAuthType {
    pub public_key: Secret<String>,
    pub private_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AffirmAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                public_key: api_key.to_owned(),
                private_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

impl From<AffirmTransactionStatus> for common_enums::AttemptStatus {
    fn from(item: AffirmTransactionStatus) -> Self {
        match item {
            AffirmTransactionStatus::Authorized => Self::Authorized,
            AffirmTransactionStatus::Captured => Self::Charged,
            AffirmTransactionStatus::Voided => Self::Voided,
            AffirmTransactionStatus::PartiallyCaptured => Self::PartialCharged,
            AffirmTransactionStatus::Disputed
            | AffirmTransactionStatus::DisputeRefunded
            | AffirmTransactionStatus::PartiallyRefunded
            | AffirmTransactionStatus::Refunded => Self::Unresolved,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AffirmPaymentsResponse {
    checkout_id: String,
    redirect_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCompleteAuthorizeResponse {
    pub provider_id: Option<i32>,
    pub status: AffirmTransactionStatus,
    pub amount: Option<MinorUnit>,
    pub authorization_expiration: Option<String>,
    pub checkout_id: Option<String>,
    pub id: String,
    pub order_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffirmTransactionStatus {
    Authorized,
    Voided,
    Captured,
    PartiallyCaptured,
    Disputed,
    DisputeRefunded,
    PartiallyRefunded,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmPSyncResponse {
    pub id: String,
    pub checkout_id: Option<String>,
    pub provider_id: Option<i32>,
    pub order_id: Option<String>,
    pub status: AffirmTransactionStatus,
    pub amount: Option<MinorUnit>,
    pub amount_refunded: Option<MinorUnit>,
    pub authorization_expiration: Option<String>,
    pub created: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmRsyncResponse {
    pub amount: MinorUnit,
    pub amount_refunded: MinorUnit,
    pub id: String,
    pub order_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AffirmResponseWrapper {
    Authorize(AffirmPaymentsResponse),
    Psync(Box<AffirmPSyncResponse>),
}

impl<F, T> TryFrom<ResponseRouterData<F, AffirmResponseWrapper, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, AffirmResponseWrapper, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match &item.response {
            AffirmResponseWrapper::Authorize(resp) => {
                let redirection_data = url::Url::parse(&resp.redirect_url)
                    .ok()
                    .map(|url| RedirectForm::from((url, Method::Get)));

                Ok(Self {
                    status: enums::AttemptStatus::AuthenticationPending,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(resp.checkout_id.clone()),
                        redirection_data: Box::new(redirection_data),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        authentication_data: None,
                        charges: None,
                        incremental_authorization_allowed: None,
                    }),
                    ..item.data
                })
            }
            AffirmResponseWrapper::Psync(resp) => {
                let status = enums::AttemptStatus::from(resp.status);
                Ok(Self {
                    status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(resp.id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: resp.order_id.clone(),
                        authentication_data: None,
                        charges: None,
                        incremental_authorization_allowed: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AffirmCompleteAuthorizeResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AffirmCompleteAuthorizeResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize)]
pub struct AffirmRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&AffirmRouterData<&RefundsRouterData<F>>> for AffirmRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AffirmRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmRefundResponse {
    pub id: String,
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<MinorUnit>,
}

impl TryFrom<RefundsResponseRouterData<Execute, AffirmRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, AffirmRefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.data.request.connector_transaction_id.to_string(),
                refund_status: enums::RefundStatus::Success,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, AffirmRsyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, AffirmRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        // Affirm refunds are synchronous
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.data.request.connector_transaction_id.to_string(),
                refund_status: enums::RefundStatus::Success,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AffirmErrorResponse {
    pub status_code: u16,
    pub code: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AffirmCaptureRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub amount: MinorUnit,
}

impl TryFrom<&AffirmRouterData<&PaymentsCaptureRouterData>> for AffirmCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &AffirmRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let order_id = match item.router_data.connector_request_reference_id.clone() {
            ref_id if ref_id.is_empty() => None,
            ref_id => Some(ref_id),
        };

        let amount = item.amount;

        Ok(Self { amount, order_id })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCaptureResponse {
    pub id: String,
    pub amount: MinorUnit,
    pub created: String,
    pub currency: Currency,
    pub fee: Option<MinorUnit>,
}

impl TryFrom<PaymentsCaptureResponseRouterData<AffirmCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<AffirmCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_captured: Some(item.response.amount.get_amount_as_i64()),
            status: enums::AttemptStatus::Charged,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
#[derive(Debug, Serialize)]
pub struct AffirmCancelRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AffirmCancelResponse {
    pub id: String,
    pub created: String,
}

impl TryFrom<PaymentsCancelResponseRouterData<AffirmCancelResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<AffirmCancelResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Voided,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
