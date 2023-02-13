use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    pii::PeekInterface,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shift4PaymentsRequest {
    amount: String,
    card: Card,
    currency: String,
    description: Option<String>,
    captured: bool,
    three_d_secure: Option<ThreeDSecure>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DeviceData;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: String,
    exp_month: String,
    exp_year: String,
    cardholder_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecure {
    require_attempt: bool,
    require_enrolled_card: bool,
    require_successful_liability_shift_for_enrolled_card: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreeDSecureInfo {
    amount: String,
    currency: String,
    enrolled: bool,
    liability_shift: Shift4PaymentStatus,
    authentication_flow: Shift4AuthenticationFlow,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for Shift4PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let submit_for_settlement = matches!(
                    item.request.capture_method,
                    Some(enums::CaptureMethod::Automatic) | None
                );
                let three_d_secure = if item.auth_type == enums::AuthenticationType::ThreeDs {
                    Some(ThreeDSecure {
                        require_attempt: false,
                        require_enrolled_card: false,
                        require_successful_liability_shift_for_enrolled_card: true,
                    })
                } else {
                    None
                };
                let payment_request = Self {
                    amount: item.request.amount.to_string(),
                    card: Card {
                        number: ccard.card_number.peek().clone(),
                        exp_month: ccard.card_exp_month.peek().clone(),
                        exp_year: ccard.card_exp_year.peek().clone(),
                        cardholder_name: ccard.card_holder_name.peek().clone(),
                    },
                    three_d_secure: three_d_secure,
                    currency: item.request.currency.to_string(),
                    description: item.description.clone(),
                    captured: submit_for_settlement,
                };
                Ok(payment_request)
            }
            _ => Err(
                errors::ConnectorError::NotImplemented("Current Payment Method".to_string()).into(),
            ),
        }
    }
}

// Auth Struct
pub struct Shift4AuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for Shift4AuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
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
    NotPossible,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Shift4AuthenticationFlow {
    Frictionless,
    #[default]
    Challenge,
}

impl From<Shift4PaymentStatus> for enums::AttemptStatus {
    fn from(item: Shift4PaymentStatus) -> Self {
        match item {
            Shift4PaymentStatus::Successful => Self::Charged,
            Shift4PaymentStatus::Failed => Self::Failure,
            Shift4PaymentStatus::NotPossible => Self::Failure,
            Shift4PaymentStatus::Pending => Self::Pending,
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

fn get_payment_status(response: &Shift4PaymentsResponse) -> enums::AttemptStatus {
    let is_authorized =
        !response.captured && matches!(response.status, Shift4PaymentStatus::Successful);
    if is_authorized {
        enums::AttemptStatus::Authorized
    } else {
        enums::AttemptStatus::from(response.status.clone())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Shift4PaymentsResponse {
    id: String,
    currency: String,
    amount: u32,
    status: Shift4PaymentStatus,
    captured: bool,
    refunded: bool,
    three_d_secure_info: Option<ThreeDSecureInfo>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, Shift4PaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Shift4PaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_payment_status(&item.response),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
    type Error = error_stack::Report<errors::ConnectorError>;
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
