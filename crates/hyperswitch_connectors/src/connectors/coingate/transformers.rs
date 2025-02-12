use std::collections::HashMap;

use common_enums::{enums, Currency};
use common_utils::{request::Method, types::StringMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm},
    types::PaymentsAuthorizeRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{types::ResponseRouterData, utils};

pub struct CoingateRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for CoingateRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct CoingatePaymentsRequest {
    price_amount: StringMajorUnit,
    price_currency: Currency,
    receive_currency: String,
    callback_url: Option<String>,
    success_url: Option<String>,
    cancel_url: Option<String>,
    title: String,
}

impl TryFrom<&CoingateRouterData<&PaymentsAuthorizeRouterData>> for CoingatePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CoingateRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(match item.router_data.request.payment_method_data {
            PaymentMethodData::Crypto(_) => Ok(Self {
                price_amount: item.amount.clone(),
                price_currency: item.router_data.request.currency,
                receive_currency: "DO_NOT_CONVERT".to_string(),
                callback_url: item.router_data.request.router_return_url.clone(),
                success_url: item.router_data.request.router_return_url.clone(),
                cancel_url: item.router_data.request.router_return_url.clone(),
                title: item.router_data.connector_request_reference_id.clone(),
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Coingate"),
            )),
        }?)
    }
}

pub struct CoingateAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CoingateAuthType {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CoingatePaymentStatus {
    New,
    Pending,
    Confirming,
    Paid,
    Invalid,
    Expired,
    Canceled,
}

impl From<CoingatePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: CoingatePaymentStatus) -> Self {
        match item {
            CoingatePaymentStatus::Paid => Self::Charged,
            CoingatePaymentStatus::Canceled
            | CoingatePaymentStatus::Expired
            | CoingatePaymentStatus::Invalid => Self::Failure,
            CoingatePaymentStatus::Confirming | CoingatePaymentStatus::New => {
                Self::AuthenticationPending
            }
            CoingatePaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoingatePaymentsResponse {
    status: CoingatePaymentStatus,
    id: i64,
    payment_url: String,
    order_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, CoingatePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CoingatePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.to_string()),
                redirection_data: Box::new(Some(RedirectForm::Form {
                    endpoint: item.response.payment_url.clone(),
                    method: Method::Get,
                    form_fields: HashMap::new(),
                })),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.order_id.clone()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CoingateErrorResponse {
    pub status_code: u16,
    pub message: String,
    pub reason: String,
}
