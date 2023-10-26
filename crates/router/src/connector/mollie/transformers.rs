use api_models::payments;
use cards::CardNumber;
use common_utils::pii::Email;
use diesel_models::enums;
use error_stack::IntoReport;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    connector::utils::{
        self, AddressDetailsData, BrowserInformationData, CardData, PaymentsAuthorizeRequestData,
        RouterData,
    },
    core::errors,
    services, types,
    types::storage::enums as storage_enums,
};

type Error = error_stack::Report<errors::ConnectorError>;

#[derive(Debug, Serialize)]
pub struct MollieRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for MollieRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        (currency_unit, currency, amount, router_data): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsRequest {
    amount: Amount,
    description: String,
    redirect_url: String,
    cancel_url: Option<String>,
    webhook_url: String,
    locale: Option<String>,
    #[serde(flatten)]
    payment_method_data: PaymentMethodData,
    metadata: Option<serde_json::Value>,
    sequence_type: SequenceType,
    mandate_id: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Amount {
    currency: enums::Currency,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodData {
    Applepay(Box<ApplePayMethodData>),
    Eps,
    Giropay,
    Ideal(Box<IdealMethodData>),
    Paypal(Box<PaypalMethodData>),
    Sofort,
    Przelewy24(Box<Przelewy24MethodData>),
    Bancontact,
    CreditCard(Box<CreditCardMethodData>),
    DirectDebit(Box<DirectDebitMethodData>),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayMethodData {
    apple_pay_payment_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdealMethodData {
    issuer: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaypalMethodData {
    billing_address: Option<Address>,
    shipping_address: Option<Address>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Przelewy24MethodData {
    billing_email: Option<Email>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectDebitMethodData {
    consumer_name: Option<Secret<String>>,
    consumer_account: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditCardMethodData {
    billing_address: Option<Address>,
    shipping_address: Option<Address>,
    card_token: Option<Secret<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SequenceType {
    #[default]
    Oneoff,
    First,
    Recurring,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub street_and_number: Secret<String>,
    pub postal_code: Secret<String>,
    pub city: String,
    pub region: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
}

pub struct MollieBrowserInfo {
    language: String,
}

impl TryFrom<&MollieRouterData<&types::PaymentsAuthorizeRouterData>> for MolliePaymentsRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.router_data.request.currency,
            value: item.amount.clone(),
        };
        let description = item.router_data.get_description()?;
        let redirect_url = item.router_data.request.get_return_url()?;
        let payment_method_data = match item.router_data.request.capture_method.unwrap_or_default()
        {
            enums::CaptureMethod::Automatic => {
                match &item.router_data.request.payment_method_data {
                    api_models::payments::PaymentMethodData::Card(_) => {
                        let pm_token = item.router_data.get_payment_method_token()?;
                        Ok(PaymentMethodData::CreditCard(Box::new(
                            CreditCardMethodData {
                                billing_address: get_billing_details(item.router_data)?,
                                shipping_address: get_shipping_details(item.router_data)?,
                                card_token: Some(Secret::new(match pm_token {
                                    types::PaymentMethodToken::Token(token) => token,
                                    types::PaymentMethodToken::ApplePayDecrypt(_) => {
                                        Err(errors::ConnectorError::InvalidWalletToken)?
                                    }
                                })),
                            },
                        )))
                    }
                    api_models::payments::PaymentMethodData::BankRedirect(ref redirect_data) => {
                        PaymentMethodData::try_from(redirect_data)
                    }
                    api_models::payments::PaymentMethodData::Wallet(ref wallet_data) => {
                        get_payment_method_for_wallet(item.router_data, wallet_data)
                    }
                    api_models::payments::PaymentMethodData::BankDebit(ref directdebit_data) => {
                        PaymentMethodData::try_from(directdebit_data)
                    }
                    _ => Err(errors::ConnectorError::NotImplemented(
                        "Payment Method".to_string(),
                    ))
                    .into_report(),
                }
            }
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: format!(
                    "{} capture",
                    item.router_data.request.capture_method.unwrap_or_default()
                ),
                connector: "Mollie".to_string(),
            })
            .into_report(),
        }?;
        Ok(Self {
            amount,
            description,
            redirect_url,
            cancel_url: None,
            /* webhook_url is a mandatory field.
            But we can't support webhook in our core hence keeping it as empty string */
            webhook_url: "".to_string(),
            locale: None,
            payment_method_data,
            metadata: None,
            sequence_type: SequenceType::Oneoff,
            mandate_id: None,
        })
    }
}

impl TryFrom<&api_models::payments::BankRedirectData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::BankRedirectData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankRedirectData::Eps { .. } => Ok(Self::Eps),
            api_models::payments::BankRedirectData::Giropay { .. } => Ok(Self::Giropay),
            api_models::payments::BankRedirectData::Ideal { .. } => {
                Ok(Self::Ideal(Box::new(IdealMethodData {
                    // To do if possible this should be from the payment request
                    issuer: None,
                })))
            }
            api_models::payments::BankRedirectData::Sofort { .. } => Ok(Self::Sofort),
            api_models::payments::BankRedirectData::Przelewy24 {
                billing_details, ..
            } => Ok(Self::Przelewy24(Box::new(Przelewy24MethodData {
                billing_email: billing_details.email.clone(),
            }))),
            api_models::payments::BankRedirectData::BancontactCard { .. } => Ok(Self::Bancontact),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

impl TryFrom<&api_models::payments::BankDebitData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::BankDebitData) -> Result<Self, Self::Error> {
        match value {
            api_models::payments::BankDebitData::SepaBankDebit {
                bank_account_holder_name,
                iban,
                ..
            } => Ok(Self::DirectDebit(Box::new(DirectDebitMethodData {
                consumer_name: bank_account_holder_name.clone(),
                consumer_account: iban.clone(),
            }))),
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieCardTokenRequest {
    card_holder: Secret<String>,
    card_number: CardNumber,
    card_cvv: Secret<String>,
    card_expiry_date: Secret<String>,
    locale: String,
    testmode: bool,
    profile_token: Secret<String>,
}

impl TryFrom<&types::TokenizationRouterData> for MollieCardTokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            api_models::payments::PaymentMethodData::Card(ccard) => {
                let auth = MollieAuthType::try_from(&item.connector_auth_type)?;
                let card_holder = ccard.card_holder_name.clone();
                let card_number = ccard.card_number.clone();
                let card_expiry_date =
                    ccard.get_card_expiry_month_year_2_digit_with_delimiter("/".to_owned());
                let card_cvv = ccard.card_cvc;
                let browser_info = get_browser_info(item)?;
                let locale = browser_info
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info.language",
                    })?
                    .language;
                let testmode =
                    item.test_mode
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "test_mode",
                        })?;
                let profile_token = auth
                    .profile_token
                    .ok_or(errors::ConnectorError::FailedToObtainAuthType)?;
                Ok(Self {
                    card_holder,
                    card_number,
                    card_cvv,
                    card_expiry_date,
                    locale,
                    testmode,
                    profile_token,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment Method".to_string(),
            ))?,
        }
    }
}

fn get_payment_method_for_wallet(
    item: &types::PaymentsAuthorizeRouterData,
    wallet_data: &api_models::payments::WalletData,
) -> Result<PaymentMethodData, Error> {
    match wallet_data {
        api_models::payments::WalletData::PaypalRedirect { .. } => {
            Ok(PaymentMethodData::Paypal(Box::new(PaypalMethodData {
                billing_address: get_billing_details(item)?,
                shipping_address: get_shipping_details(item)?,
            })))
        }
        api_models::payments::WalletData::ApplePay(applepay_wallet_data) => {
            Ok(PaymentMethodData::Applepay(Box::new(ApplePayMethodData {
                apple_pay_payment_token: applepay_wallet_data.payment_data.to_owned(),
            })))
        }
        _ => Err(errors::ConnectorError::NotImplemented(
            "Payment Method".to_string(),
        ))
        .into_report(),
    }
}

fn get_shipping_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let shipping_address = item
        .address
        .shipping
        .as_ref()
        .and_then(|shipping| shipping.address.as_ref());
    get_address_details(shipping_address)
}

fn get_billing_details(
    item: &types::PaymentsAuthorizeRouterData,
) -> Result<Option<Address>, Error> {
    let billing_address = item
        .address
        .billing
        .as_ref()
        .and_then(|billing| billing.address.as_ref());
    get_address_details(billing_address)
}

fn get_address_details(
    address: Option<&payments::AddressDetails>,
) -> Result<Option<Address>, Error> {
    let address_details = match address {
        Some(address) => {
            let street_and_number = address.get_combined_address_line()?;
            let postal_code = address.get_zip()?.to_owned();
            let city = address.get_city()?.to_owned();
            let region = None;
            let country = address.get_country()?.to_owned();
            Some(Address {
                street_and_number,
                postal_code,
                city,
                region,
                country,
            })
        }
        None => None,
    };
    Ok(address_details)
}

fn get_browser_info(
    item: &types::TokenizationRouterData,
) -> Result<Option<MollieBrowserInfo>, error_stack::Report<errors::ConnectorError>> {
    if matches!(item.auth_type, enums::AuthenticationType::ThreeDs) {
        item.request
            .browser_info
            .as_ref()
            .map(|info| {
                Ok(MollieBrowserInfo {
                    language: info.get_language()?,
                })
            })
            .transpose()
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MolliePaymentsResponse {
    pub resource: String,
    pub id: String,
    pub amount: Amount,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub status: MolliePaymentStatus,
    pub is_cancelable: Option<bool>,
    pub sequence_type: SequenceType,
    pub redirect_url: Option<String>,
    pub webhook_url: Option<String>,
    #[serde(rename = "_links")]
    pub links: Links,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MolliePaymentStatus {
    Open,
    Canceled,
    #[default]
    Pending,
    Authorized,
    Expired,
    Failed,
    Paid,
}

impl From<MolliePaymentStatus> for enums::AttemptStatus {
    fn from(item: MolliePaymentStatus) -> Self {
        match item {
            MolliePaymentStatus::Paid => Self::Charged,
            MolliePaymentStatus::Failed => Self::Failure,
            MolliePaymentStatus::Pending => Self::Pending,
            MolliePaymentStatus::Open => Self::AuthenticationPending,
            MolliePaymentStatus::Canceled => Self::Voided,
            MolliePaymentStatus::Authorized => Self::Authorized,
            MolliePaymentStatus::Expired => Self::Failure,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Link {
    href: Url,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    self_: Option<Link>,
    checkout: Option<Link>,
    dashboard: Option<Link>,
    documentation: Option<Link>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDetails {
    pub card_number: String,
    pub card_holder: String,
    pub card_expiry_date: String,
    pub card_cvv: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BankDetails {
    billing_email: String,
}

pub struct MollieAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) profile_token: Option<Secret<String>>,
}

impl TryFrom<&types::ConnectorAuthType> for MollieAuthType {
    type Error = Error;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_token: None,
            }),
            types::ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                profile_token: Some(key1.to_owned()),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MollieCardTokenResponse {
    card_token: Secret<String>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MollieCardTokenResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, MollieCardTokenResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: storage_enums::AttemptStatus::Pending,
            payment_method_token: Some(types::PaymentMethodToken::Token(
                item.response.card_token.clone().expose(),
            )),
            response: Ok(types::PaymentsResponseData::TokenizationResponse {
                token: item.response.card_token.expose(),
            }),
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = Error;
    fn try_from(
        item: types::ResponseRouterData<F, MolliePaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let url = item
            .response
            .links
            .checkout
            .map(|link| services::RedirectForm::from((link.href, services::Method::Get)));
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: url,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

// REFUND :
#[derive(Default, Debug, Serialize)]
pub struct MollieRefundRequest {
    amount: Amount,
    description: Option<String>,
}

impl<F> TryFrom<&MollieRouterData<&types::RefundsRouterData<F>>> for MollieRefundRequest {
    type Error = Error;
    fn try_from(
        item: &MollieRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.router_data.request.currency,
            value: item.amount.clone(),
        };
        Ok(Self {
            amount,
            description: item.router_data.request.reason.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    resource: String,
    id: String,
    amount: Amount,
    settlement_id: Option<String>,
    settlement_amount: Option<Amount>,
    status: MollieRefundStatus,
    description: Option<String>,
    metadata: serde_json::Value,
    payment_id: String,
    #[serde(rename = "_links")]
    links: Links,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MollieRefundStatus {
    Queued,
    #[default]
    Pending,
    Processing,
    Refunded,
    Failed,
    Canceled,
}

impl From<MollieRefundStatus> for enums::RefundStatus {
    fn from(item: MollieRefundStatus) -> Self {
        match item {
            MollieRefundStatus::Queued
            | MollieRefundStatus::Pending
            | MollieRefundStatus::Processing => Self::Pending,
            MollieRefundStatus::Refunded => Self::Success,
            MollieRefundStatus::Failed | MollieRefundStatus::Canceled => Self::Failure,
        }
    }
}

impl<T> TryFrom<types::RefundsResponseRouterData<T, RefundResponse>>
    for types::RefundsRouterData<T>
{
    type Error = Error;
    fn try_from(
        item: types::RefundsResponseRouterData<T, RefundResponse>,
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

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub status: u16,
    pub title: Option<String>,
    pub detail: String,
    pub field: Option<String>,
    #[serde(rename = "_links")]
    pub links: Option<Links>,
}
