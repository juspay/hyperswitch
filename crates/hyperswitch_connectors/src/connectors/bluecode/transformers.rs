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

pub struct BluecodeRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for BluecodeRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BluecodeMetadataObject {
    pub shop_name: String,
}

impl TryFrom<&Option<common_utils::pii::SecretSerdeValue>> for BluecodeMetadataObject {
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
pub struct BluecodePaymentsRequest {
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
pub struct BluecodeCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&BluecodeRouterData<&PaymentsAuthorizeRouterData>> for BluecodePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluecodeRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.request.capture_method != Some(enums::CaptureMethod::Automatic) {
            return Err(errors::ConnectorError::FlowNotSupported {
                flow: format!("{:?}", item.router_data.request.capture_method),
                connector: "Bluecode".to_string(),
            }
            .into());
        }
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(WalletData::BluecodeRedirect {}) => {
                let bluecode_mca_metadata =
                    BluecodeMetadataObject::try_from(&item.router_data.connector_meta_data)?;
                Self::try_from((item, &bluecode_mca_metadata))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl
    TryFrom<(
        &BluecodeRouterData<&PaymentsAuthorizeRouterData>,
        &BluecodeMetadataObject,
    )> for BluecodePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: (
            &BluecodeRouterData<&PaymentsAuthorizeRouterData>,
            &BluecodeMetadataObject,
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

pub struct BluecodeAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BluecodeAuthType {
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

impl From<BluecodePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: BluecodePaymentStatus) -> Self {
        match item {
            BluecodePaymentStatus::ManualProcessing => Self::Pending,
            BluecodePaymentStatus::Pending | BluecodePaymentStatus::PaymentInitiated => {
                Self::AuthenticationPending
            }
            BluecodePaymentStatus::Failed => Self::Failure,
            BluecodePaymentStatus::Completed => Self::Charged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BluecodePaymentsResponse {
    pub id: i64,
    pub order_id: String,
    pub amount: FloatMajorUnit,
    pub currency: enums::Currency,
    pub charged_amount: FloatMajorUnit,
    pub charged_currency: enums::Currency,
    pub status: BluecodePaymentStatus,
    pub payment_link: url::Url,
    pub etoken: Secret<String>,
    pub payment_request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BluecodeSyncResponse {
    pub id: Option<i64>,
    pub order_id: String,
    pub user_id: Option<i64>,
    pub customer_id: Option<String>,
    pub customer_email: Option<Email>,
    pub customer_phone: Option<String>,
    pub status: BluecodePaymentStatus,
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
pub enum BluecodePaymentStatus {
    Pending,
    PaymentInitiated,
    ManualProcessing,
    Failed,
    Completed,
}

impl<F, T> TryFrom<ResponseRouterData<F, BluecodePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BluecodePaymentsResponse, T, PaymentsResponseData>,
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

impl<F, T> TryFrom<ResponseRouterData<F, BluecodeSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BluecodeSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: item.data.response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct BluecodeRefundRequest {
    pub amount: FloatMajorUnit,
}

impl<F> TryFrom<&BluecodeRouterData<&RefundsRouterData<F>>> for BluecodeRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &BluecodeRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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
pub struct BluecodeErrorResponse {
    pub message: String,
    pub context_data: HashMap<String, Value>,
}

pub(crate) fn get_bluecode_webhook_event(
    status: BluecodePaymentStatus,
) -> api_models::webhooks::IncomingWebhookEvent {
    match status {
        BluecodePaymentStatus::Completed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
        }
        BluecodePaymentStatus::PaymentInitiated
        | BluecodePaymentStatus::ManualProcessing
        | BluecodePaymentStatus::Pending => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
        }
        BluecodePaymentStatus::Failed => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure
        }
    }
}

pub(crate) fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<BluecodeSyncResponse, common_utils::errors::ParsingError> {
    let webhook: BluecodeSyncResponse = body.parse_struct("BluecodeIncomingWebhook")?;

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
