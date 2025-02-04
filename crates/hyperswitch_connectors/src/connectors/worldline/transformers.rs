use common_enums::enums::{AttemptStatus, BankNames, CaptureMethod, CountryAlpha2, Currency};
use common_utils::{pii::Email, request::Method};
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::{
        payments::Authorize,
        refunds::{Execute, RSync},
    },
    router_request_types::{PaymentsAuthorizeData, ResponseId},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{api::CurrencyUnit, errors};
use masking::{PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CardData, RouterData as RouterDataUtils},
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
    pub country_code: Option<CountryAlpha2>,
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
    pub country_code: Option<CountryAlpha2>,
    pub house_number: Option<Secret<String>>,
    pub name: Option<Name>,
    pub state: Option<Secret<String>>,
    pub state_code: Option<String>,
    pub street: Option<Secret<String>>,
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
impl<T> TryFrom<(&CurrencyUnit, Currency, i64, T)> for WorldlineRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (&CurrencyUnit, Currency, i64, T),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

impl
    TryFrom<
        &WorldlineRouterData<&RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>>,
    > for PaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &WorldlineRouterData<
            &RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        >,
    ) -> Result<Self, Self::Error> {
        let payment_data =
            match &item.router_data.request.payment_method_data {
                PaymentMethodData::Card(card) => {
                    let card_holder_name = item.router_data.get_optional_billing_full_name();
                    WorldlinePaymentMethod::CardPaymentMethodSpecificInput(Box::new(
                        make_card_request(&item.router_data.request, card, card_holder_name)?,
                    ))
                }
                PaymentMethodData::BankRedirect(bank_redirect) => {
                    WorldlinePaymentMethod::RedirectPaymentMethodSpecificInput(Box::new(
                        make_bank_redirect_request(item.router_data, bank_redirect)?,
                    ))
                }
                PaymentMethodData::CardRedirect(_)
                | PaymentMethodData::Wallet(_)
                | PaymentMethodData::PayLater(_)
                | PaymentMethodData::BankDebit(_)
                | PaymentMethodData::BankTransfer(_)
                | PaymentMethodData::Crypto(_)
                | PaymentMethodData::MandatePayment
                | PaymentMethodData::Reward
                | PaymentMethodData::RealTimePayment(_)
                | PaymentMethodData::MobilePayment(_)
                | PaymentMethodData::Upi(_)
                | PaymentMethodData::Voucher(_)
                | PaymentMethodData::GiftCard(_)
                | PaymentMethodData::OpenBanking(_)
                | PaymentMethodData::CardToken(_)
                | PaymentMethodData::NetworkToken(_)
                | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("worldline"),
                    ))?
                }
            };

        let billing_address = item.router_data.get_billing()?;

        let customer = build_customer_info(billing_address, &item.router_data.request.email)?;
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
            .get_optional_shipping()
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
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("worldline"),
            )
            .into()),
        }
    }
}

impl TryFrom<&BankNames> for WorldlineBic {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank: &BankNames) -> Result<Self, Self::Error> {
        match bank {
            BankNames::AbnAmro => Ok(Self::Abnamro),
            BankNames::AsnBank => Ok(Self::Asn),
            BankNames::Ing => Ok(Self::Ing),
            BankNames::Knab => Ok(Self::Knab),
            BankNames::Rabobank => Ok(Self::Rabobank),
            BankNames::Regiobank => Ok(Self::Regiobank),
            BankNames::SnsBank => Ok(Self::Sns),
            BankNames::TriodosBank => Ok(Self::Triodos),
            BankNames::VanLanschot => Ok(Self::Vanlanschot),
            BankNames::FrieslandBank => Ok(Self::Friesland),
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
    ccard: &hyperswitch_domain_models::payment_method_data::Card,
    card_holder_name: Option<Secret<String>>,
) -> Result<CardPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    let expiry_year = ccard.card_exp_year.peek();
    let secret_value = format!(
        "{}{}",
        ccard.card_exp_month.peek(),
        &expiry_year
            .get(expiry_year.len() - 2..)
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?
    );
    let expiry_date: Secret<String> = Secret::new(secret_value);
    let card = Card {
        card_number: ccard.card_number.clone(),
        cardholder_name: card_holder_name.unwrap_or(Secret::new("".to_string())),
        cvv: ccard.card_cvc.clone(),
        expiry_date,
    };
    #[allow(clippy::as_conversions)]
    let payment_product_id = Gateway::try_from(ccard.get_card_issuer()?)? as u16;
    let card_payment_method_specific_input = CardPaymentMethod {
        card,
        requires_approval: matches!(req.capture_method, Some(CaptureMethod::Manual)),
        payment_product_id,
    };
    Ok(card_payment_method_specific_input)
}

fn make_bank_redirect_request(
    req: &PaymentsAuthorizeRouterData,
    bank_redirect: &BankRedirectData,
) -> Result<RedirectPaymentMethod, error_stack::Report<errors::ConnectorError>> {
    let return_url = req.request.router_return_url.clone();
    let redirection_data = RedirectionData { return_url };
    let (payment_method_specific_data, payment_product_id) = match bank_redirect {
        BankRedirectData::Giropay {
            bank_account_iban, ..
        } => (
            {
                PaymentMethodSpecificData::PaymentProduct816SpecificInput(Box::new(Giropay {
                    bank_account_iban: BankAccountIban {
                        account_holder_name: req.get_billing_full_name()?.to_owned(),
                        iban: bank_account_iban.clone(),
                    },
                }))
            },
            816,
        ),
        BankRedirectData::Ideal { bank_name, .. } => (
            {
                PaymentMethodSpecificData::PaymentProduct809SpecificInput(Box::new(Ideal {
                    issuer_id: bank_name
                        .map(|bank_name| WorldlineBic::try_from(&bank_name))
                        .transpose()?,
                }))
            },
            809,
        ),
        BankRedirectData::BancontactCard { .. }
        | BankRedirectData::Bizum {}
        | BankRedirectData::Blik { .. }
        | BankRedirectData::Eps { .. }
        | BankRedirectData::Interac { .. }
        | BankRedirectData::OnlineBankingCzechRepublic { .. }
        | BankRedirectData::OnlineBankingFinland { .. }
        | BankRedirectData::OnlineBankingPoland { .. }
        | BankRedirectData::OnlineBankingSlovakia { .. }
        | BankRedirectData::OpenBankingUk { .. }
        | BankRedirectData::Przelewy24 { .. }
        | BankRedirectData::Sofort { .. }
        | BankRedirectData::Trustly { .. }
        | BankRedirectData::OnlineBankingFpx { .. }
        | BankRedirectData::OnlineBankingThailand { .. }
        | BankRedirectData::LocalBankRedirect {} => {
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
    billing: &hyperswitch_domain_models::address::Address,
) -> Option<(
    &hyperswitch_domain_models::address::Address,
    &hyperswitch_domain_models::address::AddressDetails,
)> {
    let address = billing.address.as_ref()?;
    address.country.as_ref()?;
    Some((billing, address))
}

fn build_customer_info(
    billing_address: &hyperswitch_domain_models::address::Address,
    email: &Option<Email>,
) -> Result<Customer, error_stack::Report<errors::ConnectorError>> {
    let (billing, address) =
        get_address(billing_address).ok_or(errors::ConnectorError::MissingRequiredField {
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

impl From<hyperswitch_domain_models::address::AddressDetails> for BillingAddress {
    fn from(value: hyperswitch_domain_models::address::AddressDetails) -> Self {
        Self {
            city: value.city,
            country_code: value.country,
            state: value.state,
            zip: value.zip,
            ..Default::default()
        }
    }
}

impl From<hyperswitch_domain_models::address::AddressDetails> for Shipping {
    fn from(value: hyperswitch_domain_models::address::AddressDetails) -> Self {
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

impl TryFrom<&ConnectorAuthType> for WorldlineAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
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

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
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

fn get_status(item: (PaymentStatus, CaptureMethod)) -> AttemptStatus {
    let (status, capture_method) = item;
    match status {
        PaymentStatus::Captured | PaymentStatus::Paid | PaymentStatus::ChargebackNotification => {
            AttemptStatus::Charged
        }
        PaymentStatus::Cancelled => AttemptStatus::Voided,
        PaymentStatus::Rejected => AttemptStatus::Failure,
        PaymentStatus::RejectedCapture => AttemptStatus::CaptureFailed,
        PaymentStatus::CaptureRequested => {
            if matches!(
                capture_method,
                CaptureMethod::Automatic | CaptureMethod::SequentialAutomatic
            ) {
                AttemptStatus::Pending
            } else {
                AttemptStatus::CaptureInitiated
            }
        }

        PaymentStatus::PendingApproval => AttemptStatus::Authorized,
        PaymentStatus::Created => AttemptStatus::Started,
        PaymentStatus::Redirected => AttemptStatus::AuthenticationPending,
        _ => AttemptStatus::Pending,
    }
}

/// capture_method is not part of response from connector.
/// This is used to decide payment status while converting connector response to RouterData.
/// To keep this try_from logic generic in case of AUTHORIZE, SYNC and CAPTURE flows capture_method will be set from RouterData request.
#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct Payment {
    pub id: String,
    pub status: PaymentStatus,
    #[serde(skip_deserializing)]
    pub capture_method: CaptureMethod,
}

impl<F, T> TryFrom<ResponseRouterData<F, Payment, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, Payment, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: get_status((item.response.status, item.response.capture_method)),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub payment: Payment,
    pub merchant_action: Option<MerchantAction>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantAction {
    pub redirect_data: RedirectData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RedirectData {
    #[serde(rename = "redirectURL")]
    pub redirect_url: Url,
}

impl<F, T> TryFrom<ResponseRouterData<F, PaymentResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .merchant_action
            .map(|action| action.redirect_data.redirect_url)
            .map(|redirect_url| RedirectForm::from((redirect_url, Method::Get)));
        Ok(Self {
            status: get_status((
                item.response.payment.status,
                item.response.payment.capture_method,
            )),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment.id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}
#[derive(Default, Debug, Serialize)]
pub struct ApproveRequest {}

impl TryFrom<&PaymentsCaptureRouterData> for ApproveRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

#[derive(Default, Debug, Serialize)]
pub struct WorldlineRefundRequest {
    amount_of_money: AmountOfMoney,
}

impl<F> TryFrom<&RefundsRouterData<F>> for WorldlineRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount_of_money: AmountOfMoney {
                amount: item.request.refund_amount,
                currency_code: item.request.currency.to_string(),
            },
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, Deserialize, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Cancelled,
    Rejected,
    Refunded,
    #[default]
    Processing,
}

impl From<RefundStatus> for common_enums::enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Refunded => Self::Success,
            RefundStatus::Cancelled | RefundStatus::Rejected => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = common_enums::enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = common_enums::enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub code: Option<String>,
    pub property_name: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Deserialize, PartialEq, Serialize)]
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
    pub merchant_id: common_utils::id_type::MerchantId,
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
