use api_models::payments as api_models;
use common_utils::pii::{self, Email};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums,
        transformers::ForeignFrom,
    },
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub card_number: Secret<String, pii::CardNumber>,
    pub cardholder_name: Secret<String>,
    pub cvv: Secret<String>,
    pub expiry_date: Secret<String>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentMethod {
    pub card: Card,
    pub requires_approval: bool,
    pub payment_product_id: u16,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AmountOfMoney {
    pub amount: i64,
    pub currency_code: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub amount_of_money: AmountOfMoney,
    pub customer: Customer,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub city: Option<String>,
    pub country_code: Option<api_enums::CountryCode>,
    pub house_number: Option<String>,
    pub state: Option<Secret<String>>,
    pub state_code: Option<String>,
    pub street: Option<String>,
    pub zip: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContactDetails {
    pub email_address: Option<Secret<String, Email>>,
    pub mobile_phone_number: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub billing_address: BillingAddress,
    pub contact_details: Option<ContactDetails>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    pub first_name: Option<Secret<String>>,
    pub surname: Option<Secret<String>>,
    pub surname_prefix: Option<Secret<String>>,
    pub title: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shipping {
    pub city: Option<String>,
    pub country_code: Option<api_enums::CountryCode>,
    pub house_number: Option<String>,
    pub name: Option<Name>,
    pub state: Option<Secret<String>>,
    pub state_code: Option<String>,
    pub street: Option<String>,
    pub zip: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsRequest {
    pub card_payment_method_specific_input: CardPaymentMethod,
    pub order: Order,
    pub shipping: Option<Shipping>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data {
            api::PaymentMethodData::Card(ref card) => {
                make_card_request(&item.address, &item.request, card)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Gateway {
    Amex = 2,
    Discover = 128,
    MasterCard = 3,
    Visa = 1,
}

impl TryFrom<utils::CardIssuer> for Gateway {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(issuer: utils::CardIssuer) -> Result<Self, Self::Error> {
        match issuer {
            utils::CardIssuer::AmericanExpress => Ok(Self::Amex),
            utils::CardIssuer::Master => Ok(Self::MasterCard),
            utils::CardIssuer::Discover => Ok(Self::Discover),
            utils::CardIssuer::Visa => Ok(Self::Visa),
            _ => Err(errors::ConnectorError::NotSupported {
                message: api_enums::PaymentMethod::Card.to_string(),
                connector: "worldline",
                payment_experience: api_enums::PaymentExperience::RedirectToUrl.to_string(),
            }
            .into()),
        }
    }
}

fn make_card_request(
    address: &types::PaymentAddress,
    req: &types::PaymentsAuthorizeData,
    ccard: &api_models::Card,
) -> Result<PaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let expiry_year = ccard.card_exp_year.peek().clone();
    let secret_value = format!(
        "{}{}",
        ccard.card_exp_month.peek(),
        &expiry_year[expiry_year.len() - 2..]
    );
    let expiry_date: Secret<String> = Secret::new(secret_value);
    let card = Card {
        card_number: ccard
            .card_number
            .clone()
            .map(|card| card.split_whitespace().collect()),
        cardholder_name: ccard.card_holder_name.clone(),
        cvv: ccard.card_cvc.clone(),
        expiry_date,
    };
    #[allow(clippy::as_conversions)]
    let payment_product_id = Gateway::try_from(ccard.get_card_issuer()?)? as u16;
    let card_payment_method_specific_input = CardPaymentMethod {
        card,
        requires_approval: matches!(req.capture_method, Some(enums::CaptureMethod::Manual)),
        payment_product_id,
    };

    let customer = build_customer_info(address, &req.email)?;

    let order = Order {
        amount_of_money: AmountOfMoney {
            amount: req.amount,
            currency_code: req.currency.to_string().to_uppercase(),
        },
        customer,
    };

    let shipping = address
        .shipping
        .as_ref()
        .and_then(|shipping| shipping.address.clone())
        .map(|address| Shipping { ..address.into() });

    Ok(PaymentsRequest {
        card_payment_method_specific_input,
        order,
        shipping,
    })
}

fn get_address(
    payment_address: &types::PaymentAddress,
) -> Option<(&api_models::Address, &api_models::AddressDetails)> {
    let billing = payment_address.billing.as_ref()?;
    let address = billing.address.as_ref()?;
    address.country.as_ref()?;
    Some((billing, address))
}

fn build_customer_info(
    payment_address: &types::PaymentAddress,
    email: &Option<Secret<String, Email>>,
) -> Result<Customer, error_stack::Report<errors::ConnectorError>> {
    let (billing, address) =
        get_address(payment_address).ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "billing.address.country",
        })?;

    let number_with_country_code = billing.phone.as_ref().and_then(|phone| {
        phone.number.as_ref().and_then(|number| {
            phone
                .country_code
                .as_ref()
                .map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
        })
    });

    Ok(Customer {
        billing_address: BillingAddress {
            ..address.clone().into()
        },
        contact_details: Some(ContactDetails {
            mobile_phone_number: number_with_country_code,
            email_address: email.clone(),
        }),
    })
}

impl From<api_models::AddressDetails> for BillingAddress {
    fn from(value: api_models::AddressDetails) -> Self {
        Self {
            city: value.city,
            country_code: value.country,
            state: value.state,
            zip: value.zip,
            ..Default::default()
        }
    }
}

impl From<api_models::AddressDetails> for Shipping {
    fn from(value: api_models::AddressDetails) -> Self {
        Self {
            city: value.city,
            country_code: value.country,
            name: Some(Name {
                first_name: value.first_name,
                surname: value.last_name,
                ..Default::default()
            }),
            state: value.state,
            zip: value.zip,
            ..Default::default()
        }
    }
}

pub struct AuthType {
    pub api_key: String,
    pub api_secret: String,
    pub merchant_account_id: String,
}

impl TryFrom<&types::ConnectorAuthType> for AuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_string(),
                api_secret: api_secret.to_string(),
                merchant_account_id: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Captured,
    Paid,
    ChargebackNotification,
    Cancelled,
    Rejected,
    RejectedCapture,
    PendingApproval,
    CaptureRequested,
    #[default]
    Processing,
    Created,
}

impl ForeignFrom<(PaymentStatus, enums::CaptureMethod)> for enums::AttemptStatus {
    fn foreign_from(item: (PaymentStatus, enums::CaptureMethod)) -> Self {
        let (status, capture_method) = item;
        match status {
            PaymentStatus::Captured
            | PaymentStatus::Paid
            | PaymentStatus::ChargebackNotification => Self::Charged,
            PaymentStatus::Cancelled => Self::Voided,
            PaymentStatus::Rejected => Self::Failure,
            PaymentStatus::RejectedCapture => Self::CaptureFailed,
            PaymentStatus::CaptureRequested => {
                if capture_method == enums::CaptureMethod::Automatic {
                    Self::Pending
                } else {
                    Self::CaptureInitiated
                }
            }
            PaymentStatus::PendingApproval => Self::Authorized,
            PaymentStatus::Created => Self::Started,
            _ => Self::Pending,
        }
    }
}

/// capture_method is not part of response from connector.
/// This is used to decide payment status while converting connector response to RouterData.
/// To keep this try_from logic generic in case of AUTHORIZE, SYNC and CAPTURE flows capture_method will be set from RouterData request.
#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct Payment {
    pub id: String,
    pub status: PaymentStatus,
    #[serde(skip_deserializing)]
    pub capture_method: enums::CaptureMethod,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Payment, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.response.capture_method,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct PaymentResponse {
    pub payment: Payment,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.payment.status,
                item.response.payment.capture_method,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize)]
pub struct ApproveRequest {}

impl TryFrom<&types::PaymentsCaptureRouterData> for ApproveRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

#[derive(Default, Debug, Serialize)]
pub struct WorldlineRefundRequest {
    amount_of_money: AmountOfMoney,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for WorldlineRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_of_money: AmountOfMoney {
                amount: item.request.refund_amount,
                currency_code: item.request.currency.to_string(),
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Cancelled,
    Rejected,
    Refunded,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Refunded => Self::Success,
            RefundStatus::Cancelled | RefundStatus::Rejected => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
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
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
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
                connector_refund_id: item.response.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: Option<String>,
    pub property_name: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_id: Option<String>,
    pub errors: Vec<Error>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookBody {
    pub api_version: Option<String>,
    pub id: String,
    pub created: String,
    pub merchant_id: String,
    #[serde(rename = "type")]
    pub event_type: WebhookEvent,
    pub payment: Option<serde_json::Value>,
    pub refund: Option<serde_json::Value>,
    pub payout: Option<serde_json::Value>,
    pub token: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub enum WebhookEvent {
    #[serde(rename = "payment.rejected")]
    Rejected,
    #[serde(rename = "payment.rejected_capture")]
    RejectedCapture,
    #[serde(rename = "payment.paid")]
    Paid,
}
