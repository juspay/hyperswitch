use api_models::payments;
use common_utils::pii::Email;
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    services,
    types::{
        self,
        api::{self, enums as api_enums},
        storage::enums,
        transformers::ForeignFrom,
        PaymentsAuthorizeData, PaymentsResponseData,
    },
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub card_number: cards::CardNumber,
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
pub struct References {
    pub merchant_reference: String,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub amount_of_money: AmountOfMoney,
    pub customer: Customer,
    pub references: References,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub city: Option<String>,
    pub country_code: Option<api_enums::CountryAlpha2>,
    pub house_number: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub state_code: Option<Secret<String>>,
    pub street: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContactDetails {
    pub email_address: Option<Email>,
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
    pub country_code: Option<api_enums::CountryAlpha2>,
    pub house_number: Option<String>,
    pub name: Option<Name>,
    pub state: Option<Secret<String>>,
    pub state_code: Option<String>,
    pub street: Option<String>,
    pub zip: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorldlinePaymentMethod {
    CardPaymentMethodSpecificInput(Box<CardPaymentMethod>),
    RedirectPaymentMethodSpecificInput(Box<RedirectPaymentMethod>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectPaymentMethod {
    pub payment_product_id: u16,
    pub redirection_data: RedirectionData,
    #[serde(flatten)]
    pub payment_method_specific_data: PaymentMethodSpecificData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectionData {
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentMethodSpecificData {
    PaymentProduct816SpecificInput(Box<Giropay>),
    PaymentProduct809SpecificInput(Box<Ideal>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Giropay {
    pub bank_account_iban: BankAccountIban,
}

#[derive(Debug, Serialize)]
pub struct Ideal {
    #[serde(rename = "issuerId")]
    pub issuer_id: Option<WorldlineBic>,
}

#[derive(Debug, Serialize)]
pub enum WorldlineBic {
    #[serde(rename = "ABNANL2A")]
    Abnamro,
    #[serde(rename = "ASNBNL21")]
    Asn,
    #[serde(rename = "FRBKNL2L")]
    Friesland,
    #[serde(rename = "KNABNL2H")]
    Knab,
    #[serde(rename = "RABONL2U")]
    Rabobank,
    #[serde(rename = "RBRBNL21")]
    Regiobank,
    #[serde(rename = "SNSBNL2A")]
    Sns,
    #[serde(rename = "TRIONL2U")]
    Triodos,
    #[serde(rename = "FVLBNL22")]
    Vanlanschot,
    #[serde(rename = "INGBNL2A")]
    Ing,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAccountIban {
    pub account_holder_name: Secret<String>,
    pub iban: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsRequest {
    #[serde(flatten)]
    pub payment_data: WorldlinePaymentMethod,
    pub order: Order,
    pub shipping: Option<Shipping>,
}

#[derive(Debug, Serialize)]
pub struct WorldlineRouterData<T> {
    amount: i64,
    router_data: T,
}
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for WorldlineRouterData<T>
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
            amount,
            router_data: item,
        })
    }
}

impl
    TryFrom<
        &WorldlineRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
            >,
        >,
    > for PaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &WorldlineRouterData<
            &types::RouterData<
                types::api::payments::Authorize,
                PaymentsAuthorizeData,
                PaymentsResponseData,
            >,
        >,
    ) -> Result<Self, Self::Error> {
        let payment_data = match &item.router_data.request.payment_method_data {
            api::PaymentMethodData::Card(card) => {
                WorldlinePaymentMethod::CardPaymentMethodSpecificInput(Box::new(make_card_request(
                    &item.router_data.request,
                    card,
                )?))
            }
            api::PaymentMethodData::BankRedirect(bank_redirect) => {
                WorldlinePaymentMethod::RedirectPaymentMethodSpecificInput(Box::new(
                    make_bank_redirect_request(&item.router_data.request, bank_redirect)?,
                ))
            }
            api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Wallet(_)
            | api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldline"),
            ))?,
        };

        let customer =
            build_customer_info(&item.router_data.address, &item.router_data.request.email)?;
        let order = Order {
            amount_of_money: AmountOfMoney {
                amount: item.amount,
                currency_code: item.router_data.request.currency.to_string().to_uppercase(),
            },
            customer,
            references: References {
                merchant_reference: item.router_data.connector_request_reference_id.clone(),
            },
        };

        let shipping = item
            .router_data
            .address
            .shipping
            .as_ref()
            .and_then(|shipping| shipping.address.clone())
            .map(Shipping::from);
        Ok(Self {
            payment_data,
            order,
            shipping,
        })
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
                message: issuer.to_string(),
                connector: "worldline",
            }
            .into()),
        }
    }
}

impl TryFrom<&api_models::enums::BankNames> for WorldlineBic {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank: &api_models::enums::BankNames) -> Result<Self, Self::Error> {
        match bank {
            api_models::enums::BankNames::AbnAmro => Ok(Self::Abnamro),
            api_models::enums::BankNames::AsnBank => Ok(Self::Asn),
            api_models::enums::BankNames::Ing => Ok(Self::Ing),
            api_models::enums::BankNames::Knab => Ok(Self::Knab),
            api_models::enums::BankNames::Rabobank => Ok(Self::Rabobank),
            api_models::enums::BankNames::Regiobank => Ok(Self::Regiobank),
            api_models::enums::BankNames::SnsBank => Ok(Self::Sns),
            api_models::enums::BankNames::TriodosBank => Ok(Self::Triodos),
            api_models::enums::BankNames::VanLanschot => Ok(Self::Vanlanschot),
            api_models::enums::BankNames::FrieslandBank => Ok(Self::Friesland),
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: bank.to_string(),
                connector: "Worldline".to_string(),
            }
            .into()),
        }
    }
}

fn make_card_request(
    req: &PaymentsAuthorizeData,
    ccard: &payments::Card,
) -> Result<CardPaymentMethod, error_stack::Report<errors::ConnectorError>> {
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
    #[allow(clippy::as_conversions)]
    let payment_product_id = Gateway::try_from(ccard.get_card_issuer()?)? as u16;
    let card_payment_method_specific_input = CardPaymentMethod {
        card,
        requires_approval: matches!(req.capture_method, Some(enums::CaptureMethod::Manual)),
        payment_product_id,
    };
    Ok(card_payment_method_specific_input)
}

fn make_bank_redirect_request(
    req: &PaymentsAuthorizeData,
    bank_redirect: &payments::BankRedirectData,
) -> Result<RedirectPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    let return_url = req.router_return_url.clone();
    let redirection_data = RedirectionData { return_url };
    let (payment_method_specific_data, payment_product_id) = match bank_redirect {
        payments::BankRedirectData::Giropay {
            billing_details,
            bank_account_iban,
            ..
        } => (
            {
                PaymentMethodSpecificData::PaymentProduct816SpecificInput(Box::new(Giropay {
                    bank_account_iban: BankAccountIban {
                        account_holder_name: billing_details.billing_name.clone().ok_or(
                            errors::ConnectorError::MissingRequiredField {
                                field_name: "billing_details.billing_name",
                            },
                        )?,
                        iban: bank_account_iban.clone(),
                    },
                }))
            },
            816,
        ),
        payments::BankRedirectData::Ideal { bank_name, .. } => (
            {
                PaymentMethodSpecificData::PaymentProduct809SpecificInput(Box::new(Ideal {
                    issuer_id: bank_name
                        .map(|bank_name| WorldlineBic::try_from(&bank_name))
                        .transpose()?,
                }))
            },
            809,
        ),
        payments::BankRedirectData::BancontactCard { .. }
        | payments::BankRedirectData::Bizum {}
        | payments::BankRedirectData::Blik { .. }
        | payments::BankRedirectData::Eps { .. }
        | payments::BankRedirectData::Interac { .. }
        | payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
        | payments::BankRedirectData::OnlineBankingFinland { .. }
        | payments::BankRedirectData::OnlineBankingPoland { .. }
        | payments::BankRedirectData::OnlineBankingSlovakia { .. }
        | payments::BankRedirectData::OpenBankingUk { .. }
        | payments::BankRedirectData::Przelewy24 { .. }
        | payments::BankRedirectData::Sofort { .. }
        | payments::BankRedirectData::Trustly { .. }
        | payments::BankRedirectData::OnlineBankingFpx { .. }
        | payments::BankRedirectData::OnlineBankingThailand { .. } => {
            return Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldline"),
            )
            .into())
        }
    };
    Ok(RedirectPaymentMethod {
        payment_product_id,
        redirection_data,
        payment_method_specific_data,
    })
}

fn get_address(
    payment_address: &types::PaymentAddress,
) -> Option<(&payments::Address, &payments::AddressDetails)> {
    let billing = payment_address.billing.as_ref()?;
    let address = billing.address.as_ref()?;
    address.country.as_ref()?;
    Some((billing, address))
}

fn build_customer_info(
    payment_address: &types::PaymentAddress,
    email: &Option<Email>,
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

impl From<payments::AddressDetails> for BillingAddress {
    fn from(value: payments::AddressDetails) -> Self {
        Self {
            city: value.city,
            country_code: value.country,
            state: value.state,
            zip: value.zip,
            ..Default::default()
        }
    }
}

impl From<payments::AddressDetails> for Shipping {
    fn from(value: payments::AddressDetails) -> Self {
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

pub struct WorldlineAuthType {
    pub api_key: Secret<String>,
    pub api_secret: Secret<String>,
    pub merchant_account_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for WorldlineAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                merchant_account_id: key1.to_owned(),
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
    Redirected,
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
            PaymentStatus::Redirected => Self::AuthenticationPending,
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

impl<F, T> TryFrom<types::ResponseRouterData<F, Payment, T, PaymentsResponseData>>
    for types::RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, Payment, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.status,
                item.response.capture_method,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub payment: Payment,
    pub merchant_action: Option<MerchantAction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAction {
    pub redirect_data: RedirectData,
}

#[derive(Debug, Deserialize)]
pub struct RedirectData {
    #[serde(rename = "redirectURL")]
    pub redirect_url: Url,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, PaymentResponse, T, PaymentsResponseData>>
    for types::RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, PaymentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .merchant_action
            .map(|action| action.redirect_data.redirect_url)
            .map(|redirect_url| {
                services::RedirectForm::from((redirect_url, services::Method::Get))
            });
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((
                item.response.payment.status,
                item.response.payment.capture_method,
            )),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.payment.id.clone(),
                ),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment.id),
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
    #[serde(other)]
    Unknown,
}
