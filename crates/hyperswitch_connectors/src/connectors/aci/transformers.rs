use std::str::FromStr;

use cards::NetworkToken;
use common_enums::enums;
use common_types::payments::{ApplePayPredecryptData, GPayPredecryptData};
use common_utils::{
    id_type,
    pii::{Email, IpAddress},
    request::Method,
    types::{SemanticVersion, StringMajorUnit},
};
use error_stack::report;
use hyperswitch_domain_models::{
    payment_method_data::{
        ApplePayWalletData, BankRedirectData, Card, GooglePayWalletData, NetworkTokenData,
        PayLaterData, PaymentMethodData, SamsungPayWalletData, WalletData,
    },
    payment_methods::storage_enums::MitCategory,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::{
        authentication::{PostAuthentication, PreAuthentication},
        SetupMandate,
    },
    router_request_types::{
        authentication::{
            AuthNFlowType, ConnectorPostAuthenticationRequestData, PreAuthNRequestData,
        },
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsSyncData, ResponseId,
        SetupMandateRequestData, UcsAuthenticationData,
    },
    router_response_types::{
        AuthenticationResponseData, MandateReference, PaymentsResponseData, RedirectForm,
        RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use api_models::payments::MandateReferenceId;
use super::aci_result_codes::{FAILURE_CODES, PENDING_CODES, SUCCESSFUL_CODES};
use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, CardData, NetworkTokenData as NetworkTokenDataTrait, PaymentsAuthorizeRequestData,
        PhoneDetailsData, RouterData as _,
    },
};

type Error = error_stack::Report<errors::ConnectorError>;

/// Dynamic `customParameters[key]` entries forwarded to ACI.
/// Each entry serializes as its own form field, e.g. `customParameters[paymentId]=pay_xxx`.
#[derive(Debug, Default)]
pub struct AciCustomParameters(Vec<(String, String)>);

impl AciCustomParameters {
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.0.push((key.into(), value.into()));
    }
}

impl Serialize for AciCustomParameters {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(&format!("customParameters[{k}]"), v)?;
        }
        map.end()
    }
}

trait AttemptStatusMapper {
    fn get_capture_method(&self) -> Option<enums::CaptureMethod>;

    fn map_success_status(&self, auto_capture: bool) -> enums::AttemptStatus {
        if auto_capture {
            enums::AttemptStatus::Charged
        } else {
            enums::AttemptStatus::Authorized
        }
    }
}

impl AttemptStatusMapper for PaymentsAuthorizeData {
    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        self.capture_method
    }
}

impl AttemptStatusMapper for PaymentsSyncData {
    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        self.capture_method
    }
}

impl AttemptStatusMapper for PaymentsCancelData {
    fn get_capture_method(&self) -> Option<enums::CaptureMethod> {
        None
    }

    fn map_success_status(&self, _auto_capture: bool) -> enums::AttemptStatus {
        enums::AttemptStatus::Voided
    }
}

#[derive(Debug, Serialize)]
pub struct AciRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for AciRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct AciAuthType {
    pub api_key: Secret<String>,
    pub entity_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for AciAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = item {
            Ok(Self {
                api_key: api_key.to_owned(),
                entity_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum StandingInstructionReason {
    #[serde(rename = "RESUBMISSION")]
    Resubmission,
    #[serde(rename = "DELAYED_CHARGES")]
    DelayedCharges,
    #[serde(rename = "NO_SHOW")]
    NoShow,
}

#[derive(Debug, Serialize, Default)]
pub struct AciCustomerBrowserInfo {
    #[serde(rename = "customer.browser.acceptHeader")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_header: Option<String>,
    #[serde(rename = "customer.browser.language")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "customer.browser.screenHeight")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<String>,
    #[serde(rename = "customer.browser.screenWidth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<String>,
    #[serde(rename = "customer.browser.timezone")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(rename = "customer.browser.userAgent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(rename = "customer.browser.javascriptEnabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript_enabled: Option<bool>,
    #[serde(rename = "customer.browser.javaEnabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_enabled: Option<bool>,
    #[serde(rename = "customer.browser.screenColorDepth")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_depth: Option<String>,
    #[serde(rename = "customer.browser.challengeWindow")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_window: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciCustomerData {
    #[serde(rename = "customer.ip")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<Secret<String, IpAddress>>,
    #[serde(rename = "customer.email")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Email>,
    #[serde(rename = "customer.givenName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<Secret<String>>,
    #[serde(rename = "customer.surname")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surname: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciBillingAddress {
    #[serde(rename = "billing.street1")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street1: Option<Secret<String>>,
    #[serde(rename = "billing.city")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(rename = "billing.state")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Secret<String>>,
    #[serde(rename = "billing.country")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "billing.postcode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postcode: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Default)]
pub struct AciExternalThreeDsData {
    #[serde(rename = "threeDSecure.eci")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
    #[serde(rename = "threeDSecure.verificationId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_id: Option<Secret<String>>,
    #[serde(rename = "threeDSecure.dsTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ds_transaction_id: Option<String>,
    #[serde(rename = "threeDSecure.acsTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acs_transaction_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsRequest {
    #[serde(flatten)]
    pub txn_details: TransactionDetails,
    #[serde(flatten)]
    pub payment_method: PaymentDetails,
    #[serde(flatten)]
    pub instruction: Option<Instruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_result_url: Option<String>,
    #[serde(rename = "customParameters[3DS2_enrolled]")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds_two_enrolled: Option<bool>,
    #[serde(rename = "merchantTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merchant_transaction_id: Option<String>,
    #[serde(flatten)]
    pub customer_browser_info: AciCustomerBrowserInfo,
    #[serde(flatten)]
    pub customer_data: AciCustomerData,
    #[serde(flatten)]
    pub billing_address: AciBillingAddress,
    #[serde(flatten)]
    pub external_three_ds: AciExternalThreeDsData,
    #[serde(flatten)]
    pub custom_parameters: AciCustomParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    pub entity_id: Secret<String>,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCancelRequest {
    pub entity_id: Secret<String>,
    pub payment_type: AciPaymentType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciMandateRequest {
    pub entity_id: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_brand: Option<PaymentBrand>,
    #[serde(flatten)]
    pub payment_details: PaymentDetails,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciMandateResponse {
    pub id: String,
    pub result: ResultCode,
    pub build_number: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PaymentDetails {
    #[serde(rename = "card")]
    AciCard(Box<CardDetails>),
    BankRedirect(Box<BankRedirectionPMData>),
    Wallet(Box<WalletPMData>),
    Klarna,
    Mandate,
    AciNetworkToken(Box<AciNetworkTokenData>),
    AciApplePayEncrypted(Box<AciApplePayEncryptedData>),
    AciGooglePayEncrypted(Box<AciGooglePayEncryptedData>),
    AciSamsungPay(Box<AciSamsungPayData>),
}

impl TryFrom<(&WalletData, &PaymentsAuthorizeRouterData)> for PaymentDetails {
    type Error = Error;
    fn try_from(value: (&WalletData, &PaymentsAuthorizeRouterData)) -> Result<Self, Self::Error> {
        let (wallet_data, item) = value;
        let payment_data = match wallet_data {
            WalletData::MbWayRedirect(_) => {
                let phone_details = item.get_billing_phone()?;
                Self::Wallet(Box::new(WalletPMData {
                    payment_brand: PaymentBrand::Mbway,
                    account_id: Some(phone_details.get_number_with_hash_country_code()?),
                }))
            }
            WalletData::AliPayRedirect { .. } => Self::Wallet(Box::new(WalletPMData {
                payment_brand: PaymentBrand::AliPay,
                account_id: None,
            })),
            // ApplePay and GooglePay are handled separately in AciPaymentsRequest::try_from
            // to extract decrypted token data
            WalletData::ApplePay(_) | WalletData::GooglePay(_) => {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: "Wallet via PaymentDetails".to_string(),
                    connector: "ACI".to_string(),
                })?
            }
            WalletData::SamsungPay(samsung_pay_data) => Self::try_from(samsung_pay_data.as_ref())?,
            WalletData::AliPayHkRedirect(_)
            | WalletData::AmazonPayRedirect(_)
            | WalletData::Paysera(_)
            | WalletData::Skrill(_)
            | WalletData::MomoRedirect(_)
            | WalletData::KakaoPayRedirect(_)
            | WalletData::GoPayRedirect(_)
            | WalletData::GcashRedirect(_)
            | WalletData::AmazonPay(_)
            | WalletData::ApplePayThirdPartySdk(_)
            | WalletData::DanaRedirect { .. }
            | WalletData::BluecodeRedirect {}
            | WalletData::GooglePayThirdPartySdk(_)
            | WalletData::MobilePayRedirect(_)
            | WalletData::PaypalRedirect(_)
            | WalletData::PaypalSdk(_)
            | WalletData::Paze(_)
            | WalletData::TwintRedirect { .. }
            | WalletData::VippsRedirect { .. }
            | WalletData::TouchNGoRedirect(_)
            | WalletData::WeChatPayRedirect(_)
            | WalletData::WeChatPayQr(_)
            | WalletData::CashappQr(_)
            | WalletData::SwishQr(_)
            | WalletData::AliPayQr(_)
            | WalletData::ApplePayRedirect(_)
            | WalletData::GooglePayRedirect(_)
            | WalletData::Mifinity(_)
            | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };
        Ok(payment_data)
    }
}

/// Convert Apple Pay decrypted data to ACI network token format
impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &ApplePayWalletData,
        Option<&PaymentMethodToken>,
    )> for PaymentDetails
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &ApplePayWalletData,
            Option<&PaymentMethodToken>,
        ),
    ) -> Result<Self, Self::Error> {
        let (_item, apple_pay_wallet_data, payment_method_token) = value;

        // Try to get decrypted data; if available, use tokenAccount.* flow
        match get_apple_pay_data(apple_pay_wallet_data, payment_method_token) {
            Ok(apple_pay_data) => {
                let payment_brand =
                    parse_wallet_card_network(&apple_pay_wallet_data.payment_method.network)
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "apple_pay.payment_method.network",
                        })?;

                let aci_network_token_data = AciNetworkTokenData {
                    token_type: AciTokenAccountType::Network,
                    token_number: NetworkToken::from(
                        apple_pay_data.application_primary_account_number.clone(),
                    ),
                    token_expiry_month: apple_pay_data.application_expiration_month.clone(),
                    token_expiry_year: apple_pay_data.get_four_digit_expiry_year(),
                    token_cryptogram: Some(
                        apple_pay_data
                            .payment_data
                            .online_payment_cryptogram
                            .clone(),
                    ),
                    eci: apple_pay_data.payment_data.eci_indicator.clone(),
                    payment_brand,
                };

                Ok(Self::AciNetworkToken(Box::new(aci_network_token_data)))
            }
            // Fall back to encrypted passthrough — let ACI decrypt
            Err(_) => {
                let encrypted_token = apple_pay_wallet_data
                    .payment_data
                    .get_encrypted_apple_pay_payment_data_optional()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "apple_pay.payment_data (encrypted)",
                    })?;

                Ok(Self::AciApplePayEncrypted(Box::new(
                    AciApplePayEncryptedData {
                        payment_token: Secret::new(encrypted_token.clone()),
                        source: "web".to_string(),
                    },
                )))
            }
        }
    }
}

/// Convert Google Pay decrypted data to ACI network token format
impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &GooglePayWalletData,
        Option<&PaymentMethodToken>,
    )> for PaymentDetails
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &GooglePayWalletData,
            Option<&PaymentMethodToken>,
        ),
    ) -> Result<Self, Self::Error> {
        let (_item, google_pay_wallet_data, payment_method_token) = value;

        // Try to get decrypted data; if available, use tokenAccount.* flow
        match get_google_pay_data(google_pay_wallet_data, payment_method_token) {
            Ok(google_pay_data) => {
                let payment_brand = parse_wallet_card_network(
                    &google_pay_wallet_data.info.card_network,
                )
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "google_pay.info.card_network",
                })?;

                let aci_network_token_data = AciNetworkTokenData {
                    token_type: AciTokenAccountType::Network,
                    token_number: NetworkToken::from(
                        google_pay_data.application_primary_account_number.clone(),
                    ),
                    token_expiry_month: google_pay_data.card_exp_month.clone(),
                    token_expiry_year: google_pay_data.get_four_digit_expiry_year().map_err(
                        |_| errors::ConnectorError::MissingRequiredField {
                            field_name: "google_pay.card_exp_year",
                        },
                    )?,
                    token_cryptogram: google_pay_data.cryptogram.clone(),
                    eci: google_pay_data.eci_indicator.clone(),
                    payment_brand,
                };

                Ok(Self::AciNetworkToken(Box::new(aci_network_token_data)))
            }
            // Fall back to encrypted passthrough — let ACI decrypt
            Err(_) => {
                let encrypted_token = google_pay_wallet_data
                    .tokenization_data
                    .get_encrypted_google_pay_token()
                    .map_err(|_| errors::ConnectorError::MissingRequiredField {
                        field_name: "google_pay.tokenization_data (encrypted)",
                    })?;

                Ok(Self::AciGooglePayEncrypted(Box::new(
                    AciGooglePayEncryptedData {
                        payment_token: Secret::new(encrypted_token),
                        source: "web".to_string(),
                    },
                )))
            }
        }
    }
}

/// Convert Samsung Pay encrypted token to ACI Samsung Pay format
impl TryFrom<&SamsungPayWalletData> for PaymentDetails {
    type Error = Error;
    fn try_from(samsung_pay_data: &SamsungPayWalletData) -> Result<Self, Self::Error> {
        let payment_brand =
            parse_samsung_pay_card_brand(&samsung_pay_data.payment_credential.card_brand)?;

        let aci_samsung_pay_data = AciSamsungPayData {
            payment_token: samsung_pay_data.payment_credential.token_data.data.clone(),
            source: "app".to_string(),
            payment_brand,
        };

        Ok(Self::AciSamsungPay(Box::new(aci_samsung_pay_data)))
    }
}

impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &BankRedirectData,
    )> for PaymentDetails
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let payment_data = match bank_redirect_data {
            BankRedirectData::Eps { .. } => Self::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Eps,
                bank_account_country: Some(item.router_data.get_billing_country()?),
                bank_account_bank_name: None,
                bank_account_bic: None,
                bank_account_iban: None,
                billing_country: None,
                merchant_customer_id: None,
                merchant_transaction_id: None,
                customer_email: None,
            })),
            BankRedirectData::Eft { .. } => Self::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Eft,
                bank_account_country: Some(item.router_data.get_billing_country()?),
                bank_account_bank_name: None,
                bank_account_bic: None,
                bank_account_iban: None,
                billing_country: None,
                merchant_customer_id: None,
                merchant_transaction_id: None,
                customer_email: None,
            })),
            BankRedirectData::Giropay {
                bank_account_bic,
                bank_account_iban,
                ..
            } => Self::BankRedirect(Box::new(BankRedirectionPMData {
                payment_brand: PaymentBrand::Giropay,
                bank_account_country: Some(item.router_data.get_billing_country()?),
                bank_account_bank_name: None,
                bank_account_bic: bank_account_bic.clone(),
                bank_account_iban: bank_account_iban.clone(),
                billing_country: None,
                merchant_customer_id: None,
                merchant_transaction_id: None,
                customer_email: None,
            })),
            BankRedirectData::Ideal { bank_name, .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Ideal,
                    bank_account_country: Some(item.router_data.get_billing_country()?),
                    bank_account_bank_name: Some(bank_name.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "ideal.bank_name",
                        },
                    )?),
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: None,
                }))
            }
            BankRedirectData::Sofort { .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Sofortueberweisung,
                    bank_account_country: Some(item.router_data.get_billing_country()?),
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: None,
                }))
            }
            BankRedirectData::Przelewy24 { .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Przelewy,
                    bank_account_country: None,
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: Some(item.router_data.get_billing_email()?),
                }))
            }
            BankRedirectData::Interac { .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::InteracOnline,
                    bank_account_country: Some(item.router_data.get_billing_country()?),
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: None,
                    merchant_customer_id: None,
                    merchant_transaction_id: None,
                    customer_email: Some(item.router_data.get_billing_email()?),
                }))
            }
            BankRedirectData::Trustly { .. } => {
                Self::BankRedirect(Box::new(BankRedirectionPMData {
                    payment_brand: PaymentBrand::Trustly,
                    bank_account_country: None,
                    bank_account_bank_name: None,
                    bank_account_bic: None,
                    bank_account_iban: None,
                    billing_country: Some(item.router_data.get_billing_country()?),
                    merchant_customer_id: Some(Secret::new(item.router_data.get_customer_id()?)),
                    merchant_transaction_id: Some(Secret::new(
                        item.router_data.connector_request_reference_id.chars().take(16).collect(),
                    )),
                    customer_email: None,
                }))
            }
            BankRedirectData::Bizum { .. }
            | BankRedirectData::Blik { .. }
            | BankRedirectData::BancontactCard { .. }
            | BankRedirectData::OnlineBankingCzechRepublic { .. }
            | BankRedirectData::OnlineBankingFinland { .. }
            | BankRedirectData::OnlineBankingFpx { .. }
            | BankRedirectData::OnlineBankingPoland { .. }
            | BankRedirectData::OnlineBankingSlovakia { .. }
            | BankRedirectData::OnlineBankingThailand { .. }
            | BankRedirectData::LocalBankRedirect {}
            | BankRedirectData::OpenBankingUk { .. }
            | BankRedirectData::OpenBanking { .. } => Err(errors::ConnectorError::NotImplemented(
                "Payment method".to_string(),
            ))?,
        };
        Ok(payment_data)
    }
}

fn get_aci_payment_brand(
    card_network: Option<common_enums::CardNetwork>,
    is_network_token_flow: bool,
) -> Result<PaymentBrand, Error> {
    match card_network {
        Some(common_enums::CardNetwork::Visa) => Ok(PaymentBrand::Visa),
        Some(common_enums::CardNetwork::Mastercard) => Ok(PaymentBrand::Mastercard),
        Some(common_enums::CardNetwork::AmericanExpress) => Ok(PaymentBrand::AmericanExpress),
        Some(common_enums::CardNetwork::JCB) => Ok(PaymentBrand::Jcb),
        Some(common_enums::CardNetwork::DinersClub) => Ok(PaymentBrand::DinersClub),
        Some(common_enums::CardNetwork::Discover) => Ok(PaymentBrand::Discover),
        Some(common_enums::CardNetwork::UnionPay) => Ok(PaymentBrand::UnionPay),
        Some(common_enums::CardNetwork::Maestro) => Ok(PaymentBrand::Maestro),
        Some(unsupported_network) => Err(errors::ConnectorError::NotSupported {
            message: format!("Card network {unsupported_network} is not supported by ACI"),
            connector: "ACI",
        })?,
        None => {
            if is_network_token_flow {
                Ok(PaymentBrand::Visa)
            } else {
                Err(errors::ConnectorError::MissingRequiredField {
                    field_name: "card.card_network",
                }
                .into())
            }
        }
    }
}

fn parse_wallet_card_network(network: &str) -> Option<PaymentBrand> {
    match network.to_lowercase().as_str() {
        "visa" => Some(PaymentBrand::Visa),
        "mastercard" => Some(PaymentBrand::Mastercard),
        "amex" | "americanexpress" | "american express" => Some(PaymentBrand::AmericanExpress),
        "jcb" => Some(PaymentBrand::Jcb),
        "diners" | "dinersclub" | "diners club" => Some(PaymentBrand::DinersClub),
        "discover" => Some(PaymentBrand::Discover),
        "unionpay" | "union pay" => Some(PaymentBrand::UnionPay),
        "maestro" => Some(PaymentBrand::Maestro),
        _ => None,
    }
}

fn parse_samsung_pay_card_brand(
    card_brand: &common_enums::SamsungPayCardBrand,
) -> Result<PaymentBrand, Error> {
    match card_brand {
        common_enums::SamsungPayCardBrand::Visa => Ok(PaymentBrand::Visa),
        common_enums::SamsungPayCardBrand::MasterCard => Ok(PaymentBrand::Mastercard),
        common_enums::SamsungPayCardBrand::Amex => Ok(PaymentBrand::AmericanExpress),
        common_enums::SamsungPayCardBrand::Discover => Ok(PaymentBrand::Discover),
        common_enums::SamsungPayCardBrand::Unknown => {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "samsung_pay.card_brand",
            })?
        }
    }
}

fn get_apple_pay_data(
    apple_pay_wallet_data: &ApplePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Result<ApplePayPredecryptData, error_stack::Report<errors::ConnectorError>> {
    if let Some(PaymentMethodToken::ApplePayDecrypt(decrypted_data)) = payment_method_token {
        return Ok(*decrypted_data.clone());
    }

    match &apple_pay_wallet_data.payment_data {
        common_types::payments::ApplePayPaymentData::Decrypted(decrypted_data) => {
            Ok(decrypted_data.clone())
        }
        common_types::payments::ApplePayPaymentData::Encrypted(_) => {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "decrypted apple pay data",
            })?
        }
    }
}

fn get_google_pay_data(
    google_pay_wallet_data: &GooglePayWalletData,
    payment_method_token: Option<&PaymentMethodToken>,
) -> Result<GPayPredecryptData, error_stack::Report<errors::ConnectorError>> {
    if let Some(PaymentMethodToken::GooglePayDecrypt(decrypted_data)) = payment_method_token {
        return Ok(*decrypted_data.clone());
    }

    match &google_pay_wallet_data.tokenization_data {
        common_types::payments::GpayTokenizationData::Decrypted(decrypted_data) => {
            Ok(decrypted_data.clone())
        }
        common_types::payments::GpayTokenizationData::Encrypted(_) => {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "decrypted google pay data",
            })?
        }
    }
}

impl TryFrom<(Card, Option<Secret<String>>)> for PaymentDetails {
    type Error = Error;
    fn try_from(
        (card_data, card_holder_name): (Card, Option<Secret<String>>),
    ) -> Result<Self, Self::Error> {
        let card_expiry_year = card_data.get_expiry_year_4_digit();

        let payment_brand = get_aci_payment_brand(card_data.card_network, false).ok();

        Ok(Self::AciCard(Box::new(CardDetails {
            card_number: card_data.card_number,
            card_holder: card_holder_name.ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "card_holder_name",
            })?,
            card_expiry_month: card_data.card_exp_month.clone(),
            card_expiry_year,
            card_cvv: card_data.card_cvc,
            payment_brand,
        })))
    }
}

impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &NetworkTokenData,
    )> for PaymentDetails
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &NetworkTokenData,
        ),
    ) -> Result<Self, Self::Error> {
        let (_item, network_token_data) = value;
        let token_number = network_token_data.get_network_token();
        let payment_brand = get_aci_payment_brand(network_token_data.card_network.clone(), true)?;
        let aci_network_token_data = AciNetworkTokenData {
            token_type: AciTokenAccountType::Network,
            token_number,
            token_expiry_month: network_token_data.get_network_token_expiry_month(),
            token_expiry_year: network_token_data.get_expiry_year_4_digit(),
            token_cryptogram: Some(
                network_token_data
                    .get_cryptogram()
                    .clone()
                    .unwrap_or_default(),
            ),
            eci: network_token_data.eci.clone(),
            payment_brand,
        };
        Ok(Self::AciNetworkToken(Box::new(aci_network_token_data)))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciTokenAccountType {
    Network,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciNetworkTokenData {
    #[serde(rename = "tokenAccount.type")]
    pub token_type: AciTokenAccountType,
    #[serde(rename = "tokenAccount.number")]
    pub token_number: NetworkToken,
    #[serde(rename = "tokenAccount.expiryMonth")]
    pub token_expiry_month: Secret<String>,
    #[serde(rename = "tokenAccount.expiryYear")]
    pub token_expiry_year: Secret<String>,
    #[serde(rename = "tokenAccount.cryptogram")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_cryptogram: Option<Secret<String>>,
    #[serde(rename = "threeDSecure.eci")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eci: Option<String>,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciApplePayEncryptedData {
    #[serde(rename = "applePay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: indicates the Apple Pay integration channel.
    /// "web" for browser-based flows, "app" for native iOS app flows.
    #[serde(rename = "applePay.source")]
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciGooglePayEncryptedData {
    #[serde(rename = "googlePay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: indicates the Google Pay integration channel.
    /// "web" for browser-based flows, "app" for native Android app flows.
    #[serde(rename = "googlePay.source")]
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciSamsungPayData {
    #[serde(rename = "samsungPay.paymentToken")]
    pub payment_token: Secret<String>,
    /// Required by ACI: Samsung Pay is always app-based.
    #[serde(rename = "samsungPay.source")]
    pub source: String,
    #[serde(rename = "paymentBrand")]
    pub payment_brand: PaymentBrand,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BankRedirectionPMData {
    payment_brand: PaymentBrand,
    #[serde(rename = "bankAccount.country")]
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_account_country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "bankAccount.bankName")]
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_account_bank_name: Option<common_enums::BankNames>,
    #[serde(rename = "bankAccount.bic")]
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_account_bic: Option<Secret<String>>,
    #[serde(rename = "bankAccount.iban")]
    #[serde(skip_serializing_if = "Option::is_none")]
    bank_account_iban: Option<Secret<String>>,
    #[serde(rename = "billing.country")]
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_country: Option<api_models::enums::CountryAlpha2>,
    #[serde(rename = "customer.email")]
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_email: Option<Email>,
    #[serde(rename = "customer.merchantCustomerId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_customer_id: Option<Secret<id_type::CustomerId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletPMData {
    payment_brand: PaymentBrand,
    #[serde(rename = "virtualAccount.accountId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    account_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentBrand {
    Eps,
    Eft,
    Ideal,
    Giropay,
    Sofortueberweisung,
    InteracOnline,
    Przelewy,
    Trustly,
    Mbway,
    #[serde(rename = "ALIPAY")]
    AliPay,
    // Card network brands
    #[serde(rename = "VISA")]
    Visa,
    #[serde(rename = "MASTER")]
    Mastercard,
    #[serde(rename = "AMEX")]
    AmericanExpress,
    #[serde(rename = "JCB")]
    Jcb,
    #[serde(rename = "DINERS")]
    DinersClub,
    #[serde(rename = "DISCOVER")]
    Discover,
    #[serde(rename = "UNIONPAY")]
    UnionPay,
    #[serde(rename = "MAESTRO")]
    Maestro,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct CardDetails {
    #[serde(rename = "card.number")]
    pub card_number: cards::CardNumber,
    #[serde(rename = "card.holder")]
    pub card_holder: Secret<String>,
    #[serde(rename = "card.expiryMonth")]
    pub card_expiry_month: Secret<String>,
    #[serde(rename = "card.expiryYear")]
    pub card_expiry_year: Secret<String>,
    #[serde(rename = "card.cvv")]
    pub card_cvv: Secret<String>,
    #[serde(rename = "paymentBrand")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_brand: Option<PaymentBrand>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionMode {
    Initial,
    Subsequent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum InstructionType {
    Unscheduled,
    Recurring,
    Installment,
}

#[derive(Debug, Clone, Serialize)]
pub enum InstructionSource {
    #[serde(rename = "CIT")]
    CardholderInitiatedTransaction,
    #[serde(rename = "MIT")]
    MerchantInitiatedTransaction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(rename = "standingInstruction.mode")]
    mode: InstructionMode,

    #[serde(rename = "standingInstruction.type")]
    transaction_type: InstructionType,

    #[serde(rename = "standingInstruction.source")]
    source: InstructionSource,

    #[serde(rename = "standingInstruction.initialTransactionId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_transaction_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    create_registration: Option<bool>,

    #[serde(rename = "standingInstruction.reason")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<StandingInstructionReason>,

    #[serde(rename = "standingInstruction.expiry")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,

    #[serde(rename = "standingInstruction.frequency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct BankDetails {
    #[serde(rename = "bankAccount.holder")]
    pub account_holder: Secret<String>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum AciPaymentType {
    #[serde(rename = "PA")]
    Preauthorization,
    #[default]
    #[serde(rename = "DB")]
    Debit,
    #[serde(rename = "CD")]
    Credit,
    #[serde(rename = "CP")]
    Capture,
    #[serde(rename = "RV")]
    Reversal,
    #[serde(rename = "RF")]
    Refund,
}

impl TryFrom<&AciRouterData<&PaymentsAuthorizeRouterData>> for AciPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AciRouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ref card_data) => Self::try_from((item, card_data)),
            PaymentMethodData::NetworkToken(ref network_token_data) => {
                Self::try_from((item, network_token_data))
            }
            PaymentMethodData::Wallet(ref wallet_data) => Self::try_from((item, wallet_data)),
            PaymentMethodData::PayLater(ref pay_later_data) => {
                Self::try_from((item, pay_later_data))
            }
            PaymentMethodData::BankRedirect(ref bank_redirect_data) => {
                Self::try_from((item, bank_redirect_data))
            }
            PaymentMethodData::MandatePayment => {
                let mandate_id = item.router_data.request.mandate_id.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "mandate_id",
                    },
                )?;
                Self::try_from((item, mandate_id))
            }
            PaymentMethodData::Crypto(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardWithOptionalCVC(_)
            | PaymentMethodData::CardWithNetworkTokenDetails(_)
            | PaymentMethodData::CardWithLimitedDetails(_)
            | PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
            | PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Aci"),
                ))?
            }
        }
    }
}

impl TryFrom<(&AciRouterData<&PaymentsAuthorizeRouterData>, &WalletData)> for AciPaymentsRequest {
    type Error = Error;
    fn try_from(
        value: (&AciRouterData<&PaymentsAuthorizeRouterData>, &WalletData),
    ) -> Result<Self, Self::Error> {
        let (item, wallet_data) = value;
        let txn_details = get_transaction_details(item)?;

        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        match wallet_data {
            WalletData::ApplePay(apple_pay_data) => {
                let payment_method_token = item.router_data.payment_method_token.as_ref();
                let payment_method =
                    PaymentDetails::try_from((item, apple_pay_data, payment_method_token))?;
                let instruction = get_instruction_details(item);

                Ok(Self {
                    txn_details,
                    payment_method,
                    instruction,
                    shopper_result_url: item.router_data.request.router_return_url.clone(),
                    three_ds_two_enrolled: None,
                    merchant_transaction_id,
                    customer_browser_info,
                    customer_data,
                    billing_address,
                    external_three_ds,
                    custom_parameters,
                })
            }
            WalletData::GooglePay(google_pay_data) => {
                let payment_method_token = item.router_data.payment_method_token.as_ref();
                let payment_method =
                    PaymentDetails::try_from((item, google_pay_data, payment_method_token))?;
                let instruction = get_instruction_details(item);

                Ok(Self {
                    txn_details,
                    payment_method,
                    instruction,
                    shopper_result_url: item.router_data.request.router_return_url.clone(),
                    three_ds_two_enrolled: None,
                    merchant_transaction_id,
                    customer_browser_info,
                    customer_data,
                    billing_address,
                    external_three_ds,
                    custom_parameters,
                })
            }
            WalletData::SamsungPay(samsung_pay_data) => {
                let payment_method = PaymentDetails::try_from(samsung_pay_data.as_ref())?;
                let instruction = get_instruction_details(item);

                Ok(Self {
                    txn_details,
                    payment_method,
                    instruction,
                    shopper_result_url: item.router_data.request.router_return_url.clone(),
                    three_ds_two_enrolled: None,
                    merchant_transaction_id,
                    customer_browser_info,
                    customer_data,
                    billing_address,
                    external_three_ds,
                    custom_parameters,
                })
            }
            // Handle other wallet types via PaymentDetails::try_from
            _ => {
                let payment_method = PaymentDetails::try_from((wallet_data, item.router_data))?;

                Ok(Self {
                    txn_details,
                    payment_method,
                    instruction: None,
                    shopper_result_url: item.router_data.request.router_return_url.clone(),
                    three_ds_two_enrolled: None,
                    merchant_transaction_id,
                    customer_browser_info,
                    customer_data,
                    billing_address,
                    external_three_ds,
                    custom_parameters,
                })
            }
        }
    }
}

impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &BankRedirectData,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &BankRedirectData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, bank_redirect_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from((item, bank_redirect_data))?;
        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        Ok(Self {
            txn_details,
            payment_method,
            instruction: None,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
            three_ds_two_enrolled: None,
            merchant_transaction_id,
            customer_browser_info,
            customer_data,
            billing_address,
            external_three_ds,
            custom_parameters,
        })
    }
}

impl TryFrom<(&AciRouterData<&PaymentsAuthorizeRouterData>, &PayLaterData)> for AciPaymentsRequest {
    type Error = Error;
    fn try_from(
        value: (&AciRouterData<&PaymentsAuthorizeRouterData>, &PayLaterData),
    ) -> Result<Self, Self::Error> {
        let (item, _pay_later_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::Klarna;
        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        Ok(Self {
            txn_details,
            payment_method,
            instruction: None,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
            three_ds_two_enrolled: None,
            merchant_transaction_id,
            customer_browser_info,
            customer_data,
            billing_address,
            external_three_ds,
            custom_parameters,
        })
    }
}

impl TryFrom<(&AciRouterData<&PaymentsAuthorizeRouterData>, &Card)> for AciPaymentsRequest {
    type Error = Error;
    fn try_from(
        value: (&AciRouterData<&PaymentsAuthorizeRouterData>, &Card),
    ) -> Result<Self, Self::Error> {
        let (item, card_data) = value;
        let card_holder_name = item.router_data.get_optional_billing_full_name();
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from((card_data.clone(), card_holder_name))?;
        let instruction = get_instruction_details(item);
        let three_ds_two_enrolled = item
            .router_data
            .is_three_ds()
            .then_some(item.router_data.request.enrolled_for_3ds);
        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        Ok(Self {
            txn_details,
            payment_method,
            instruction,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
            three_ds_two_enrolled,
            merchant_transaction_id,
            customer_browser_info,
            customer_data,
            billing_address,
            external_three_ds,
            custom_parameters,
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        &NetworkTokenData,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            &NetworkTokenData,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, network_token_data) = value;
        let txn_details = get_transaction_details(item)?;
        let payment_method = PaymentDetails::try_from((item, network_token_data))?;
        let instruction = get_instruction_details(item);
        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        Ok(Self {
            txn_details,
            payment_method,
            instruction,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
            three_ds_two_enrolled: None,
            merchant_transaction_id,
            customer_browser_info,
            customer_data,
            billing_address,
            external_three_ds,
            custom_parameters,
        })
    }
}

impl
    TryFrom<(
        &AciRouterData<&PaymentsAuthorizeRouterData>,
        api_models::payments::MandateIds,
    )> for AciPaymentsRequest
{
    type Error = Error;
    fn try_from(
        value: (
            &AciRouterData<&PaymentsAuthorizeRouterData>,
            api_models::payments::MandateIds,
        ),
    ) -> Result<Self, Self::Error> {
        let (item, _mandate_data) = value;
        let instruction = get_instruction_details(item);
        let txn_details = get_transaction_details(item)?;
        let (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters) =
            get_common_payment_fields(item);

        Ok(Self {
            txn_details,
            payment_method: PaymentDetails::Mandate,
            instruction,
            shopper_result_url: item.router_data.request.router_return_url.clone(),
            three_ds_two_enrolled: None,
            merchant_transaction_id,
            customer_browser_info,
            customer_data,
            billing_address,
            external_three_ds,
            custom_parameters,
        })
    }
}

fn get_common_payment_fields(
    item: &AciRouterData<&PaymentsAuthorizeRouterData>,
) -> (AciCustomerBrowserInfo, AciCustomerData, AciBillingAddress, AciExternalThreeDsData, Option<String>, AciCustomParameters) {
    let customer_browser_info = item
        .router_data
        .request
        .get_browser_info()
        .ok()
        .map(|bi| AciCustomerBrowserInfo {
            accept_header: bi.accept_header,
            language: bi.language,
            screen_height: bi.screen_height.map(|h| h.to_string()),
            screen_width: bi.screen_width.map(|w| w.to_string()),
            timezone: bi.time_zone.map(|t| t.to_string()),
            user_agent: bi.user_agent,
            javascript_enabled: bi.java_script_enabled,
            java_enabled: bi.java_enabled,
            color_depth: bi.color_depth.map(|c| c.to_string()),
            challenge_window: None,
        })
        .unwrap_or_default();

    let ip = item.router_data.request.get_ip_address_as_optional();
    let email = item.router_data.get_optional_billing_email();
    let given_name = item.router_data.get_optional_billing_first_name();
    let surname = item.router_data.get_optional_billing_last_name();

    let customer_data = AciCustomerData {
        ip,
        email,
        given_name,
        surname,
    };

    let billing_address = AciBillingAddress {
        street1: item.router_data.get_optional_billing_line1(),
        city: item.router_data.get_optional_billing_city(),
        state: item.router_data.get_optional_billing_state(),
        country: item.router_data.get_optional_billing_country(),
        postcode: item.router_data.get_optional_billing_zip(),
    };

    let external_three_ds = item
        .router_data
        .request
        .authentication_data
        .as_ref()
        .map(|auth| AciExternalThreeDsData {
            eci: auth.eci.clone(),
            verification_id: Some(auth.cavv.clone()),
            ds_transaction_id: auth.ds_trans_id.clone(),
            acs_transaction_id: auth.acs_trans_id.clone(),
        })
        .unwrap_or_default();

    // ACI acquirers enforce a 16-character max on merchantTransactionId.
    let merchant_transaction_id = {
        let id = &item.router_data.connector_request_reference_id;
        Some(id.chars().take(16).collect::<String>())
    };

    // Build customParameters: merchant metadata first, then fixed debug fields
    // (fixed fields always win over any conflicting metadata key).
    let mut custom_parameters = AciCustomParameters::default();
    if let Some(metadata) = &item.router_data.request.metadata {
        if let Some(obj) = metadata.as_object() {
            for (k, v) in obj {
                let str_val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => continue,
                    other => other.to_string(),
                };
                custom_parameters.insert(k, str_val);
            }
        }
    }
    custom_parameters.insert("orchestrator", "hyperswitch");
    custom_parameters.insert("paymentId", &item.router_data.payment_id);

    (customer_browser_info, customer_data, billing_address, external_three_ds, merchant_transaction_id, custom_parameters)
}

fn get_transaction_details(
    item: &AciRouterData<&PaymentsAuthorizeRouterData>,
) -> Result<TransactionDetails, error_stack::Report<errors::ConnectorError>> {
    let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;
    let payment_type = if item.router_data.request.is_auto_capture()? {
        AciPaymentType::Debit
    } else {
        AciPaymentType::Preauthorization
    };
    Ok(TransactionDetails {
        entity_id: auth.entity_id,
        amount: item.amount.to_owned(),
        currency: item.router_data.request.currency.to_string(),
        payment_type,
    })
}

fn get_instruction_details(
    item: &AciRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<Instruction> {
    if item.router_data.request.customer_acceptance.is_some() {
        return Some(Instruction {
            mode: InstructionMode::Initial,
            transaction_type: InstructionType::Unscheduled,
            source: InstructionSource::CardholderInitiatedTransaction,
            initial_transaction_id: None,
            create_registration: Some(true),
            reason: None,
            expiry: None,
            frequency: None,
        });
    } else if item.router_data.request.mandate_id.is_some() {
        // For MIT: pass the CITI (traceId) as standingInstruction.initialTransactionId.
        // Primary source: connector_mandate_request_reference_id (CITI stored on ACI registration).
        // Fallback: NetworkMandateId (CITI stored as network_txn_id on DB-only mandates).
        let initial_transaction_id = item
            .router_data
            .request
            .get_connector_mandate_request_reference_id()
            .ok()
            .or_else(|| {
                item.router_data
                    .request
                    .mandate_id
                    .as_ref()
                    .and_then(|m| match &m.mandate_reference_id {
                        Some(MandateReferenceId::NetworkMandateId(ntid)) => Some(ntid.clone()),
                        _ => None,
                    })
            });

        let transaction_type = match item.router_data.request.mit_category.as_ref() {
            Some(MitCategory::Installment) => InstructionType::Installment,
            Some(MitCategory::Recurring) => InstructionType::Recurring,
            Some(MitCategory::Unscheduled) | Some(MitCategory::Resubmission) | None => {
                InstructionType::Unscheduled
            }
        };

        let reason = match item.router_data.request.mit_category.as_ref() {
            Some(MitCategory::Resubmission) => Some(StandingInstructionReason::Resubmission),
            _ => None,
        };

        return Some(Instruction {
            mode: InstructionMode::Subsequent,
            transaction_type,
            source: InstructionSource::MerchantInitiatedTransaction,
            initial_transaction_id,
            create_registration: None,
            reason,
            expiry: None,
            frequency: None,
        });
    } else if matches!(
        item.router_data.request.setup_future_usage,
        Some(enums::FutureUsage::OffSession)
    ) {
        // CIT with setup_future_usage=off_session but no ACI registration (DB-only mandate).
        // Still required to send standingInstruction.mode=INITIAL so the acquirer marks
        // this transaction as the first in a COF/standing-instruction series.
        return Some(Instruction {
            mode: InstructionMode::Initial,
            transaction_type: InstructionType::Unscheduled,
            source: InstructionSource::CardholderInitiatedTransaction,
            initial_transaction_id: None,
            create_registration: None,
            reason: None,
            expiry: None,
            frequency: None,
        });
    }
    None
}


impl TryFrom<&PaymentsCancelRouterData> for AciCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        let aci_payment_request = Self {
            entity_id: auth.entity_id,
            payment_type: AciPaymentType::Reversal,
        };
        Ok(aci_payment_request)
    }
}

impl TryFrom<&RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>>
    for AciMandateRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;

        let (payment_brand, payment_details) = match &item.request.payment_method_data {
            PaymentMethodData::Card(card_data) => {
                let brand = get_aci_payment_brand(card_data.card_network.clone(), false).ok();
                match brand.as_ref() {
                    Some(PaymentBrand::Visa)
                    | Some(PaymentBrand::Mastercard)
                    | Some(PaymentBrand::AmericanExpress) => (),
                    Some(_) => {
                        return Err(errors::ConnectorError::NotSupported {
                            message: "Payment method not supported for mandate setup".to_string(),
                            connector: "ACI",
                        }
                        .into());
                    }
                    None => (),
                };

                let details = PaymentDetails::AciCard(Box::new(CardDetails {
                    card_number: card_data.card_number.clone(),
                    card_expiry_month: card_data.card_exp_month.clone(),
                    card_expiry_year: card_data.get_expiry_year_4_digit(),
                    card_cvv: card_data.card_cvc.clone(),
                    card_holder: card_data.card_holder_name.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "card_holder_name",
                        },
                    )?,
                    payment_brand: brand.clone(),
                }));

                (brand, details)
            }
            _ => {
                return Err(errors::ConnectorError::NotSupported {
                    message: "Payment method not supported for mandate setup".to_string(),
                    connector: "ACI",
                }
                .into());
            }
        };

        Ok(Self {
            entity_id: auth.entity_id,
            payment_brand,
            payment_details,
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AciPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RedirectShopper,
}

fn map_aci_attempt_status<T: AttemptStatusMapper>(
    request: &T,
    item: AciPaymentStatus,
    auto_capture: bool,
) -> enums::AttemptStatus {
    match item {
        AciPaymentStatus::Succeeded => request.map_success_status(auto_capture),
        AciPaymentStatus::Failed => enums::AttemptStatus::Failure,
        AciPaymentStatus::Pending => enums::AttemptStatus::Authorizing,
        AciPaymentStatus::RedirectShopper => enums::AttemptStatus::AuthenticationPending,
    }
}

impl FromStr for AciPaymentStatus {
    type Err = error_stack::Report<errors::ConnectorError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if FAILURE_CODES.contains(&s) {
            Ok(Self::Failed)
        } else if PENDING_CODES.contains(&s) {
            Ok(Self::Pending)
        } else if SUCCESSFUL_CODES.contains(&s) {
            Ok(Self::Succeeded)
        } else {
            Err(report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(s.to_owned())
            )))
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AciRiskScore {
    pub score: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciThreeDSecureResponse {
    pub eci: Option<String>,
    pub verification_id: Option<Secret<String>>,
    pub version: Option<String>,
    pub flow: Option<String>,
    pub ds_transaction_id: Option<String>,
    pub acs_transaction_id: Option<String>,
    pub challenge_mandated_indicator: Option<String>,
    pub authentication_type: Option<String>,
    pub card_holder_info: Option<String>,
    pub authentication_status: Option<String>,
    pub xid: Option<String>,
    pub cavv: Option<String>,
    pub error_code: Option<String>,
    pub error_description: Option<String>,
    pub error_source: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentsResponse {
    id: String,
    /// Registration token ID returned when `createRegistration=true` was sent.
    /// ACI returns this as `registrationId` at the top level.
    registration_id: Option<Secret<String>>,
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
    pub(super) redirect: Option<AciRedirectionData>,
    /// The payment type from the response (DB, PA, CP, RF, etc.)
    payment_type: Option<AciPaymentType>,
    /// The card network / payment brand (VISA, MASTERCARD, etc.)
    payment_brand: Option<PaymentBrand>,
    /// The processed amount
    amount: Option<StringMajorUnit>,
    /// The processed currency (ISO 4217)
    currency: Option<String>,
    /// Merchant-facing human-readable short transaction ID
    short_id: Option<String>,
    /// Acquirer / connector response details
    result_details: Option<AciResponseResultDetails>,
    /// Masked card details returned in the response
    card: Option<AciResponseCardDetails>,
    /// Risk score from ACI
    #[serde(skip_serializing)]
    risk: Option<AciRiskScore>,
    /// 3DS authentication data from ACI
    #[serde(rename = "threeDSecure")]
    #[serde(skip_serializing)]
    three_d_secure: Option<AciThreeDSecureResponse>,
}

/// Masked card details returned in an ACI payment response.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciResponseCardDetails {
    pub bin: Option<String>,
    #[serde(rename = "last4Digits")]
    pub last4_digits: Option<String>,
    pub holder: Option<Secret<String>>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    #[serde(rename = "binCountry")]
    pub bin_country: Option<String>,
}

/// Acquirer / connector-level response details in an ACI payment response.
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AciResponseResultDetails {
    pub extended_description: Option<String>,
    #[serde(rename = "clearingInstituteName")]
    pub clearing_institute_name: Option<String>,
    #[serde(rename = "ConnectorTxID1")]
    pub connector_tx_id1: Option<String>,
    #[serde(rename = "ConnectorTxID2")]
    pub connector_tx_id2: Option<String>,
    #[serde(rename = "ConnectorTxID3")]
    pub connector_tx_id3: Option<String>,
    #[serde(rename = "AcquirerResponse")]
    pub acquirer_response: Option<String>,
    #[serde(rename = "AuthCode")]
    pub auth_code: Option<String>,
    #[serde(rename = "MerchantAdviceCode")]
    pub merchant_advice_code: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciErrorResponse {
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRedirectionData {
    pub method: Option<Method>,
    pub parameters: Vec<Parameters>,
    pub url: Url,
    pub preconditions: Option<Vec<PreconditionData>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreconditionData {
    pub method: Option<Method>,
    pub parameters: Vec<Parameters>,
    pub url: Url,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Parameters {
    pub name: String,
    pub value: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultCode {
    pub(super) code: String,
    pub(super) description: String,
    pub(super) parameter_errors: Option<Vec<ErrorParameters>>,
    #[serde(rename = "cvvResponse")]
    pub cvv_response: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct ErrorParameters {
    pub(super) name: String,
    pub(super) value: Option<String>,
    pub(super) message: String,
}

pub struct AciParsedConnectorIds {
    pub rrn: Option<String>,
    pub citi: Option<String>,
    pub stan: Option<String>,
    pub auth_code: Option<String>,
    pub original_transaction_id: Option<String>,
}

fn parse_connector_tx_ids(details: &AciResponseResultDetails) -> AciParsedConnectorIds {
    fn split_pipe(s: &str) -> Vec<String> {
        s.split('|').map(|s| s.to_string()).collect()
    }
    fn get(v: &[String], i: usize) -> Option<String> {
        v.get(i).filter(|s| !s.is_empty()).cloned()
    }
    let t1: Vec<String> = details
        .connector_tx_id1
        .as_deref()
        .map(split_pipe)
        .unwrap_or_default();
    let t2: Vec<String> = details
        .connector_tx_id2
        .as_deref()
        .map(split_pipe)
        .unwrap_or_default();
    let t3: Vec<String> = details
        .connector_tx_id3
        .as_deref()
        .map(split_pipe)
        .unwrap_or_default();

    AciParsedConnectorIds {
        rrn: get(&t2, 2),
        citi: get(&t3, 4),
        stan: get(&t3, 0).or_else(|| get(&t2, 1)),
        auth_code: details.auth_code.clone(),
        original_transaction_id: get(&t1, 5),
    }
}

impl<F, Req> TryFrom<ResponseRouterData<F, AciPaymentsResponse, Req, PaymentsResponseData>>
    for RouterData<F, Req, PaymentsResponseData>
where
    Req: AttemptStatusMapper,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AciPaymentsResponse, Req, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.redirect.map(|data| {
            let mut form_fields = std::collections::HashMap::<_, _>::from_iter(
                data.parameters
                    .iter()
                    .map(|parameter| (parameter.name.clone(), parameter.value.clone())),
            );

            if let Some(preconditions) = data.preconditions {
                if let Some(first_precondition) = preconditions.first() {
                    for param in &first_precondition.parameters {
                        form_fields.insert(param.name.clone(), param.value.clone());
                    }
                }
            }

            // If method is Get, parameters are appended to URL
            // If method is post, we http Post the method to URL
            RedirectForm::Form {
                endpoint: data.url.to_string(),
                // Handles method for Bank redirects currently.
                // 3DS response have method within preconditions. That would require replacing below line with a function.
                method: data.method.unwrap_or(Method::Post),
                form_fields,
            }
        });

        // Parse connector transaction IDs for RRN, CITI (network_txn_id), and auth_code
        let parsed_ids = item
            .response
            .result_details
            .as_ref()
            .map(parse_connector_tx_ids);

        let rrn = parsed_ids.as_ref().and_then(|p| p.rrn.clone());
        let citi = parsed_ids.as_ref().and_then(|p| p.citi.clone());
        let stan = parsed_ids.as_ref().and_then(|p| p.stan.clone());
        let parsed_auth_code = parsed_ids.as_ref().and_then(|p| p.auth_code.clone());
        let original_transaction_id = parsed_ids.as_ref().and_then(|p| p.original_transaction_id.clone());
        let connector_response_reference_id =
            rrn.clone().or_else(|| Some(item.response.id.clone()));

        // Build mandate reference from registrationId (returned at top level when createRegistration=true).
        // connector_mandate_id = ACI registrationId (RG ID) — used for subsequent MIT lookup.
        // connector_mandate_request_reference_id = CITI (traceId) — passed as
        // standingInstruction.initialTransactionId on subsequent MIT requests.
        let mandate_reference = item
            .response
            .registration_id
            .clone()
            .map(|id| MandateReference {
                connector_mandate_id: Some(id.expose()),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: citi.clone(),
            });

        let status = if redirection_data.is_some() {
            enums::AttemptStatus::AuthenticationPending
        } else {
            let auto_capture = matches!(
                item.data.request.get_capture_method(),
                Some(enums::CaptureMethod::Automatic) | None
            );
            map_aci_attempt_status(
                &item.data.request,
                AciPaymentStatus::from_str(&item.response.result.code)?,
                auto_capture,
            )
        };

        // Build authentication_data from 3DS response if present
        let authentication_data = item.response.three_d_secure.as_ref().map(|tds| {
            Box::new(UcsAuthenticationData {
                eci: tds.eci.clone(),
                cavv: tds
                    .verification_id
                    .clone()
                    .or_else(|| tds.cavv.as_ref().map(|c| Secret::new(c.clone()))),
                threeds_server_transaction_id: tds.ds_transaction_id.clone(),
                message_version: tds
                    .version
                    .as_deref()
                    .and_then(|v| v.parse().ok()),
                ds_trans_id: tds.ds_transaction_id.clone(),
                acs_trans_id: tds.acs_transaction_id.clone(),
                trans_status: None,
                transaction_id: tds.xid.clone(),
                ucaf_collection_indicator: None,
            })
        });

        // Build ConnectorResponseData from auth_code, card_network, and payment_checks.
        // Use parsed_auth_code which covers both the direct AuthCode field and fallback
        // extraction from connector TX ID fields.
        let auth_code = parsed_auth_code;
        let card_network = item
            .response
            .payment_brand
            .as_ref()
            .map(|b| format!("{b:?}"));
        let payment_checks = {
            let cvv = item.response.result.cvv_response.clone();
            let acq = item
                .response
                .result_details
                .as_ref()
                .and_then(|d| d.acquirer_response.clone());
            if cvv.is_some() || acq.is_some() || stan.is_some() || original_transaction_id.is_some() {
                Some(serde_json::json!({
                    "cvvResponse": cvv,
                    "acquirerResponse": acq,
                    "stan": stan,
                    "originalTransactionId": original_transaction_id,
                }))
            } else {
                None
            }
        };
        let connector_response =
            if auth_code.is_some() || card_network.is_some() || payment_checks.is_some() {
                Some(ConnectorResponseData::with_additional_payment_method_data(
                    AdditionalPaymentMethodConnectorResponse::Card {
                        auth_code,
                        card_network,
                        payment_checks,
                        authentication_data: None,
                        domestic_network: None,
                    },
                ))
            } else {
                None
            };

        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: Some(item.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: citi,
                connector_response_reference_id,
                incremental_authorization_allowed: None,
                authentication_data,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            connector_response,
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCaptureRequest {
    #[serde(flatten)]
    pub txn_details: TransactionDetails,
}

impl TryFrom<&AciRouterData<&PaymentsCaptureRouterData>> for AciCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(item: &AciRouterData<&PaymentsCaptureRouterData>) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            txn_details: TransactionDetails {
                entity_id: auth.entity_id,
                amount: item.amount.to_owned(),
                currency: item.router_data.request.currency.to_string(),
                payment_type: AciPaymentType::Capture,
            },
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCaptureResponse {
    id: String,
    referenced_id: String,
    payment_type: AciPaymentType,
    amount: StringMajorUnit,
    currency: String,
    descriptor: String,
    result: AciCaptureResult,
    result_details: Option<AciResponseResultDetails>,
    build_number: String,
    timestamp: String,
    ndc: Secret<String>,
    source: Option<Secret<String>>,
    payment_method: Option<String>,
    short_id: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciCaptureResult {
    code: String,
    description: String,
}


#[derive(Debug, Default, Clone, Deserialize)]
pub enum AciStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
}

impl FromStr for AciStatus {
    type Err = error_stack::Report<errors::ConnectorError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if FAILURE_CODES.contains(&s) {
            Ok(Self::Failed)
        } else if PENDING_CODES.contains(&s) {
            Ok(Self::Pending)
        } else if SUCCESSFUL_CODES.contains(&s) {
            Ok(Self::Succeeded)
        } else {
            Err(report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(s.to_owned())
            )))
        }
    }
}

fn map_aci_capture_status(item: AciStatus) -> enums::AttemptStatus {
    match item {
        AciStatus::Succeeded => enums::AttemptStatus::Charged,
        AciStatus::Failed => enums::AttemptStatus::Failure,
        AciStatus::Pending => enums::AttemptStatus::Pending,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AciCaptureResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AciCaptureResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = map_aci_capture_status(AciStatus::from_str(&item.response.result.code)?);
        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: Some(item.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.referenced_id.clone()),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            })
        };
        Ok(Self {
            status,
            response,
            reference_id: Some(item.response.referenced_id),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciVoidResponse {
    id: String,
    referenced_id: String,
    payment_type: AciPaymentType,
    amount: StringMajorUnit,
    currency: String,
    descriptor: String,
    result: AciCaptureResult,
    result_details: Option<AciResponseResultDetails>,
    build_number: String,
    timestamp: String,
    ndc: Secret<String>,
}

fn map_aci_void_status(item: AciStatus) -> enums::AttemptStatus {
    match item {
        AciStatus::Succeeded => enums::AttemptStatus::Voided,
        AciStatus::Failed => enums::AttemptStatus::VoidFailed,
        AciStatus::Pending => enums::AttemptStatus::VoidInitiated,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, AciVoidResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, AciVoidResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = map_aci_void_status(AciStatus::from_str(&item.response.result.code)?);
        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id.clone()),
                ..Default::default()
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.referenced_id.clone()),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            })
        };
        Ok(Self {
            status,
            response,
            reference_id: Some(item.response.referenced_id),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundRequest {
    pub amount: StringMajorUnit,
    pub currency: String,
    pub payment_type: AciPaymentType,
    pub entity_id: Secret<String>,
}

impl<F> TryFrom<&AciRouterData<&RefundsRouterData<F>>> for AciRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &AciRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let amount = item.amount.to_owned();
        let currency = item.router_data.request.currency;
        let payment_type = AciPaymentType::Refund;
        let auth = AciAuthType::try_from(&item.router_data.connector_auth_type)?;

        Ok(Self {
            amount,
            currency: currency.to_string(),
            payment_type,
            entity_id: auth.entity_id,
        })
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub enum AciRefundStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
}

impl FromStr for AciRefundStatus {
    type Err = error_stack::Report<errors::ConnectorError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if FAILURE_CODES.contains(&s) {
            Ok(Self::Failed)
        } else if PENDING_CODES.contains(&s) {
            Ok(Self::Pending)
        } else if SUCCESSFUL_CODES.contains(&s) {
            Ok(Self::Succeeded)
        } else {
            Err(report!(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from(s.to_owned())
            )))
        }
    }
}

impl From<AciRefundStatus> for enums::RefundStatus {
    fn from(item: AciRefundStatus) -> Self {
        match item {
            AciRefundStatus::Succeeded => Self::Success,
            AciRefundStatus::Failed => Self::Failure,
            AciRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentReference {
    #[serde(rename = "referenceId")]
    pub reference_id: Option<String>,
    #[serde(rename = "type")]
    pub reference_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciRefundResponse {
    id: String,
    ndc: String,
    timestamp: String,
    build_number: String,
    pub(super) result: ResultCode,
    pub result_details: Option<AciResponseResultDetails>,
    pub references: Option<Vec<AciPaymentReference>>,
}

impl<F> TryFrom<RefundsResponseRouterData<F, AciRefundResponse>> for RefundsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<F, AciRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status =
            enums::RefundStatus::from(AciRefundStatus::from_str(&item.response.result.code)?);
        let response = if refund_status == enums::RefundStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            })
        };
        Ok(Self {
            response,
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            SetupMandate,
            AciMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            SetupMandate,
            AciMandateResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let mandate_reference = Some(MandateReference {
            connector_mandate_id: Some(item.response.id.clone()),
            payment_method_id: None,
            mandate_metadata: None,
            connector_mandate_request_reference_id: None,
        });

        let status = if SUCCESSFUL_CODES.contains(&item.response.result.code.as_str()) {
            enums::AttemptStatus::Charged
        } else if FAILURE_CODES.contains(&item.response.result.code.as_str()) {
            enums::AttemptStatus::Failure
        } else {
            enums::AttemptStatus::Pending
        };

        let response = if status == enums::AttemptStatus::Failure {
            Err(ErrorResponse {
                code: item.response.result.code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: Some(item.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

/// ACI sends webhook event types in UPPERCASE (e.g. "PAYMENT", "REGISTRATION").
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciWebhookEventType {
    Payment,
    Registration,
    Schedule,
    Risk,
}

/// ACI sends action values in UPPERCASE (e.g. "CREATED", "UPDATED", "DELETED").
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AciWebhookAction {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookCardDetails {
    pub bin: Option<String>,
    #[serde(rename = "last4Digits")]
    pub last4_digits: Option<String>,
    pub holder: Option<String>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookCustomerDetails {
    #[serde(rename = "givenName")]
    pub given_name: Option<Secret<String>>,
    pub surname: Option<Secret<String>>,
    #[serde(rename = "merchantCustomerId")]
    pub merchant_customer_id: Option<Secret<String>>,
    pub sex: Option<Secret<String>>,
    pub email: Option<Email>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookAuthenticationDetails {
    #[serde(rename = "entityId")]
    pub entity_id: Secret<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookRiskDetails {
    pub score: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciPaymentWebhookPayload {
    pub id: String,
    pub payment_type: String,
    pub payment_brand: String,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub presentation_amount: Option<StringMajorUnit>,
    pub presentation_currency: Option<String>,
    pub descriptor: Option<String>,
    pub result: ResultCode,
    pub authentication: Option<AciWebhookAuthenticationDetails>,
    pub card: Option<AciWebhookCardDetails>,
    pub customer: Option<AciWebhookCustomerDetails>,
    #[serde(rename = "customParameters")]
    pub custom_parameters: Option<serde_json::Value>,
    pub risk: Option<AciWebhookRiskDetails>,
    pub build_number: Option<String>,
    pub timestamp: String,
    pub ndc: String,
    #[serde(rename = "channelName")]
    pub channel_name: Option<String>,
    pub source: Option<String>,
    pub payment_method: Option<String>,
    #[serde(rename = "shortId")]
    pub short_id: Option<String>,
    /// Registration token ID returned when `createRegistration=true` was sent.
    pub registration_id: Option<Secret<String>>,
    /// Acquirer / connector-level result details.
    pub result_details: Option<AciResponseResultDetails>,
    /// 3DS authentication data from ACI
    #[serde(rename = "threeDSecure")]
    pub three_d_secure: Option<AciThreeDSecureResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AciWebhookNotification {
    #[serde(rename = "type")]
    pub event_type: AciWebhookEventType,
    pub action: Option<AciWebhookAction>,
    pub payload: serde_json::Value,
}

// ─── Standalone 3DS (/v1/threeDSecure) ───────────────────────────────────────

/// Request body for `POST /v1/threeDSecure` (standalone 3DS authentication).
#[derive(Debug, Serialize)]
pub struct AciStandaloneThreeDsRequest {
    #[serde(rename = "entityId")]
    pub entity_id: Secret<String>,
    /// Amount (optional for NPA flows).
    #[serde(rename = "amount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    /// Currency (optional for NPA flows).
    #[serde(rename = "currency")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "paymentType")]
    pub payment_type: AciPaymentType,
    #[serde(flatten)]
    pub card: CardDetails,
    /// Non-Payment Authentication flag.
    #[serde(rename = "threeDSecure.npa")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub three_ds_npa: Option<bool>,
    #[serde(rename = "shopperResultUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_result_url: Option<String>,
}

impl TryFrom<&crate::types::PreAuthNRouterData> for AciStandaloneThreeDsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &crate::types::PreAuthNRouterData,
    ) -> Result<Self, Self::Error> {
        let auth = AciAuthType::try_from(&item.connector_auth_type)?;
        let card = &item.request.card;
        let card_holder_name = card.card_holder_name.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "card_holder_name",
            },
        )?;
        let card_details = CardDetails {
            card_number: card.card_number.clone(),
            card_holder: card_holder_name,
            card_expiry_month: card.card_exp_month.clone(),
            card_expiry_year: card.get_expiry_year_4_digit(),
            card_cvv: card.card_cvc.clone(),
            payment_brand: get_aci_payment_brand(card.card_network.clone(), false).ok(),
        };
        Ok(Self {
            entity_id: auth.entity_id,
            amount: None,
            currency: None,
            payment_type: AciPaymentType::Preauthorization,
            card: card_details,
            three_ds_npa: Some(true),
            shopper_result_url: None,
        })
    }
}

/// Map ACI payment response to PreAuthentication RouterData response.
impl
    TryFrom<
        ResponseRouterData<
            PreAuthentication,
            AciPaymentsResponse,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
    > for RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            PreAuthentication,
            AciPaymentsResponse,
            PreAuthNRequestData,
            AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let result_code = &item.response.result.code;

        let response = if FAILURE_CODES.contains(&result_code.as_str()) {
            Err(ErrorResponse {
                code: result_code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: Some(item.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else if let Some(tds) = item.response.three_d_secure.as_ref() {
            // Frictionless: 3DS completed without redirect
            Ok(AuthenticationResponseData::AuthNResponse {
                authn_flow_type: AuthNFlowType::Frictionless,
                authentication_value: tds
                    .verification_id
                    .clone()
                    .or_else(|| tds.cavv.as_ref().map(|c| Secret::new(c.clone()))),
                trans_status: enums::TransactionStatus::Success,
                connector_metadata: None,
                ds_trans_id: tds.ds_transaction_id.clone(),
                eci: tds.eci.clone(),
                challenge_code: None,
                challenge_cancel: None,
                challenge_code_reason: None,
                message_extension: None,
            })
        } else {
            // Pending/redirect: challenge required
            let three_ds_method_url = item
                .response
                .redirect
                .as_ref()
                .and_then(|r| r.preconditions.as_ref())
                .and_then(|p| p.first())
                .map(|p| p.url.to_string());
            let version = item
                .response
                .three_d_secure
                .as_ref()
                .and_then(|tds| tds.version.as_ref())
                .and_then(|v| v.parse::<SemanticVersion>().ok())
                .unwrap_or(SemanticVersion::new(2, 0, 0));
            Ok(AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id: item.response.id.clone(),
                maximum_supported_3ds_version: version.clone(),
                connector_authentication_id: item.response.id.clone(),
                three_ds_method_data: None,
                three_ds_method_url,
                message_version: version,
                connector_metadata: None,
                directory_server_id: None,
                scheme_id: None,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

/// Map ACI payment response to PostAuthentication RouterData response.
impl
    TryFrom<
        ResponseRouterData<
            PostAuthentication,
            AciPaymentsResponse,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
    >
    for RouterData<
        PostAuthentication,
        ConnectorPostAuthenticationRequestData,
        AuthenticationResponseData,
    >
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<
            PostAuthentication,
            AciPaymentsResponse,
            ConnectorPostAuthenticationRequestData,
            AuthenticationResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let result_code = &item.response.result.code;

        let response = if FAILURE_CODES.contains(&result_code.as_str()) {
            Err(ErrorResponse {
                code: result_code.clone(),
                message: item.response.result.description.clone(),
                reason: Some(item.response.result.description),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                connector_response_reference_id: Some(item.response.id.clone()),
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let tds = item.response.three_d_secure.as_ref();
            Ok(AuthenticationResponseData::PostAuthNResponse {
                trans_status: if SUCCESSFUL_CODES.contains(&result_code.as_str()) {
                    enums::TransactionStatus::Success
                } else {
                    enums::TransactionStatus::Failure
                },
                authentication_value: tds.and_then(|t| {
                    t.verification_id
                        .clone()
                        .or_else(|| t.cavv.as_ref().map(|c| Secret::new(c.clone())))
                }),
                eci: tds.and_then(|t| t.eci.clone()),
                challenge_cancel: None,
                challenge_code_reason: None,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}
