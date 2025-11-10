use std::collections::HashMap;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    pii::{Email, IpAddress},
    request::Method,
    types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self as connector_utils, PaymentsAuthorizeRequestData, RouterData as OtherRouterData},
};

pub struct CalidaRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for CalidaRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CalidaMetadataObject {
    pub shop_name: String,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for CalidaMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        meta_data: &Option<common_utils::pii::SecretSerdeValue>,
    ) -> Result<Self, Self::Error> {
        let metadata = connector_utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CalidaPaymentsRequest {
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub payment_provider: String,
    pub shop_name: String,
    pub reference: String,
    pub ip_address: Option<Secret<String, IpAddress>>,
    pub first_name: Secret<String>,
    pub last_name: Secret<String>,
    pub billing_address_country_code_iso: enums::CountryAlpha2,
    pub billing_address_city: String,
    pub billing_address_line1: Secret<String>,
    pub billing_address_postal_code: Secret<String>,
    pub webhook_url: String,
    pub success_url: String,
    pub failure_url: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct CalidaCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&CalidaRouterData<&PaymentsAuthorizeRouterData>> for CalidaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CalidaRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual)
            | Some(enums::CaptureMethod::ManualMultiple)
            | Some(enums::CaptureMethod::Scheduled)
            | Some(enums::CaptureMethod::SequentialAutomatic) => {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: format!("{:?}", item.router_data.request.capture_method),
                    connector: "Calida".to_string(),
                }
                .into())
            }
            Some(enums::CaptureMethod::Automatic) | None => {
                match item.router_data.request.payment_method_data.clone() {
                    PaymentMethodData::Wallet(WalletData::BluecodeRedirect {}) => {
                        let calida_mca_metadata =
                            CalidaMetadataObject::try_from(&item.router_data.connector_meta_data)?;
                        Self::try_from((item, &calida_mca_metadata))
                    }
                    _ => Err(
                        errors::ConnectorError::NotImplemented("Payment method".to_string()).into(),
                    ),
                }
            }
        }
    }
}

impl
    TryFrom<(
        &CalidaRouterData<&PaymentsAuthorizeRouterData>,
        &CalidaMetadataObject,
    )> for CalidaPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: (
            &CalidaRouterData<&PaymentsAuthorizeRouterData>,
            &CalidaMetadataObject,
        ),
    ) -> Result<Self, Self::Error> {
        let item = value.0;

        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            payment_provider: "bluecode_payment".to_string(),
            shop_name: value.1.shop_name.clone(),
            reference: item.router_data.payment_id.clone(),
            ip_address: item.router_data.request.get_ip_address_as_optional(),
            first_name: item.router_data.get_billing_first_name()?,
            last_name: item.router_data.get_billing_last_name()?,
            billing_address_country_code_iso: item.router_data.get_billing_country()?,
            billing_address_city: item.router_data.get_billing_city()?,
            billing_address_line1: item.router_data.get_billing_line1()?,
            billing_address_postal_code: item.router_data.get_billing_zip()?,
            webhook_url: item.router_data.request.get_webhook_url()?,
            success_url: item.router_data.request.get_router_return_url()?,
            failure_url: item.router_data.request.get_router_return_url()?,
        })
    }
}

pub struct CalidaAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CalidaAuthType {
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

impl From<CalidaPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: CalidaPaymentStatus) -> Self {
        match item {
            CalidaPaymentStatus::ManualProcessing => Self::Pending,
            CalidaPaymentStatus::Pending | CalidaPaymentStatus::PaymentInitiated => {
                Self::AuthenticationPending
            }
            CalidaPaymentStatus::Failed => Self::Failure,
            CalidaPaymentStatus::Completed => Self::Charged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalidaPaymentsResponse {
    pub id: i64,
    pub order_id: String,
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub charged_amount: FloatMajorUnit,
    pub charged_currency: enums::Currency,
    pub status: CalidaPaymentStatus,
    pub payment_link: url::Url,
    pub etoken: Secret<String>,
    pub payment_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalidaSyncResponse {
    pub id: Option<i64>,
    pub order_id: String,
    pub user_id: Option<i64>,
    pub customer_id: Option<String>,
    pub customer_email: Option<Email>,
    pub customer_phone: Option<String>,
    pub status: CalidaPaymentStatus,
    pub payment_provider: Option<String>,
    pub payment_connector: Option<String>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub shop_name: Option<String>,
    pub sender_name: Option<String>,
    pub sender_email: Option<String>,
    pub description: Option<String>,
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub charged_amount: Option<FloatMajorUnit>,
    pub charged_amount_currency: Option<String>,
    pub charged_fx_amount: Option<FloatMajorUnit>,
    pub charged_fx_amount_currency: Option<enums::Currency>,
    pub is_underpaid: Option<bool>,
    pub billing_amount: Option<FloatMajorUnit>,
    pub billing_currency: Option<String>,
    pub language: Option<String>,
    pub ip_address: Option<Secret<String, IpAddress>>,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub billing_address_line1: Option<Secret<String>>,
    pub billing_address_city: Option<Secret<String>>,
    pub billing_address_postal_code: Option<Secret<String>>,
    pub billing_address_country: Option<String>,
    pub billing_address_country_code_iso: Option<enums::CountryAlpha2>,
    pub shipping_address_country_code_iso: Option<enums::CountryAlpha2>,
    pub success_url: Option<String>,
    pub failure_url: Option<String>,
    pub source: Option<String>,
    pub bonus_code: Option<String>,
    pub dob: Option<String>,
    pub fees_amount: Option<f64>,
    pub fx_margin_amount: Option<f64>,
    pub fx_margin_percent: Option<f64>,
    pub fees_percent: Option<f64>,
    pub reseller_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CalidaPaymentStatus {
    Pending,
    PaymentInitiated,
    ManualProcessing,
    Failed,
    Completed,
}

impl<F, T> TryFrom<ResponseRouterData<F, CalidaPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CalidaPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let url = item.response.payment_link.clone();
        let redirection_data = Some(RedirectForm::from((url, Method::Get)));
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.order_id),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_request_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, CalidaSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CalidaSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: item.data.response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct CalidaRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&CalidaRouterData<&RefundsRouterData<F>>> for CalidaRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CalidaRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CalidaErrorResponse {
    pub message: String,
    pub context_data: HashMap<String, Value>,
}

pub(crate) fn get_calida_webhook_event(
    status: CalidaPaymentStatus,
) -> api_models::webhooks::IncomingWebhookEvent {
    match status {
        CalidaPaymentStatus::Completed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
        }
        CalidaPaymentStatus::PaymentInitiated
        | CalidaPaymentStatus::ManualProcessing
        | CalidaPaymentStatus::Pending => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
        }
        CalidaPaymentStatus::Failed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
        }
    }
}

pub(crate) fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<CalidaSyncResponse, common_utils::errors::ParsingError> {
    let webhook: CalidaSyncResponse = body.parse_struct("CalidaIncomingWebhook")?;

    Ok(webhook)
}

pub fn sort_and_minify_json(value: &Value) -> Result<String, errors::ConnectorError> {
    fn sort_value(val: &Value) -> Value {
        match val {
            Value::Object(map) => {
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_by_key(|(k, _)| k.to_owned());

                let sorted_map: Map<String, Value> = entries
                    .into_iter()
                    .map(|(k, v)| (k.clone(), sort_value(v)))
                    .collect();

                Value::Object(sorted_map)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(sort_value).collect()),
            _ => val.clone(),
        }
    }

    let sorted_value = sort_value(value);
    serde_json::to_string(&sorted_value)
        .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
}
