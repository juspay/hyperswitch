use api_models::payments::{AddressDetails, CCard};
use common_utils::pii::{self, Email};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    core::errors,
    types::{self, api, storage::enums, PaymentAddress, PaymentsAuthorizeData},
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
    pub payment_product_id: i8,
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
    address: &PaymentAddress,
    req: &PaymentsAuthorizeData,
    ccard: &CCard,
) -> Result<PaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let expiry_month = ccard.card_exp_month.peek().clone();
    let expiry_year = ccard.card_exp_year.peek().clone();
    let secret_value = expiry_month + &expiry_year[2..];
    let expiry_date: Secret<String> = Secret::new(secret_value);
    let card = Card {
        card_number: ccard.card_number.clone(),
        cardholder_name: ccard.card_holder_name.clone(),
        cvv: ccard.card_cvc.clone(),
        expiry_date,
    };
    let card_payment_method_specific_input = CardPaymentMethod {
        card,
        requires_approval: matches!(req.capture_method, Some(enums::CaptureMethod::Manual)),
        payment_product_id: 1,
    };

    let customer = build_customer_info(&address.clone(), req.email.clone())?;

    let order = Order {
        amount_of_money: AmountOfMoney {
            amount: req.amount,
            currency_code: req.currency.to_string().to_uppercase(),
        },
        customer,
    };

    let shipping: Option<Shipping> = address
        .clone()
        .shipping
        .and_then(|shipping| shipping.address)
        .map(|address| Shipping { ..address.into() });

    Ok(PaymentsRequest {
        card_payment_method_specific_input,
        order,
        shipping,
    })
}

fn build_customer_info(
    payment_address: &PaymentAddress,
    email: Option<Secret<String, Email>>,
) -> Result<Customer, error_stack::Report<errors::ConnectorError>> {
    let billing = payment_address
        .billing
        .clone()
        .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
    let address = billing
        .address
        .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
    address
        .country
        .clone()
        .ok_or(errors::ConnectorError::RequestEncodingFailed)?;

    let phone_country_code = billing.phone.clone().and_then(|phone| phone.country_code);
    let number: Option<Secret<String>> = billing.phone.and_then(|phone| phone.number);
    let number_with_country_code: Option<Secret<String>> = number.and_then(|number| {
        phone_country_code.map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
    });

    Ok(Customer {
        billing_address: BillingAddress { ..address.into() },
        contact_details: Some(ContactDetails {
            mobile_phone_number: number_with_country_code,
            email_address: email,
        }),
    })
}

impl From<AddressDetails> for BillingAddress {
    fn from(value: AddressDetails) -> Self {
        Self {
            city: value.city,
            country_code: value.country,
            state: value.state,
            zip: value.zip,
            ..Default::default()
        }
    }
}

impl From<AddressDetails> for Shipping {
    fn from(value: AddressDetails) -> Self {
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

// PaymentsResponse
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PaymentStatus {
    Captured,
    Paid,
    #[serde(rename = "CHARGEBACK_NOTIFICATION")]
    ChargeBackNotification,
    Cancelled,
    Rejected,
    #[serde(rename = "CHARGEBACK_NOTIFICATION")]
    RejectedCapture,
    #[serde(rename = "PENDING_APPROVAL")]
    PendingApproval,
    #[serde(rename = "CAPTURE_REQUESTED")]
    CaptureRequested,
    #[default]
    Processing,
}

impl From<PaymentStatus> for enums::AttemptStatus {
    fn from(item: PaymentStatus) -> Self {
        match item {
            PaymentStatus::Captured => Self::Charged,
            PaymentStatus::Paid => Self::Charged,
            PaymentStatus::ChargeBackNotification => Self::Charged,
            PaymentStatus::Cancelled => Self::Voided,
            PaymentStatus::Rejected => Self::Failure,
            PaymentStatus::RejectedCapture => Self::Failure,
            PaymentStatus::CaptureRequested => Self::CaptureInitiated,
            PaymentStatus::PendingApproval => Self::Authorizing,
            _ => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payment {
    id: String,
    status: PaymentStatus,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentResponse {
    payment: Payment,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.payment.status.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.payment.id),
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
pub struct WorldlineRefundRequest {
    amount_of_money: AmountOfMoney,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for WorldlineRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_of_money: AmountOfMoney {
                amount: item.request.refund_amount,
                currency_code: item.request.currency.to_string(),
            },
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
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
            RefundStatus::Cancelled => Self::Failure,
            RefundStatus::Rejected => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
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
    type Error = error_stack::Report<errors::ParsingError>;
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

//CANCEL :
impl From<&PaymentResponse> for enums::AttemptStatus {
    fn from(item: &PaymentResponse) -> Self {
        if item.payment.status == PaymentStatus::Cancelled {
            Self::Voided
        } else {
            Self::VoidFailed
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: Option<String>,
    pub property_name: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error_id: Option<String>,
    pub errors: Vec<Error>,
}
