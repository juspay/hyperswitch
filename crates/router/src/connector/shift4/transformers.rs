use api_models::payments;
use masking::Secret;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    core::errors,
    pii, services,
    types::{self, api, storage::enums, transformers::ForeignFrom},
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shift4PaymentsRequest {
    amount: String,
    card: Option<Card>,
    currency: String,
    description: Option<String>,
    payment_method: Option<PaymentMethod>,
    captured: bool,
    flow: Option<Flow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodType {
    Eps,
    Giropay,
    Ideal,
    Sofort,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethod {
    #[serde(rename = "type")]
    method_type: PaymentMethodType,
    billing: Option<Billing>,
}

#[derive(Debug, Serialize)]
pub struct Billing {
    name: Option<Secret<String>>,
    email: Option<Secret<String, pii::Email>>,
    address: Option<Address>,
}

#[derive(Debug, Serialize)]
pub struct Address {
    line1: Option<Secret<String>>,
    line2: Option<Secret<String>>,
    zip: Option<Secret<String>>,
    state: Option<Secret<String>>,
    city: Option<String>,
    country: Option<api_models::enums::CountryCode>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: Secret<String, common_utils::pii::CardNumber>,
    exp_month: Secret<String>,
    exp_year: Secret<String>,
    cardholder_name: Secret<String>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for Shift4PaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            api::PaymentMethodData::Card(ccard) => get_card_payment_request(item, ccard),
            api::PaymentMethodData::BankRedirect(redirect_data) => {
                get_bank_redirect_request(item, redirect_data)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment Method".to_string()).into()),
        }
    }
}

fn get_card_payment_request(
    item: &types::PaymentsAuthorizeRouterData,
    card: &api_models::payments::Card,
) -> Result<Shift4PaymentsRequest, Error> {
    let submit_for_settlement = submit_for_settlement(item);
    let card = Some(Card {
        number: card.card_number.clone(),
        exp_month: card.card_exp_month.clone(),
        exp_year: card.card_exp_year.clone(),
        cardholder_name: card.card_holder_name.clone(),
    });
    Ok(Shift4PaymentsRequest {
        amount: item.request.amount.to_string(),
        card,
        currency: item.request.currency.to_string(),
        description: item.description.clone(),
        captured: submit_for_settlement,
        payment_method: None,
        flow: None,
    })
}

fn get_bank_redirect_request(
    item: &types::PaymentsAuthorizeRouterData,
    redirect_data: &payments::BankRedirectData,
) -> Result<Shift4PaymentsRequest, Error> {
    let submit_for_settlement = submit_for_settlement(item);
    let method_type = PaymentMethodType::from(redirect_data);
    let billing = get_billing(item)?;
    let payment_method = Some(PaymentMethod {
        method_type,
        billing,
    });
    let flow = get_flow(item);
    Ok(Shift4PaymentsRequest {
        amount: item.request.amount.to_string(),
        card: None,
        currency: item.request.currency.to_string(),
        description: item.description.clone(),
        captured: submit_for_settlement,
        payment_method,
        flow: Some(flow),
    })
}

impl From<&payments::BankRedirectData> for PaymentMethodType {
    fn from(value: &payments::BankRedirectData) -> Self {
        match value {
            payments::BankRedirectData::Eps { .. } => Self::Eps,
            payments::BankRedirectData::Giropay { .. } => Self::Giropay,
            payments::BankRedirectData::Ideal { .. } => Self::Ideal,
            payments::BankRedirectData::Sofort { .. } => Self::Sofort,
        }
    }
}

fn get_flow(item: &types::PaymentsAuthorizeRouterData) -> Flow {
    Flow {
        return_url: item.request.router_return_url.clone(),
    }
}

fn get_billing(item: &types::PaymentsAuthorizeRouterData) -> Result<Option<Billing>, Error> {
    let billing_address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    let address = get_address_details(billing_address);
    Ok(Some(Billing {
        name: billing_address.map(|billing| {
            Secret::new(format!("{:?} {:?}", billing.first_name, billing.last_name))
        }),
        email: item.request.email.clone(),
        address,
    }))
}

fn get_address_details(address_details: Option<&payments::AddressDetails>) -> Option<Address> {
    address_details.map(|address| Address {
        line1: address.line1.clone(),
        line2: address.line1.clone(),
        zip: address.zip.clone(),
        state: address.state.clone(),
        city: address.city.clone(),
        country: address.country,
    })
}

fn submit_for_settlement(item: &types::PaymentsAuthorizeRouterData) -> bool {
    matches!(
        item.request.capture_method,
        Some(enums::CaptureMethod::Automatic) | None
    )
}

// Auth Struct
pub struct Shift4AuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for Shift4AuthType {
    type Error = Error;
    fn try_from(item: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = item {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shift4PaymentStatus {
    Successful,
    Failed,
    #[default]
    Pending,
}

impl ForeignFrom<(bool, Option<&NextAction>, Shift4PaymentStatus)> for enums::AttemptStatus {
    fn foreign_from(item: (bool, Option<&NextAction>, Shift4PaymentStatus)) -> Self {
        let (captured, next_action, payment_status) = item;
        match payment_status {
            Shift4PaymentStatus::Successful => {
                if captured {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            Shift4PaymentStatus::Failed => Self::Failure,
            Shift4PaymentStatus::Pending => match next_action {
                Some(NextAction::Redirect) => Self::AuthenticationPending,
                Some(NextAction::Wait) | Some(NextAction::None) | None => Self::Pending,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectEventType {
    #[serde(rename = "type")]
    pub event_type: Shift4WebhookEvent,
}

#[derive(Debug, Deserialize)]
pub enum Shift4WebhookEvent {
    ChargeSucceeded,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectData {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectId {
    pub data: Shift4WebhookObjectData,
}

#[derive(Debug, Deserialize)]
pub struct Shift4WebhookObjectResource {
    pub data: serde_json::Value,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shift4PaymentsResponse {
    pub id: String,
    pub currency: String,
    pub amount: u32,
    pub status: Shift4PaymentStatus,
    pub captured: bool,
    pub refunded: bool,
    pub flow: Option<FlowResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowResponse {
    pub next_action: Option<NextAction>,
    pub redirect: Option<Redirect>,
    pub return_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Redirect {
    pub redirect_url: Option<Url>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NextAction {
    Redirect,
    Wait,
    None,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, Shift4PaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, Shift4PaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.captured,
                item.response
                    .flow
                    .as_ref()
                    .and_then(|flow| flow.next_action.as_ref()),
                item.response.status,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: item
                    .response
                    .flow
                    .and_then(|flow| flow.redirect)
                    .and_then(|redirect| redirect.redirect_url)
                    .map(|url| services::RedirectForm::from((url, services::Method::Get))),
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shift4RefundRequest {
    charge_id: String,
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for Shift4RefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            charge_id: item.request.connector_transaction_id.clone(),
            amount: item.request.refund_amount,
        })
    }
}

impl From<Shift4RefundStatus> for enums::RefundStatus {
    fn from(item: Shift4RefundStatus) -> Self {
        match item {
            self::Shift4RefundStatus::Successful => Self::Success,
            self::Shift4RefundStatus::Failed => Self::Failure,
            self::Shift4RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub charge: String,
    pub status: Shift4RefundStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shift4RefundStatus {
    Successful,
    Processing,
    #[default]
    Failed,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ErrorResponse {
    pub error: ApiErrorResponse,
}

#[derive(Default, Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct ApiErrorResponse {
    pub code: Option<String>,
    pub message: String,
}
