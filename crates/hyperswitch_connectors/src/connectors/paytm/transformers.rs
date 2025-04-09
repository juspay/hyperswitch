use api_models::webhooks::IncomingWebhookEvent;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, BytesExt},
    types::{AmountConvertor, FloatMajorUnit},
};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    router_data::{AccessToken, ErrorResponse, RouterData},
    router_flow_types::{
        access_token_auth::AccessTokenAuth,
        payments::{Authorize, Capture, PSync, PaymentMethodToken, Session, SetupMandate, Void},
        refunds::{Execute, RSync},
    },
    router_request_types::{
        AccessTokenRequestData, PaymentMethodTokenizationData, PaymentsAuthorizeData,
        PaymentsCancelData, PaymentsCaptureData, PaymentsSessionData, PaymentsSyncData,
        RefundsData, SetupMandateRequestData,
    },
    router_response_types::{
        ConnectorInfo, PaymentMethodDetails, PaymentsResponseData, RefundsResponseData,
        SupportedPaymentMethods, SupportedPaymentMethodsExt,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsSyncRouterData,
        RefundSyncRouterData, RefundsRouterData,
    },
};
use masking::{ExposeInterface, Mask};
use router_env::logger;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, domain, transformers::ForeignTryFrom},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct PaytmAuthType {
    pub api_key: masking::Secret<String>,
}

impl<Flow, Request, Response> ForeignTryFrom<&RouterData<Flow, Request, Response>> for PaytmAuthType
where
    Flow: Clone,
    Request: Clone,
    Response: Clone,
{
    type Error = errors::ConnectorError;

    fn foreign_try_from(
        value: &RouterData<Flow, Request, Response>,
    ) -> CustomResult<Self, Self::Error> {
        let auth = value.get_auth()?;
        Ok(Self {
            api_key: auth.api_key.clone(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaytmRouterData {
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub payment_method_data: PaymentMethodData,
    pub billing: Option<Billing>,
    pub order_details: Option<OrderDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentMethodData {
    pub card: Card,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Card {
    pub number: masking::Secret<String>,
    pub expiry_month: masking::Secret<String>,
    pub expiry_year: masking::Secret<String>,
    pub cvv: masking::Secret<String>,
    pub card_holder_name: Option<masking::Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Billing {
    pub address: Option<Address>,
    pub phone: Option<Phone>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Phone {
    pub number: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderDetails {
    pub order_id: String,
    pub order_amount: FloatMajorUnit,
    pub order_currency: enums::Currency,
    pub order_note: Option<String>,
}

impl<Flow, Request, Response>
    ForeignTryFrom<(&FloatMajorUnit, &RouterData<Flow, Request, Response>)> for PaytmRouterData
where
    Flow: Clone,
    Request: Clone,
    Response: Clone,
{
    type Error = errors::ConnectorError;

    fn foreign_try_from(
        (amount, value): (&FloatMajorUnit, &RouterData<Flow, Request, Response>),
    ) -> CustomResult<Self, Self::Error> {
        let payment_method_data = value.get_payment_method_data()?;
        let billing = value.get_billing()?;
        let order_details = value.get_order_details()?;

        Ok(Self {
            amount: *amount,
            currency: value.get_currency()?,
            payment_method_data,
            billing,
            order_details,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PaytmPaymentsRequest {
    pub order_id: String,
    pub order_amount: FloatMajorUnit,
    pub order_currency: String,
    pub order_note: Option<String>,
    pub customer_details: CustomerDetails,
    pub payment_details: PaymentDetails,
}

#[derive(Debug, Serialize)]
pub struct CustomerDetails {
    pub customer_id: String,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub billing_address: Option<Address>,
}

#[derive(Debug, Serialize)]
pub struct PaymentDetails {
    pub payment_method: String,
    pub card_details: CardDetails,
}

#[derive(Debug, Serialize)]
pub struct CardDetails {
    pub card_number: String,
    pub card_expiry_month: String,
    pub card_expiry_year: String,
    pub card_cvv: String,
    pub card_holder_name: Option<String>,
}

impl ForeignTryFrom<&PaytmRouterData> for PaytmPaymentsRequest {
    type Error = errors::ConnectorError;

    fn foreign_try_from(value: &PaytmRouterData) -> CustomResult<Self, Self::Error> {
        let order_details =
            value
                .order_details
                .as_ref()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "order_details",
                })?;

        let customer_details = CustomerDetails {
            customer_id: order_details.order_id.clone(),
            customer_email: value.billing.as_ref().and_then(|b| b.email.clone()),
            customer_phone: value.billing.as_ref().and_then(|b| {
                b.phone.as_ref().and_then(|p| {
                    p.number
                        .as_ref()
                        .map(|n| format!("{}{}", p.country_code.as_deref().unwrap_or("+91"), n))
                })
            }),
            billing_address: value.billing.as_ref().and_then(|b| b.address.clone()),
        };

        let payment_details = PaymentDetails {
            payment_method: "CARD".to_string(),
            card_details: CardDetails {
                card_number: value.payment_method_data.card.number.expose().clone(),
                card_expiry_month: value.payment_method_data.card.expiry_month.expose().clone(),
                card_expiry_year: value.payment_method_data.card.expiry_year.expose().clone(),
                card_cvv: value.payment_method_data.card.cvv.expose().clone(),
                card_holder_name: value
                    .payment_method_data
                    .card
                    .card_holder_name
                    .as_ref()
                    .map(|n| n.expose().clone()),
            },
        };

        Ok(Self {
            order_id: order_details.order_id.clone(),
            order_amount: order_details.order_amount,
            order_currency: order_details.order_currency.to_string(),
            order_note: order_details.order_note.clone(),
            customer_details,
            payment_details,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PaytmPaymentsResponse {
    pub order_id: String,
    pub order_amount: FloatMajorUnit,
    pub order_currency: String,
    pub order_status: String,
    pub payment_id: String,
    pub payment_status: String,
    pub payment_method: String,
    pub payment_method_details: PaymentMethodDetails,
    pub customer_details: CustomerDetails,
    pub created_at: String,
    pub updated_at: String,
}

impl ForeignTryFrom<&PaytmPaymentsResponse> for PaymentsResponseData {
    type Error = errors::ConnectorError;

    fn foreign_try_from(value: &PaytmPaymentsResponse) -> CustomResult<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(value.payment_status.as_str()),
            response: Ok(types::Response {
                resource_id: types::ResponseId::ConnectorTransactionId(value.payment_id.clone()),
                redirection_data: None,
                mandate_reference_id: None,
                connector_metadata: None,
            }),
            ..Default::default()
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PaytmSyncRequest {
    pub order_id: String,
    pub payment_id: String,
}

impl ForeignTryFrom<&PaytmRouterData> for PaytmSyncRequest {
    type Error = errors::ConnectorError;

    fn foreign_try_from(value: &PaytmRouterData) -> CustomResult<Self, Self::Error> {
        let order_details =
            value
                .order_details
                .as_ref()
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "order_details",
                })?;

        Ok(Self {
            order_id: order_details.order_id.clone(),
            payment_id: value.get_connector_transaction_id()?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PaytmSyncResponse {
    pub order_id: String,
    pub order_amount: FloatMajorUnit,
    pub order_currency: String,
    pub order_status: String,
    pub payment_id: String,
    pub payment_status: String,
    pub payment_method: String,
    pub payment_method_details: PaymentMethodDetails,
    pub customer_details: CustomerDetails,
    pub created_at: String,
    pub updated_at: String,
}

impl ForeignTryFrom<&PaytmSyncResponse> for PaymentsResponseData {
    type Error = errors::ConnectorError;

    fn foreign_try_from(value: &PaytmSyncResponse) -> CustomResult<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(value.payment_status.as_str()),
            response: Ok(types::Response {
                resource_id: types::ResponseId::ConnectorTransactionId(value.payment_id.clone()),
                redirection_data: None,
                mandate_reference_id: None,
                connector_metadata: None,
            }),
            ..Default::default()
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PaytmErrorResponse {
    pub error_code: Option<String>,
    pub error_message: String,
    pub error_details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PaytmWebhookPayload {
    pub payment: PaymentEntity,
}

#[derive(Debug, Deserialize)]
pub struct PaymentEntity {
    pub entity: PaymentDetails,
}

#[derive(Debug, Deserialize)]
pub struct PaymentDetails {
    pub id: String,
    pub status: String,
    pub amount: FloatMajorUnit,
    pub currency: String,
    pub method: String,
    pub order_id: String,
    pub created_at: String,
    pub updated_at: String,
}
