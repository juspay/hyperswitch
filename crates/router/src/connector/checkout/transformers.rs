use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    core::errors,
    pii::{self, Secret},
    services,
    types::{self, api, storage::enums},
};

#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub number: Option<Secret<String, pii::CardNumber>>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Source {
    Card(CardSource),
    // TODO: Add other sources here.
}

pub struct CheckoutAuthType {
    pub(super) api_key: String,
    pub(super) processing_channel_id: String,
}

#[derive(Debug, Serialize)]
pub struct ReturnUrl {
    pub success_url: Option<String>,
    pub failure_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentsRequest {
    pub source: Source,
    pub amount: i32,
    pub currency: String,
    pub processing_channel_id: String,
    #[serde(rename = "3ds")]
    pub three_ds: CheckoutThreeDS,
    #[serde(flatten)]
    pub return_url: ReturnUrl,
}

#[derive(Debug, Serialize)]
pub struct CheckoutThreeDS {
    enabled: bool,
    force_3ds: bool,
}

impl TryFrom<&types::ConnectorAuthType> for CheckoutAuthType {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                processing_channel_id: key1.to_string(),
            })
        } else {
            Err(errors::ValidateError.into())
        }
    }
}
impl TryFrom<&types::PaymentsRouterData> for PaymentsRequest {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(item: &types::PaymentsRouterData) -> Result<Self, Self::Error> {
        let ccard = match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => Some(ccard),
            api::PaymentMethod::BankTransfer => None,
            api::PaymentMethod::Wallet => None,
            api::PaymentMethod::PayLater(_) => None,
            api::PaymentMethod::Paypal => None,
        };

        let three_ds = match item.auth_type {
            enums::AuthenticationType::ThreeDs => CheckoutThreeDS {
                enabled: true,
                force_3ds: true,
            },
            enums::AuthenticationType::NoThreeDs => CheckoutThreeDS {
                enabled: false,
                force_3ds: false,
            },
        };

        let return_url = ReturnUrl {
            success_url: item
                .orca_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=success")),
            failure_url: item
                .orca_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=failure")),
        };

        let source_var = Source::Card(CardSource {
            source_type: Some("card".to_owned()),
            number: ccard.map(|x| x.card_number.clone()),
            expiry_month: ccard.map(|x| x.card_exp_month.clone()),
            expiry_year: ccard.map(|x| x.card_exp_year.clone()),
        });
        let connector_auth = &item.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        Ok(PaymentsRequest {
            source: source_var,
            amount: item.amount,
            currency: item.currency.to_string(),
            processing_channel_id,
            three_ds,
            return_url,
        })
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize)]
pub enum CheckoutPaymentStatus {
    Authorized,
    #[default]
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
    Captured,
}

impl From<CheckoutPaymentStatus> for enums::AttemptStatus {
    fn from(item: CheckoutPaymentStatus) -> Self {
        match item {
            CheckoutPaymentStatus::Authorized | CheckoutPaymentStatus::Captured => {
                enums::AttemptStatus::Charged
            }
            CheckoutPaymentStatus::Declined => enums::AttemptStatus::Failure,
            CheckoutPaymentStatus::Pending => enums::AttemptStatus::Authorizing,
            CheckoutPaymentStatus::CardVerified => enums::AttemptStatus::Pending,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct Href {
    href: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct Links {
    redirect: Option<Href>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
pub struct PaymentsResponse {
    id: String,
    amount: Option<i32>,
    status: CheckoutPaymentStatus,
    #[serde(rename = "_links")]
    links: Links,
}
impl<F, Req>
    TryFrom<types::ResponseRouterData<F, PaymentsResponse, Req, types::PaymentsResponseData>>
    for types::RouterData<F, Req, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentsResponse, Req, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_url = item
            .response
            .links
            .redirect
            .map(|data| Url::parse(&data.href))
            .transpose()
            .into_report()
            .change_context(errors::ParsingError)
            .attach_printable("Could not parse the redirection data")?;

        let redirection_data = redirection_url.map(|url| services::RedirectForm {
            url: url.to_string(),
            method: services::Method::Get,
            form_fields: std::collections::HashMap::from_iter(
                url.query_pairs()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            ),
        });
        Ok(types::RouterData {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData {
                connector_transaction_id: item.response.id,
                redirect: redirection_data.is_some(),
                redirection_data,
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundRequest {
    amount: Option<i32>,
    reference: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = item.request.refund_amount;
        let reference = item.request.refund_id.clone();
        Ok(Self {
            amount: Some(amount),
            reference,
        })
    }
}
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RefundResponse {
    action_id: String,
    reference: String,
}

#[derive(Deserialize)]
pub struct CheckoutRefundResponse {
    pub(super) status: u16,
    pub(super) response: RefundResponse,
}

impl From<&CheckoutRefundResponse> for enums::RefundStatus {
    fn from(item: &CheckoutRefundResponse) -> Self {
        if item.status == 202 {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, CheckoutRefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(types::RouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, CheckoutRefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(types::RouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize)]
pub struct ErrorResponse {
    pub request_id: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub error_codes: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub enum ActionType {
    Authorization,
    Void,
    Capture,
    Refund,
    Payout,
    Return,
    #[serde(rename = "Card Verification")]
    CardVerification,
}

#[derive(Deserialize)]
pub struct ActionResponse {
    #[serde(rename = "id")]
    pub action_id: String,
    pub amount: i32,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub approved: Option<bool>,
    pub reference: Option<String>,
}

impl From<&ActionResponse> for enums::RefundStatus {
    fn from(item: &ActionResponse) -> Self {
        match item.approved {
            Some(true) => Self::Success,
            Some(false) => Self::Failure,
            None => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutRedirectResponseStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct CheckoutRedirectResponse {
    pub status: Option<CheckoutRedirectResponseStatus>,
    #[serde(rename = "cko-session-id")]
    pub cko_session_id: Option<String>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, &ActionResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(types::RouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, &ActionResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ValidateError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(types::RouterData {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<CheckoutRedirectResponseStatus> for enums::AttemptStatus {
    fn from(item: CheckoutRedirectResponseStatus) -> Self {
        match item {
            CheckoutRedirectResponseStatus::Success => {
                types::storage::enums::AttemptStatus::VbvSuccessful
            }

            CheckoutRedirectResponseStatus::Failure => {
                types::storage::enums::AttemptStatus::Failure
            }
        }
    }
}
