use masking::Secret;
use serde::{Deserialize, Serialize};

use super::models::{
    payment, payment_auth, refund, update_payment_request, CreateCustomerRequest,
    CreatePaymentRequest, CreatePaymentRequestAuth, CreatePaymentRequestCard,
    CreatePaymentRequestDevice, CreateRefundRequest, Customer, Payment, Refund,
    UpdatePaymentRequest,
};
use crate::{
    core::errors,
    services,
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

// UpdatePaymentRequest
impl TryFrom<&StancerRouterData<&types::PaymentsCaptureRouterData>> for UpdatePaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &StancerRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData { amount, .. } = item;

        Ok(Self {
            amount: Some(*amount),
            status: Some(update_payment_request::Status::Capture),
            ..UpdatePaymentRequest::new()
        })
    }
}

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
// Payment
impl From<payment::Status> for enums::AttemptStatus {
    fn from(value: payment::Status) -> Self {
        match value {
            payment::Status::Authorized => Self::Authorized,
            payment::Status::Canceled | payment::Status::Expired => Self::Voided,
            payment::Status::Captured => Self::Charged,
            payment::Status::ToCapture | payment::Status::CaptureSent => Self::CaptureInitiated,
            payment::Status::Refused | payment::Status::Failed => Self::Failure,
            payment::Status::Disputed => Self::AutoRefunded,
        }
    }
}

impl From<payment_auth::Status> for enums::AttemptStatus {
    fn from(value: payment_auth::Status) -> Self {
        match value {
            payment_auth::Status::Attempted
            | payment_auth::Status::Available
            | payment_auth::Status::Requested => Self::AuthenticationPending,
            payment_auth::Status::Declined
            | payment_auth::Status::Failed
            | payment_auth::Status::Unavailable => Self::AuthenticationFailed,
            payment_auth::Status::Expired => Self::Voided,
            payment_auth::Status::Success => Self::AuthenticationSuccessful,
        }
    }
}

impl<F, T> TryFrom<types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let types::ResponseRouterData::<_, _, _, _> { response, .. } = item;
        let Payment {
            status, auth, id, ..
        } = response;

        Ok(Self {
            status: status
                .map(Into::into)
                .or(auth.as_ref().map(|auth| auth.status).map(Into::into))
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "status",
                })?,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(id.to_owned()),
                redirection_data: auth
                    .map(|auth| {
                        url::Url::parse(&auth.redirect_url)
                            .map_err(|_| errors::ConnectorError::ParsingFailed)
                    })
                    .transpose()?
                    .map(|redirect_url| {
                        services::RedirectForm::from((redirect_url, services::Method::Get))
                    }),
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(id),
            }),
            ..item.data
        })
    }
}

// CreateRefundRequest
impl<F> TryFrom<&StancerRouterData<&types::RefundsRouterData<F>>> for CreateRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StancerRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let StancerRouterData {
            amount,
            router_data,
        } = item;

        Ok(Self {
            amount: *amount,
            payment: router_data.request.connector_transaction_id.to_owned(),
        })
    }
}

// Refund
impl From<refund::Status> for enums::RefundStatus {
    fn from(item: refund::Status) -> Self {
        match item {
            refund::Status::Failed | refund::Status::NotHonored => Self::Failure,
            refund::Status::RefundSent | refund::Status::ToRefund => Self::Pending,
            refund::Status::Refunded => Self::Success,
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, Refund>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, Refund>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, Refund>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, Refund>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// CreateCustomerRequest
impl TryFrom<&types::ConnectorCustomerRouterData> for CreateCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            name: item.request.name.to_owned(),
            email: item.request.email.to_owned(),
            mobile: item.request.phone.to_owned(),
        })
    }
}

// Customer
impl<F, T> TryFrom<types::ResponseRouterData<F, Customer, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Customer, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.id,
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
