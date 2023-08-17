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

impl TryFrom<&types::PaymentsAuthorizeRouterData> for MolliePaymentsRequest {
    type Error = Error;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
        };
        let description = item.get_description()?;
        let redirect_url = item.request.get_return_url()?;
        let payment_method_data = match item.request.capture_method.unwrap_or_default() {
            enums::CaptureMethod::Automatic => match &item.request.payment_method_data {
                api_models::payments::PaymentMethodData::Card(_) => Ok(
                    PaymentMethodData::CreditCard(Box::new(CreditCardMethodData {
                        billing_address: get_billing_details(item)?,
                        shipping_address: get_shipping_details(item)?,
                        card_token: Some(Secret::new(item.get_payment_method_token()?)),
                    })),
                ),
                api_models::payments::PaymentMethodData::BankRedirect(ref redirect_data) => {
                    PaymentMethodData::try_from(redirect_data)
                }
                api_models::payments::PaymentMethodData::Wallet(ref wallet_data) => {
                    get_payment_method_for_wallet(item, wallet_data)
                }
                api_models::payments::PaymentMethodData::BankDebit(ref directdebit_data) => {
                    PaymentMethodData::try_from(directdebit_data)
                }
                api_models::payments::PaymentMethodData::CardRedirect(ref cardredirect_data) => {
                    PaymentMethodData::try_from(cardredirect_data)
                }
                api_models::payments::PaymentMethodData::PayLater(ref paylater_data) => {
                    PaymentMethodData::try_from(paylater_data)
                }
                api_models::payments::PaymentMethodData::BankTransfer(ref banktransfer_data) => {
                    PaymentMethodData::try_from(banktransfer_data.as_ref())
                }
                api_models::payments::PaymentMethodData::Voucher(ref voucher_data) => {
                    PaymentMethodData::try_from(voucher_data)
                }
                api_models::payments::PaymentMethodData::GiftCard(ref giftcard_data) => {
                    PaymentMethodData::try_from(giftcard_data.as_ref())
                }
                api_models::payments::PaymentMethodData::MandatePayment
                | api_models::payments::PaymentMethodData::Crypto(_)
                | api_models::payments::PaymentMethodData::Reward(_)
                | api_models::payments::PaymentMethodData::Upi(_) => {
                    Err(errors::ConnectorError::NotSupported {
                        message: utils::get_unsupported_payment_method_error_message(),
                        connector: "Mollie",
                    })
                    .into_report()
                }
            },
            _ => Err(errors::ConnectorError::FlowNotSupported {
                flow: format!(
                    "{} capture",
                    item.request.capture_method.unwrap_or_default()
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
            api_models::payments::BankRedirectData::Bizum {}
            | api_models::payments::BankRedirectData::Blik { .. }
            | api_models::payments::BankRedirectData::Interac { .. }
            | api_models::payments::BankRedirectData::OnlineBankingCzechRepublic { .. }
            | api_models::payments::BankRedirectData::OnlineBankingFinland { .. }
            | api_models::payments::BankRedirectData::OnlineBankingPoland { .. }
            | api_models::payments::BankRedirectData::OnlineBankingSlovakia { .. }
            | api_models::payments::BankRedirectData::Trustly { .. }
            | api_models::payments::BankRedirectData::OnlineBankingFpx { .. }
            | api_models::payments::BankRedirectData::OnlineBankingThailand { .. }
            | api_models::payments::BankRedirectData::OpenBankingUk { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::get_unsupported_payment_method_error_message(),
                    connector: "Mollie",
                }
                .into())
            }
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
            api_models::payments::BankDebitData::AchBankDebit { .. }
            | api_models::payments::BankDebitData::BecsBankDebit { .. }
            | api_models::payments::BankDebitData::BacsBankDebit { .. } => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::get_unsupported_payment_method_error_message(),
                    connector: "Mollie",
                }
                .into())
            }
        }
    }
}

impl TryFrom<&api_models::payments::CardRedirectData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::CardRedirectData) -> Result<Self, Self::Error> {
        match value {
            payments::CardRedirectData::Knet {}
            | payments::CardRedirectData::Benefit {}
            | payments::CardRedirectData::MomoAtm {} => Err(errors::ConnectorError::NotSupported {
                message: utils::get_unsupported_payment_method_error_message(),
                connector: "Mollie",
            })
            .into_report(),
        }
    }
}

impl TryFrom<&api_models::payments::PayLaterData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::PayLaterData) -> Result<Self, Self::Error> {
        match value {
            payments::PayLaterData::KlarnaRedirect { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("mollie"),
                ))
                .into_report()
            }
            payments::PayLaterData::KlarnaSdk { .. }
            | payments::PayLaterData::AffirmRedirect {}
            | payments::PayLaterData::AfterpayClearpayRedirect { .. }
            | payments::PayLaterData::PayBrightRedirect {}
            | payments::PayLaterData::WalleyRedirect {}
            | payments::PayLaterData::AlmaRedirect {}
            | payments::PayLaterData::AtomeRedirect {} => {
                Err(errors::ConnectorError::NotSupported {
                    message: utils::get_unsupported_payment_method_error_message(),
                    connector: "Mollie",
                })
                .into_report()
            }
        }
    }
}

impl TryFrom<&api_models::payments::BankTransferData> for PaymentMethodData {
    type Error = Error;
    fn try_from(
        bank_transfer_data: &api_models::payments::BankTransferData,
    ) -> Result<Self, Self::Error> {
        match bank_transfer_data {
            payments::BankTransferData::SepaBankTransfer { .. } => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("mollie"),
                ))
                .into_report()
            }
            payments::BankTransferData::AchBankTransfer { .. }
            | payments::BankTransferData::BacsBankTransfer { .. }
            | payments::BankTransferData::MultibancoBankTransfer { .. }
            | payments::BankTransferData::PermataBankTransfer { .. }
            | payments::BankTransferData::BcaBankTransfer { .. }
            | payments::BankTransferData::BniVaBankTransfer { .. }
            | payments::BankTransferData::BriVaBankTransfer { .. }
            | payments::BankTransferData::CimbVaBankTransfer { .. }
            | payments::BankTransferData::DanamonVaBankTransfer { .. }
            | payments::BankTransferData::MandiriVaBankTransfer { .. }
            | payments::BankTransferData::Pix {}
            | payments::BankTransferData::Pse {} => Err(errors::ConnectorError::NotSupported {
                message: utils::get_unsupported_payment_method_error_message(),
                connector: "Mollie",
            })
            .into_report(),
        }
    }
}

impl TryFrom<&api_models::payments::VoucherData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::VoucherData) -> Result<Self, Self::Error> {
        match value {
            payments::VoucherData::Boleto(_)
            | payments::VoucherData::Efecty
            | payments::VoucherData::PagoEfectivo
            | payments::VoucherData::RedCompra
            | payments::VoucherData::RedPagos
            | payments::VoucherData::Alfamart(_)
            | payments::VoucherData::Indomaret(_)
            | payments::VoucherData::SevenEleven(_)
            | payments::VoucherData::Lawson(_)
            | payments::VoucherData::MiniStop(_)
            | payments::VoucherData::FamilyMart(_)
            | payments::VoucherData::Seicomart(_)
            | payments::VoucherData::PayEasy(_)
            | payments::VoucherData::Oxxo => Err(errors::ConnectorError::NotSupported {
                message: utils::get_unsupported_payment_method_error_message(),
                connector: "Mollie",
            })
            .into_report(),
        }
    }
}

impl TryFrom<&api_models::payments::GiftCardData> for PaymentMethodData {
    type Error = Error;
    fn try_from(value: &api_models::payments::GiftCardData) -> Result<Self, Self::Error> {
        match value {
            payments::GiftCardData::PaySafeCard {} => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("mollie"),
            ))
            .into_report(),
            payments::GiftCardData::Givex(_) => Err(errors::ConnectorError::NotSupported {
                message: utils::get_unsupported_payment_method_error_message(),
                connector: "Mollie",
            })
            .into_report(),
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
            api_models::payments::PaymentMethodData::Wallet(_)
            | api_models::payments::PaymentMethodData::CardRedirect(_)
            | api_models::payments::PaymentMethodData::PayLater(_)
            | api_models::payments::PaymentMethodData::BankRedirect(_)
            | api_models::payments::PaymentMethodData::BankDebit(_)
            | api_models::payments::PaymentMethodData::BankTransfer(_)
            | api_models::payments::PaymentMethodData::Crypto(_)
            | api_models::payments::PaymentMethodData::MandatePayment
            | api_models::payments::PaymentMethodData::Reward(_)
            | api_models::payments::PaymentMethodData::Upi(_)
            | api_models::payments::PaymentMethodData::Voucher(_)
            | api_models::payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("mollie"),
                ))?
            }
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
        api_models::payments::WalletData::AliPayQr(_)
        | api_models::payments::WalletData::AliPayRedirect(_)
        | api_models::payments::WalletData::AliPayHkRedirect(_)
        | api_models::payments::WalletData::MomoRedirect(_)
        | api_models::payments::WalletData::KakaoPayRedirect(_)
        | api_models::payments::WalletData::GoPayRedirect(_)
        | api_models::payments::WalletData::GcashRedirect(_)
        | api_models::payments::WalletData::ApplePayRedirect(_)
        | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
        | api_models::payments::WalletData::DanaRedirect {}
        | api_models::payments::WalletData::GooglePay(_)
        | api_models::payments::WalletData::GooglePayRedirect(_)
        | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
        | api_models::payments::WalletData::MbWayRedirect(_)
        | api_models::payments::WalletData::MobilePayRedirect(_)
        | api_models::payments::WalletData::PaypalSdk(_)
        | api_models::payments::WalletData::SamsungPay(_)
        | api_models::payments::WalletData::TwintRedirect {}
        | api_models::payments::WalletData::VippsRedirect {}
        | api_models::payments::WalletData::TouchNGoRedirect(_)
        | api_models::payments::WalletData::WeChatPayRedirect(_)
        | api_models::payments::WalletData::WeChatPayQr(_)
        | api_models::payments::WalletData::CashappQr(_)
        | api_models::payments::WalletData::SwishQr(_) => {
            Err(errors::ConnectorError::NotSupported {
                message: utils::get_unsupported_payment_method_error_message(),
                connector: "Mollie",
            })
            .into_report()
        }
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
            payment_method_token: Some(item.response.card_token.clone().expose()),
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

impl<F> TryFrom<&types::RefundsRouterData<F>> for MollieRefundRequest {
    type Error = Error;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let amount = Amount {
            currency: item.request.currency,
            value: utils::to_currency_base_unit(item.request.refund_amount, item.request.currency)?,
        };
        Ok(Self {
            amount,
            description: item.request.reason.to_owned(),
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
