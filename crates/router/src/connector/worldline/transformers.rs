use std::collections::HashMap;

use api_models::payments as api_models;
use common_utils::pii::{self, Email};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{
        self, api,
        storage::enums,
        transformers::{self, ForeignFrom},
    },
};

static CARD_REGEX: Lazy<HashMap<CardProduct, Result<Regex, regex::Error>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // Reference: https://gist.github.com/michaelkeevildown/9096cd3aac9029c4e6e05588448a8841
    // [#379]: Determine card issuer from card BIN number
    map.insert(CardProduct::Master, Regex::new(r"^5[1-5][0-9]{14}$"));
    map.insert(
        CardProduct::AmericanExpress,
        Regex::new(r"^3[47][0-9]{13}$"),
    );
    map.insert(CardProduct::Visa, Regex::new(r"^4[0-9]{12}(?:[0-9]{3})?$"));
    map.insert(CardProduct::Discover, Regex::new(r"^65[4-9][0-9]{13}|64[4-9][0-9]{13}|6011[0-9]{12}|(622(?:12[6-9]|1[3-9][0-9]|[2-8][0-9][0-9]|9[01][0-9]|92[0-5])[0-9]{10})$"));
    map
});

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
    pub country_code: Option<String>,
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
    pub country_code: Option<String>,
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
            api::PaymentMethod::Card(ref card) => {
                make_card_request(&item.address, &item.request, card)
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

fn make_card_request(
    address: &types::PaymentAddress,
    req: &types::PaymentsAuthorizeData,
    ccard: &api_models::Card,
) -> Result<PaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let card_number = ccard.card_number.peek().as_ref();
    let expiry_year = ccard.card_exp_year.peek().clone();
    let secret_value = format!(
        "{}{}",
        ccard.card_exp_month.peek(),
        &expiry_year[expiry_year.len() - 2..]
    );
    let expiry_date: Secret<String> = Secret::new(secret_value);
    let card = Card {
        card_number: ccard.card_number.clone(),
        cardholder_name: ccard.card_holder_name.clone(),
        cvv: ccard.card_cvc.clone(),
        expiry_date,
    };
    let payment_product_id = get_card_product_id(card_number)?;
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

fn get_card_product_id(
    card_number: &str,
) -> Result<u16, error_stack::Report<errors::ConnectorError>> {
    for (k, v) in CARD_REGEX.iter() {
        let regex: Regex = v
            .clone()
            .into_report()
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        if regex.is_match(card_number) {
            return Ok(k.product_id());
        }
    }
    Err(error_stack::Report::new(
        errors::ConnectorError::NotImplemented("Payment Method".into()),
    ))
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
}

impl From<transformers::Foreign<(PaymentStatus, enums::CaptureMethod)>>
    for transformers::Foreign<enums::AttemptStatus>
{
    fn from(item: transformers::Foreign<(PaymentStatus, enums::CaptureMethod)>) -> Self {
        let (status, capture_method) = item.0;
        match status {
            PaymentStatus::Captured
            | PaymentStatus::Paid
            | PaymentStatus::ChargebackNotification => enums::AttemptStatus::Charged,
            PaymentStatus::Cancelled => enums::AttemptStatus::Voided,
            PaymentStatus::Rejected | PaymentStatus::RejectedCapture => {
                enums::AttemptStatus::Failure
            }
            PaymentStatus::CaptureRequested => {
                if capture_method == enums::CaptureMethod::Automatic {
                    enums::AttemptStatus::Pending
                } else {
                    enums::AttemptStatus::CaptureInitiated
                }
            }
            PaymentStatus::PendingApproval => enums::AttemptStatus::Authorized,
            _ => enums::AttemptStatus::Pending,
        }
        .into()
    }
}

/// capture_method is not part of response from connector.
/// This is used to decide payment status while converting connector response to RouterData.
/// To keep this try_from logic generic in case of AUTHORIZE, SYNC and CAPTURE flows capture_method will be set from RouterData request.
#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct Payment {
    id: String,
    status: PaymentStatus,
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

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum CardProduct {
    AmericanExpress,
    Master,
    Visa,
    Discover,
}

impl CardProduct {
    fn product_id(&self) -> u16 {
        match *self {
            Self::AmericanExpress => 2,
            Self::Master => 3,
            Self::Visa => 1,
            Self::Discover => 128,
        }
    }
}
