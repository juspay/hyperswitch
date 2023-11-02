use base64::Engine;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{AddressDetailsData, CardData, PaymentsAuthorizeRequestData},
    consts,
    core::errors,
    types::{self, api, storage::enums},
};

pub struct ElavonRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for ElavonRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: crate::connector::utils::get_amount_as_string(currency_unit, amount, currency)?,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentRequest {
    #[serde(rename = "type")]
    pub flow_type: ElavonFlowType,
    pub total: TotalAmount,
    pub description: Option<String>,
    pub custom_reference: String,
    pub ship_to: Option<OrderShippingAddress>,
    pub card: ElavonCardData,
    pub do_capture: bool,
}

#[derive(Debug, Serialize)]
pub enum ElavonPaymentsRequest {
    CardPaymentRequest(CardPaymentRequest),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ElavonFlowType {
    Sale,
    Refund,
    Void,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElavonCaptureRequest {
    do_capture: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalAmount {
    amount: String,
    currency_code: api::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElavonCardData {
    holder_name: Secret<String>,
    number: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderShippingAddress {
    full_name: Option<Secret<String>>,
    street1: Option<Secret<String>>,
    stree2: Option<Secret<String>>,
    city: Option<String>,
    region: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
    country_code: Option<String>,
    primary_phone: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ElavonCard {
    name: Secret<String>,
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&ElavonRouterData<&types::PaymentsAuthorizeRouterData>> for ElavonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &ElavonRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let card_data = CardPaymentRequest {
                    flow_type: ElavonFlowType::Sale,
                    total: TotalAmount {
                        amount: item.amount.clone(),
                        currency_code: item.router_data.request.currency,
                    },
                    description: item.router_data.description.clone(),
                    custom_reference: item.router_data.connector_request_reference_id.clone(),
                    ship_to: item.router_data.address.shipping.as_ref().and_then(
                        |shipping_address| OrderShippingAddress::try_from(shipping_address).ok(),
                    ),

                    card: ElavonCardData {
                        holder_name: req_card.card_holder_name.to_owned(),
                        number: Secret::new(req_card.card_number.to_string()),
                        expiration_month: req_card.card_exp_month.to_owned(),
                        expiration_year: req_card.get_expiry_year_4_digit(),
                        security_code: req_card.card_cvc.to_owned(),
                    },
                    do_capture: item.router_data.request.is_auto_capture()?,
                };

                Ok(Self::CardPaymentRequest(card_data))
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&api_models::payments::Address> for OrderShippingAddress {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(address_data: &api_models::payments::Address) -> Result<Self, Self::Error> {
        match address_data.address.as_ref() {
            Some(address) => Ok(Self {
                full_name: address.get_full_name().ok(),
                street1: address.get_line1().ok().cloned(),
                stree2: address.get_line2().ok().cloned(),
                city: address.get_city().ok().cloned(),
                region: address.get_state().ok().cloned(),
                postal_code: address.get_zip().ok().cloned(),
                country_code: address
                    .get_country()
                    .map(|country_code| country_code.to_string())
                    .ok(),
                primary_phone: address_data
                    .phone
                    .as_ref()
                    .and_then(|phone_data| phone_data.number.clone()),
            }),
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "address",
            })?,
        }
    }
}

pub struct ElavonAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for ElavonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::BodyKey { api_key, key1 } => {
                let auth_key = format!("{}:{}", key1.peek(), api_key.peek());
                let auth_header = format!("Basic {}", consts::BASE64_ENGINE.encode(auth_key));
                Ok(Self {
                    api_key: Secret::new(auth_header),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ElavonStatus {
    Declined,
    Authorized,
    Captured,
    Voided,
    Settled,
    Expired,
    SettlementDelayed,
    Rejected,
    HeldForReview,
    #[default]
    Unknown,
    AuthorizationPending,
}

impl From<ElavonStatus> for enums::AttemptStatus {
    fn from(item: ElavonStatus) -> Self {
        match item {
            ElavonStatus::Declined => Self::Failure,
            ElavonStatus::Captured => Self::Charged,
            ElavonStatus::Voided => Self::Voided,
            ElavonStatus::Settled => Self::Charged,
            ElavonStatus::Expired => Self::Failure,
            ElavonStatus::SettlementDelayed => Self::Pending,
            ElavonStatus::Rejected => Self::Failure,
            ElavonStatus::HeldForReview => Self::ConfirmationAwaited,
            ElavonStatus::Unknown => Self::Unresolved,
            ElavonStatus::AuthorizationPending => Self::Authorizing,
            ElavonStatus::Authorized => Self::Authorized,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElavonPaymentsResponse {
    state: ElavonStatus,
    id: String,
    custom_reference: String,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, ElavonPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, ElavonPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.state),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.custom_reference),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ElavonRefundRequest {
    #[serde(rename = "type")]
    pub flow_type: ElavonFlowType,
    pub parent_transaction: String,
    pub total: TotalAmount,
}

impl<F> TryFrom<(&ElavonRouterData<&types::RefundsRouterData<F>>, String)> for ElavonRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        refund_data: (&ElavonRouterData<&types::RefundsRouterData<F>>, String),
    ) -> Result<Self, Self::Error> {
        let item = refund_data.0;
        Ok(Self {
            flow_type: ElavonFlowType::Refund,
            parent_transaction: format!(
                "{}/transactions/{}",
                refund_data.1, item.router_data.request.connector_transaction_id
            ),
            total: TotalAmount {
                amount: item.amount.clone(),
                currency_code: item.router_data.request.currency,
            },
        })
    }
}

impl From<ElavonStatus> for enums::RefundStatus {
    fn from(item: ElavonStatus) -> Self {
        match item {
            ElavonStatus::Declined => Self::Failure,
            ElavonStatus::Authorized => Self::Pending,
            ElavonStatus::Captured => Self::Pending,
            ElavonStatus::Voided => Self::Failure,
            ElavonStatus::Settled => Self::Success,
            ElavonStatus::Expired => Self::Failure,
            ElavonStatus::SettlementDelayed => Self::Pending,
            ElavonStatus::Rejected => Self::Failure,
            ElavonStatus::HeldForReview => Self::ManualReview,
            ElavonStatus::Unknown => Self::Pending, // Have to get confirmation from the connector
            ElavonStatus::AuthorizationPending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    state: ElavonStatus,
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
                refund_status: enums::RefundStatus::from(item.response.state),
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
                refund_status: enums::RefundStatus::from(item.response.state),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ElavonErrorResponse {
    pub status: u16,
    pub failures: Vec<ElavonFailureData>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ElavonFailureData {
    pub code: String,
    pub description: String,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for ElavonCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self { do_capture: true })
    }
}

#[derive(Debug, Serialize)]
pub struct CancelRequest {
    #[serde(rename = "type")]
    pub flow_type: ElavonFlowType,
    pub parent_transaction: String,
}

impl TryFrom<(&types::PaymentsCancelRouterData, String)> for CancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        void_data: (&types::PaymentsCancelRouterData, String),
    ) -> Result<Self, Self::Error> {
        let item = void_data.0;
        Ok(Self {
            flow_type: ElavonFlowType::Void,
            parent_transaction: format!(
                "{}/transactions/{}",
                void_data.1, item.request.connector_transaction_id
            ),
        })
    }
}
