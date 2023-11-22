use masking::Secret;
use serde::{Deserialize, Serialize};

use super::models::{
    CreatePaymentRequest, CreatePaymentRequestAuth, CreatePaymentRequestCard,
    CreatePaymentRequestDevice,
};
use crate::{
    core::errors,
    types::{self, api, storage::enums},
};

pub struct StancerRouterData<T> {
    pub amount: i32,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for StancerRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: amount
                .try_into()
                .map_err(|_| errors::ConnectorError::ParsingFailed)?,
            router_data: item,
        })
    }
}

// CreatePaymentRequest
impl TryFrom<&StancerRouterData<&types::PaymentsAuthorizeRouterData>> for CreatePaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StancerRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData {
            amount,
            router_data,
        } = item;
        let request = CreatePaymentRequest {
            description: router_data.description.to_owned(),
            order_id: Some(router_data.connector_request_reference_id.to_owned()),
            unique_id: Some(router_data.payment_id.to_owned()),
            capture: router_data.request.capture_method.map(
                |capture_method| match capture_method {
                    common_enums::CaptureMethod::Automatic => true,
                    common_enums::CaptureMethod::Manual
                    | common_enums::CaptureMethod::ManualMultiple
                    | common_enums::CaptureMethod::Scheduled => false,
                },
            ),
            customer: router_data.connector_customer.to_owned(),
            ..CreatePaymentRequest::new(
                *amount,
                router_data.request.currency.to_string().to_lowercase(),
            )
        };
        let use_3ds = matches!(
            router_data.auth_type,
            common_enums::AuthenticationType::ThreeDs
        );

        match &router_data.request.payment_method_data {
            api::PaymentMethodData::Card(card) => Ok(CreatePaymentRequest {
                card: Some(
                    CreatePaymentRequestCard {
                        number: card.card_number.to_owned(),
                        cvc: card.card_cvc.to_owned(),
                        exp_year: card.card_exp_year.to_owned(),
                        exp_month: card.card_exp_month.to_owned(),
                    }
                    .into(),
                ),
                auth: use_3ds
                    .then(|| {
                        router_data
                            .return_url
                            .to_owned()
                            .map(|return_url| CreatePaymentRequestAuth { return_url }.into())
                    })
                    .flatten(),
                device: use_3ds
                    .then(|| {
                        router_data
                            .request
                            .browser_info
                            .as_ref()
                            .and_then(|browser_info| {
                                Some(
                                    CreatePaymentRequestDevice {
                                        ip: browser_info.ip_address.as_ref()?.to_string(),
                                        port: None,
                                        user_agent: browser_info.user_agent.to_owned(),
                                        http_accept: browser_info.accept_header.to_owned(),
                                        languages: browser_info.language.to_owned(),
                                    }
                                    .into(),
                                )
                            })
                    })
                    .flatten(),
                ..request
            }),
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct StancerAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for StancerAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StancerPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<StancerPaymentStatus> for enums::AttemptStatus {
    fn from(item: StancerPaymentStatus) -> Self {
        match item {
            StancerPaymentStatus::Succeeded => Self::Charged,
            StancerPaymentStatus::Failed => Self::Failure,
            StancerPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StancerPaymentsResponse {
    status: StancerPaymentStatus,
    id: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, StancerPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, StancerPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct StancerRefundRequest {
    pub amount: i64,
}

impl<F> TryFrom<&StancerRouterData<&types::RefundsRouterData<F>>> for StancerRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StancerRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item
                .amount
                .try_into()
                .map_err(|_| errors::ConnectorError::ParsingFailed)?,
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

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StancerErrorResponse {
    Error {
        message: serde_json::Value,
        #[serde(rename = "type")]
        error_type: String,
    },
}
