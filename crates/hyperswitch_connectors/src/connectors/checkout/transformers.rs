use common_enums::{
    enums::{self, AttemptStatus, PaymentChannel},
    CountryAlpha2,
};
use common_utils::{
    errors::{CustomResult, ParsingError},
    ext_traits::ByteSliceExt,
    request::{Method, RequestContent},
    types::MinorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    payment_methods::storage_enums::MitCategory,
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::{Execute, RSync, SetupMandate},
    router_request_types::{ResponseId, SetupMandateRequestData},
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundsRouterData, TokenizationRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors, webhooks};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_with::skip_serializing_none;
use time::PrimitiveDateTime;
use url::Url;

use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        PaymentsResponseRouterData, PaymentsSyncResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData, SubmitEvidenceRouterData, UploadFileRouterData,
    },
    unimplemented_payment_method,
    utils::{
        self, PaymentsAuthorizeRequestData, PaymentsCaptureRequestData, PaymentsSyncRequestData,
        RouterData as OtherRouterData, WalletData as OtherWalletData,
    },
};

#[derive(Debug, Serialize)]
pub struct CheckoutRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for CheckoutRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "token_data")]
pub enum TokenRequest {
    Googlepay(CheckoutGooglePayData),
    Applepay(CheckoutApplePayData),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "token_data")]
pub enum PreDecryptedTokenRequest {
    Applepay(Box<CheckoutApplePayData>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutGooglePayData {
    protocol_version: Secret<String>,
    signature: Secret<String>,
    signed_message: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutApplePayData {
    version: Secret<String>,
    data: Secret<String>,
    signature: Secret<String>,
    header: CheckoutApplePayHeader,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutApplePayHeader {
    ephemeral_public_key: Secret<String>,
    public_key_hash: Secret<String>,
    transaction_id: Secret<String>,
}

impl TryFrom<&TokenizationRouterData> for TokenRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::Wallet(wallet_data) => match wallet_data.clone() {
                WalletData::GooglePay(_data) => {
                    let json_wallet_data: CheckoutGooglePayData =
                        wallet_data.get_wallet_token_as_json("Google Pay".to_string())?;
                    Ok(Self::Googlepay(json_wallet_data))
                }
                WalletData::ApplePay(_data) => {
                    let json_wallet_data: CheckoutApplePayData =
                        wallet_data.get_wallet_token_as_json("Apple Pay".to_string())?;
                    Ok(Self::Applepay(json_wallet_data))
                }
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPay(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::Paysera(_)
                | WalletData::Skrill(_)
                | WalletData::BluecodeRedirect {}
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
                | WalletData::GooglePayRedirect(_)
                | WalletData::GooglePayThirdPartySdk(_)
                | WalletData::MbWayRedirect(_)
                | WalletData::MobilePayRedirect(_)
                | WalletData::PaypalRedirect(_)
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::Mifinity(_)
                | WalletData::RevolutPay(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("checkout"),
                )
                .into()),
            },
            PaymentMethodData::Card(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("checkout"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CheckoutTokenResponse {
    token: Secret<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, CheckoutTokenResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, CheckoutTokenResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PaymentsResponseData::TokenizationResponse {
                token: item.response.token.expose(),
            }),
            ..item.data
        })
    }
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct CheckoutAddress {
    pub address_line1: Option<Secret<String>>,
    pub address_line2: Option<Secret<String>>,
    pub city: Option<String>,
    pub state: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub country: Option<CountryAlpha2>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct CheckoutAccountHolderDetails {
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Option<Secret<String>>,
    pub billing_address: Option<CheckoutAddress>,
    pub account_holder: Option<CheckoutAccountHolderDetails>,
}

#[derive(Debug, Serialize)]
pub struct WalletSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub token: Secret<String>,
    pub billing_address: Option<CheckoutAddress>,
}

#[derive(Debug, Serialize)]
pub struct MandateSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    #[serde(rename = "id")]
    pub source_id: Option<String>,
    pub billing_address: Option<CheckoutAddress>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentSource {
    Card(CardSource),
    Wallets(WalletSource),
    ApplePayPredecrypt(Box<ApplePayPredecrypt>),
    MandatePayment(MandateSource),
    GooglePayPredecrypt(Box<GooglePayPredecrypt>),
}

#[derive(Debug, Serialize)]
pub struct GooglePayPredecrypt {
    #[serde(rename = "type")]
    _type: String,
    token: cards::CardNumber,
    token_type: String,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    eci: String,
    cryptogram: Option<Secret<String>>,
    pub billing_address: Option<CheckoutAddress>,
}

#[derive(Debug, Serialize)]
pub struct ApplePayPredecrypt {
    token: cards::CardNumber,
    #[serde(rename = "type")]
    decrypt_type: String,
    token_type: String,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    eci: Option<String>,
    cryptogram: Secret<String>,
    pub billing_address: Option<CheckoutAddress>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutSourceTypes {
    Card,
    Token,
    #[serde(rename = "id")]
    SourceId,
}

#[derive(Debug, Serialize)]
pub enum CheckoutPaymentType {
    Regular,
    Unscheduled,
    #[serde(rename = "MOTO")]
    Moto,
    Installment,
    Recurring,
}

pub struct CheckoutAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) processing_channel_id: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

#[derive(Debug, Serialize)]
pub struct ReturnUrl {
    pub success_url: Option<String>,
    pub failure_url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutCustomer {
    pub name: Option<Secret<String>>,
    pub email: Option<common_utils::pii::Email>,
    pub phone: Option<CheckoutPhoneDetails>,
    pub tax_number: Option<Secret<String>>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutPhoneDetails {
    pub country_code: Option<String>,
    pub number: Option<Secret<String>>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutProcessing {
    pub order_id: Option<String>,
    pub tax_amount: Option<MinorUnit>,
    pub discount_amount: Option<MinorUnit>,
    pub duty_amount: Option<MinorUnit>,
    pub shipping_amount: Option<MinorUnit>,
    pub shipping_tax_amount: Option<MinorUnit>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutShipping {
    pub address: Option<CheckoutAddress>,
    pub from_address_zip: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutLineItem {
    pub commodity_code: Option<String>,
    pub discount_amount: Option<MinorUnit>,
    pub name: Option<String>,
    pub quantity: Option<u16>,
    pub reference: Option<String>,
    pub tax_exempt: Option<bool>,
    pub tax_amount: Option<MinorUnit>,
    pub total_amount: Option<MinorUnit>,
    pub unit_of_measure: Option<String>,
    pub unit_price: Option<MinorUnit>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutBillingDescriptor {
    pub name: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub reference: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize)]
pub struct PaymentsRequest {
    pub source: PaymentSource,
    pub amount: MinorUnit,
    pub currency: String,
    pub processing_channel_id: Secret<String>,
    #[serde(rename = "3ds")]
    pub three_ds: CheckoutThreeDS,
    #[serde(flatten)]
    pub return_url: ReturnUrl,
    pub capture: bool,
    pub reference: String,
    #[serde(skip_serializing_if = "is_metadata_empty")]
    pub metadata: Option<Secret<serde_json::Value>>,
    pub payment_type: CheckoutPaymentType,
    pub merchant_initiated: Option<bool>,
    pub previous_payment_id: Option<String>,
    pub store_for_future_use: Option<bool>,
    pub billing_descriptor: Option<CheckoutBillingDescriptor>,
    // Level 2/3 data fields
    pub customer: Option<CheckoutCustomer>,
    pub processing: Option<CheckoutProcessing>,
    pub shipping: Option<CheckoutShipping>,
    pub items: Option<Vec<CheckoutLineItem>>,
    pub partial_authorization: Option<CheckoutPartialAuthorization>,
    pub payment_ip: Option<Secret<String, common_utils::pii::IpAddress>>,
}

#[skip_serializing_none]
#[derive(Debug, Default, Serialize)]
pub struct CheckoutPartialAuthorization {
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutMeta {
    pub psync_flow: CheckoutPaymentIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum CheckoutPaymentIntent {
    Capture,
    Authorize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutChallengeIndicator {
    NoPreference,
    ChallengeRequestedMandate,
    ChallengeRequested,
    NoChallengeRequested,
}

#[derive(Debug, Serialize)]
pub struct CheckoutThreeDS {
    enabled: bool,
    force_3ds: bool,
    eci: Option<String>,
    cryptogram: Option<Secret<String>>,
    xid: Option<String>,
    version: Option<String>,
    challenge_indicator: CheckoutChallengeIndicator,
}

impl TryFrom<&ConnectorAuthType> for CheckoutAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            api_secret,
            key1,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                processing_channel_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

fn split_account_holder_name(
    card_holder_name: Option<Secret<String>>,
) -> (Option<Secret<String>>, Option<Secret<String>>) {
    let account_holder_name = card_holder_name
        .as_ref()
        .map(|name| name.clone().expose().trim().to_string());
    match account_holder_name {
        Some(name) if !name.is_empty() => match name.rsplit_once(' ') {
            Some((first, last)) => (
                Some(Secret::new(first.to_string())),
                Some(Secret::new(last.to_string())),
            ),
            None => (Some(Secret::new(name)), None),
        },
        _ => (None, None),
    }
}

fn build_metadata(
    item: &CheckoutRouterData<&PaymentsAuthorizeRouterData>,
) -> Option<Secret<serde_json::Value>> {
    // get metadata or create empty json object
    let mut metadata_json = item
        .router_data
        .request
        .metadata
        .clone()
        .unwrap_or_else(|| json!({}));

    // get udf5 value (name or integrator)
    let udf5 = item
        .router_data
        .request
        .partner_merchant_identifier_details
        .as_ref()
        .and_then(|p| p.partner_details.as_ref())
        .and_then(|e| e.name.clone().or(e.integrator.clone()));

    // insert udf5 if present
    if let Some(v) = udf5 {
        if let Some(obj) = metadata_json.as_object_mut() {
            obj.insert("udf5".to_string(), json!(v));
        } else {
            metadata_json = json!({ "udf5": v });
        }
    }

    Some(Secret::new(metadata_json))
}

fn is_metadata_empty(val: &Option<Secret<serde_json::Value>>) -> bool {
    match val {
        None => true,
        Some(secret) => {
            let inner = secret.clone().expose();
            match inner {
                serde_json::Value::Null => true,
                serde_json::Value::Object(map) => map.is_empty(),
                _ => false,
            }
        }
    }
}

impl TryFrom<&CheckoutRouterData<&PaymentsAuthorizeRouterData>> for PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CheckoutRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let capture = matches!(
            item.router_data.request.capture_method,
            Some(enums::CaptureMethod::Automatic)
        );

        let payment_type = if matches!(
            item.router_data.request.payment_channel,
            Some(PaymentChannel::MailOrder | PaymentChannel::TelephoneOrder)
        ) {
            CheckoutPaymentType::Moto
        } else if item.router_data.request.is_mandate_payment() {
            CheckoutPaymentType::Unscheduled
        } else {
            CheckoutPaymentType::Regular
        };

        let (challenge_indicator, store_for_future_use) =
            if item.router_data.request.is_mandate_payment() {
                (
                    CheckoutChallengeIndicator::ChallengeRequestedMandate,
                    Some(true),
                )
            } else {
                (CheckoutChallengeIndicator::ChallengeRequested, None)
            };

        let billing_details = Some(CheckoutAddress {
            city: item.router_data.get_optional_billing_city(),
            address_line1: item.router_data.get_optional_billing_line1(),
            address_line2: item.router_data.get_optional_billing_line2(),
            state: item.router_data.get_optional_billing_state(),
            zip: item.router_data.get_optional_billing_zip(),
            country: item.router_data.get_optional_billing_country(),
        });

        let (
            source_var,
            previous_payment_id,
            merchant_initiated,
            payment_type,
            store_for_future_use,
        ) = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                let (first_name, last_name) = split_account_holder_name(ccard.card_holder_name);

                let payment_source = PaymentSource::Card(CardSource {
                    source_type: CheckoutSourceTypes::Card,
                    number: ccard.card_number.clone(),
                    expiry_month: ccard.card_exp_month.clone(),
                    expiry_year: ccard.card_exp_year.clone(),
                    cvv: Some(ccard.card_cvc),
                    billing_address: billing_details,
                    account_holder: Some(CheckoutAccountHolderDetails {
                        first_name,
                        last_name,
                    }),
                });
                Ok((
                    payment_source,
                    None,
                    Some(false),
                    payment_type,
                    store_for_future_use,
                ))
            }
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(_) => {
                    let p_source = match item.router_data.get_payment_method_token()? {
                        PaymentMethodToken::Token(token) => PaymentSource::Wallets(WalletSource {
                            source_type: CheckoutSourceTypes::Token,
                            token,
                            billing_address: billing_details,
                        }),
                        PaymentMethodToken::ApplePayDecrypt(_) => Err(
                            unimplemented_payment_method!("Apple Pay", "Simplified", "Checkout"),
                        )?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Checkout"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(google_pay_decrypted_data) => {
                            let token = google_pay_decrypted_data
                                .application_primary_account_number
                                .clone();

                            let expiry_month = google_pay_decrypted_data
                                .get_expiry_month()
                                .change_context(errors::ConnectorError::InvalidDataFormat {
                                    field_name: "payment_method_data.card.card_exp_month",
                                })?;

                            let expiry_year = google_pay_decrypted_data
                                .get_four_digit_expiry_year()
                                .change_context(errors::ConnectorError::InvalidDataFormat {
                                    field_name: "payment_method_data.card.card_exp_year",
                                })?;

                            let cryptogram = google_pay_decrypted_data.cryptogram.clone();

                            PaymentSource::GooglePayPredecrypt(Box::new(GooglePayPredecrypt {
                                _type: "network_token".to_string(),
                                token,
                                token_type: "googlepay".to_string(),
                                expiry_month,
                                expiry_year,
                                eci: "06".to_string(),
                                cryptogram,
                                billing_address: billing_details,
                            }))
                        }
                    };
                    Ok((
                        p_source,
                        None,
                        Some(false),
                        payment_type,
                        store_for_future_use,
                    ))
                }
                WalletData::ApplePay(_) => {
                    let payment_method_token = item.router_data.get_payment_method_token()?;
                    match payment_method_token {
                        PaymentMethodToken::Token(apple_pay_payment_token) => {
                            let p_source = PaymentSource::Wallets(WalletSource {
                                source_type: CheckoutSourceTypes::Token,
                                token: apple_pay_payment_token,
                                billing_address: billing_details,
                            });
                            Ok((
                                p_source,
                                None,
                                Some(false),
                                payment_type,
                                store_for_future_use,
                            ))
                        }
                        PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                            let exp_month = decrypt_data.get_expiry_month().change_context(
                                errors::ConnectorError::InvalidDataFormat {
                                    field_name: "expiration_month",
                                },
                            )?;
                            let expiry_year_4_digit = decrypt_data.get_four_digit_expiry_year();
                            let p_source =
                                PaymentSource::ApplePayPredecrypt(Box::new(ApplePayPredecrypt {
                                    token: decrypt_data.application_primary_account_number,
                                    decrypt_type: "network_token".to_string(),
                                    token_type: "applepay".to_string(),
                                    expiry_month: exp_month,
                                    expiry_year: expiry_year_4_digit,
                                    eci: decrypt_data.payment_data.eci_indicator,
                                    cryptogram: decrypt_data.payment_data.online_payment_cryptogram,
                                    billing_address: billing_details,
                                }));
                            Ok((
                                p_source,
                                None,
                                Some(false),
                                payment_type,
                                store_for_future_use,
                            ))
                        }
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Checkout"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Checkout"))?
                        }
                    }
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("checkout"),
                )),
            },
            PaymentMethodData::MandatePayment => {
                let mandate_source = PaymentSource::MandatePayment(MandateSource {
                    source_type: CheckoutSourceTypes::SourceId,
                    source_id: item.router_data.request.connector_mandate_id(),
                    billing_address: billing_details,
                });
                let previous_id = Some(
                    item.router_data
                        .request
                        .get_connector_mandate_request_reference_id()?,
                );
                let p_type = match item.router_data.request.mit_category {
                    Some(MitCategory::Installment) => CheckoutPaymentType::Installment,
                    Some(MitCategory::Recurring) => CheckoutPaymentType::Recurring,
                    Some(MitCategory::Unscheduled) | None => CheckoutPaymentType::Unscheduled,
                    _ => CheckoutPaymentType::Unscheduled,
                };
                Ok((mandate_source, previous_id, Some(true), p_type, None))
            }
            PaymentMethodData::CardDetailsForNetworkTransactionId(ccard) => {
                let (first_name, last_name) = split_account_holder_name(ccard.card_holder_name);
                let payment_source = PaymentSource::Card(CardSource {
                    source_type: CheckoutSourceTypes::Card,
                    number: ccard.card_number.clone(),
                    expiry_month: ccard.card_exp_month.clone(),
                    expiry_year: ccard.card_exp_year.clone(),
                    cvv: None,
                    billing_address: billing_details,
                    account_holder: Some(CheckoutAccountHolderDetails {
                        first_name,
                        last_name,
                    }),
                });

                let previous_id = Some(
                    item.router_data
                        .request
                        .get_optional_network_transaction_id()
                        .ok_or_else(utils::missing_field_err("network_transaction_id"))
                        .attach_printable("Checkout unable to find NTID for MIT")?,
                );

                let p_type = match item.router_data.request.mit_category {
                    Some(MitCategory::Installment) => CheckoutPaymentType::Installment,
                    Some(MitCategory::Recurring) => CheckoutPaymentType::Recurring,
                    Some(MitCategory::Unscheduled) | None => CheckoutPaymentType::Unscheduled,
                    _ => CheckoutPaymentType::Unscheduled,
                };
                Ok((payment_source, previous_id, Some(true), p_type, None))
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("checkout"),
            )),
        }?;

        let authentication_data = item.router_data.request.authentication_data.as_ref();

        let three_ds = match item.router_data.auth_type {
            enums::AuthenticationType::ThreeDs => CheckoutThreeDS {
                enabled: true,
                force_3ds: true,
                eci: authentication_data.and_then(|auth| auth.eci.clone()),
                cryptogram: authentication_data.map(|auth| auth.cavv.clone()),
                xid: authentication_data
                    .and_then(|auth| auth.threeds_server_transaction_id.clone()),
                version: authentication_data.and_then(|auth| {
                    auth.message_version
                        .clone()
                        .map(|version| version.to_string())
                }),
                challenge_indicator,
            },
            enums::AuthenticationType::NoThreeDs => CheckoutThreeDS {
                enabled: false,
                force_3ds: false,
                eci: None,
                cryptogram: None,
                xid: None,
                version: None,
                challenge_indicator: CheckoutChallengeIndicator::NoPreference,
            },
        };

        let return_url = ReturnUrl {
            success_url: item
                .router_data
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=success")),
            failure_url: item
                .router_data
                .request
                .router_return_url
                .as_ref()
                .map(|return_url| format!("{return_url}?status=failure")),
        };

        let connector_auth = &item.router_data.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        let metadata = build_metadata(item);
        let (customer, processing, shipping, items) = if let Some(l2l3_data) =
            &item.router_data.l2_l3_data
        {
            (
                l2l3_data.customer_info.as_ref().map(|_| CheckoutCustomer {
                    name: l2l3_data.get_customer_name(),
                    email: l2l3_data.get_customer_email(),
                    phone: Some(CheckoutPhoneDetails {
                        country_code: l2l3_data.get_customer_phone_country_code(),
                        number: l2l3_data.get_customer_phone_number(),
                    }),
                    tax_number: l2l3_data.get_customer_tax_registration_id(),
                }),
                l2l3_data.order_info.as_ref().map(|_| CheckoutProcessing {
                    order_id: l2l3_data.get_merchant_order_reference_id(),
                    tax_amount: l2l3_data.get_order_tax_amount(),
                    discount_amount: l2l3_data.get_discount_amount(),
                    duty_amount: l2l3_data.get_duty_amount(),
                    shipping_amount: l2l3_data.get_shipping_cost(),
                    shipping_tax_amount: l2l3_data.get_shipping_amount_tax(),
                }),
                Some(CheckoutShipping {
                    address: Some(CheckoutAddress {
                        country: l2l3_data.get_shipping_country(),
                        address_line1: l2l3_data.get_shipping_address_line1(),
                        address_line2: l2l3_data.get_shipping_address_line2(),
                        city: l2l3_data.get_shipping_city(),
                        state: l2l3_data.get_shipping_state(),
                        zip: l2l3_data.get_shipping_zip(),
                    }),
                    from_address_zip: l2l3_data.get_shipping_origin_zip().map(|zip| zip.expose()),
                }),
                l2l3_data.get_order_details().map(|details| {
                    details
                        .iter()
                        .map(|item| CheckoutLineItem {
                            commodity_code: item.commodity_code.clone(),
                            discount_amount: item.unit_discount_amount,
                            name: Some(item.product_name.clone()),
                            quantity: Some(item.quantity),
                            reference: item.product_id.clone(),
                            tax_exempt: None,
                            tax_amount: item.total_tax_amount,
                            total_amount: item.total_amount,
                            unit_of_measure: item.unit_of_measure.clone(),
                            unit_price: Some(item.amount),
                        })
                        .collect()
                }),
            )
        } else {
            (None, None, None, None)
        };

        let partial_authorization = item.router_data.request.enable_partial_authorization.map(
            |enable_partial_authorization| CheckoutPartialAuthorization {
                enabled: *enable_partial_authorization,
            },
        );

        let payment_ip = item.router_data.request.get_ip_address_as_optional();

        let billing_descriptor =
            item.router_data
                .request
                .billing_descriptor
                .as_ref()
                .map(|descriptor| CheckoutBillingDescriptor {
                    name: descriptor.name.clone(),
                    city: descriptor.city.clone(),
                    reference: descriptor.reference.clone(),
                });

        let request = Self {
            source: source_var,
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency.to_string(),
            processing_channel_id,
            three_ds,
            return_url,
            capture,
            reference: item.router_data.connector_request_reference_id.clone(),
            metadata,
            payment_type,
            merchant_initiated,
            previous_payment_id,
            store_for_future_use,
            customer,
            processing,
            shipping,
            items,
            partial_authorization,
            payment_ip,
            billing_descriptor,
        };

        Ok(request)
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CheckoutPaymentStatus {
    Authorized,
    #[default]
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
    Captured,
    #[serde(rename = "Retry Scheduled")]
    RetryScheduled,
    Voided,
    #[serde(rename = "Partially Captured")]
    PartiallyCaptured,
    #[serde(rename = "Partially Refunded")]
    PartiallyRefunded,
    Refunded,
    Canceled,
    Expired,
}

impl TryFrom<CheckoutWebhookEventType> for CheckoutPaymentStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: CheckoutWebhookEventType) -> Result<Self, Self::Error> {
        match value {
            CheckoutWebhookEventType::PaymentApproved => Ok(Self::Authorized),
            CheckoutWebhookEventType::PaymentCaptured => Ok(Self::Captured),
            CheckoutWebhookEventType::PaymentDeclined => Ok(Self::Declined),
            CheckoutWebhookEventType::AuthenticationStarted
            | CheckoutWebhookEventType::AuthenticationApproved
            | CheckoutWebhookEventType::AuthenticationAttempted => Ok(Self::Pending),
            CheckoutWebhookEventType::AuthenticationExpired
            | CheckoutWebhookEventType::AuthenticationFailed
            | CheckoutWebhookEventType::PaymentAuthenticationFailed
            | CheckoutWebhookEventType::PaymentCaptureDeclined => Ok(Self::Declined),
            CheckoutWebhookEventType::PaymentCanceled => Ok(Self::Canceled),
            CheckoutWebhookEventType::PaymentVoided => Ok(Self::Voided),
            CheckoutWebhookEventType::PaymentRefunded
            | CheckoutWebhookEventType::PaymentRefundDeclined
            | CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeExpired
            | CheckoutWebhookEventType::DisputeAccepted
            | CheckoutWebhookEventType::DisputeCanceled
            | CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutWebhookEventType::DisputeEvidenceRequired
            | CheckoutWebhookEventType::DisputeArbitrationLost
            | CheckoutWebhookEventType::DisputeArbitrationWon
            | CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeLost
            | CheckoutWebhookEventType::Unknown => {
                Err(errors::ConnectorError::WebhookEventTypeNotFound.into())
            }
        }
    }
}

fn get_attempt_status_cap(
    item: (CheckoutPaymentStatus, Option<enums::CaptureMethod>),
) -> AttemptStatus {
    let (status, capture_method) = item;
    match status {
        CheckoutPaymentStatus::Authorized => {
            if capture_method == Some(enums::CaptureMethod::Automatic) || capture_method.is_none() {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Authorized
            }
        }
        CheckoutPaymentStatus::Captured
        | CheckoutPaymentStatus::PartiallyRefunded
        | CheckoutPaymentStatus::Refunded
        | CheckoutPaymentStatus::CardVerified => AttemptStatus::Charged,
        CheckoutPaymentStatus::PartiallyCaptured => AttemptStatus::PartialCharged,
        CheckoutPaymentStatus::Declined
        | CheckoutPaymentStatus::Expired
        | CheckoutPaymentStatus::Canceled => AttemptStatus::Failure,
        CheckoutPaymentStatus::Pending => AttemptStatus::AuthenticationPending,
        CheckoutPaymentStatus::RetryScheduled => AttemptStatus::Pending,
        CheckoutPaymentStatus::Voided => AttemptStatus::Voided,
    }
}

fn get_attempt_status_intent(
    item: (CheckoutPaymentStatus, CheckoutPaymentIntent),
) -> AttemptStatus {
    let (status, psync_flow) = item;

    match status {
        CheckoutPaymentStatus::Authorized => {
            if psync_flow == CheckoutPaymentIntent::Capture {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Authorized
            }
        }
        CheckoutPaymentStatus::Captured
        | CheckoutPaymentStatus::PartiallyRefunded
        | CheckoutPaymentStatus::Refunded
        | CheckoutPaymentStatus::CardVerified => AttemptStatus::Charged,
        CheckoutPaymentStatus::PartiallyCaptured => AttemptStatus::PartialCharged,
        CheckoutPaymentStatus::Declined
        | CheckoutPaymentStatus::Expired
        | CheckoutPaymentStatus::Canceled => AttemptStatus::Failure,
        CheckoutPaymentStatus::Pending => AttemptStatus::AuthenticationPending,
        CheckoutPaymentStatus::RetryScheduled => AttemptStatus::Pending,
        CheckoutPaymentStatus::Voided => AttemptStatus::Voided,
    }
}

fn get_attempt_status_bal(item: (CheckoutPaymentStatus, Option<Balances>)) -> AttemptStatus {
    let (status, balances) = item;

    match status {
        CheckoutPaymentStatus::Authorized => {
            if let Some(Balances {
                available_to_capture: 0,
            }) = balances
            {
                AttemptStatus::Charged
            } else {
                AttemptStatus::Authorized
            }
        }
        CheckoutPaymentStatus::Captured
        | CheckoutPaymentStatus::PartiallyRefunded
        | CheckoutPaymentStatus::Refunded => AttemptStatus::Charged,
        CheckoutPaymentStatus::PartiallyCaptured => AttemptStatus::PartialCharged,
        CheckoutPaymentStatus::Declined
        | CheckoutPaymentStatus::Expired
        | CheckoutPaymentStatus::Canceled => AttemptStatus::Failure,
        CheckoutPaymentStatus::Pending => AttemptStatus::AuthenticationPending,
        CheckoutPaymentStatus::CardVerified | CheckoutPaymentStatus::RetryScheduled => {
            AttemptStatus::Pending
        }
        CheckoutPaymentStatus::Voided => AttemptStatus::Voided,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Href {
    #[serde(rename = "href")]
    redirection_url: Url,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Links {
    redirect: Option<Href>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Source {
    id: Option<String>,
    avs_check: Option<String>,
    cvv_check: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentsResponse {
    id: String,
    amount: Option<MinorUnit>,
    currency: Option<String>,
    scheme_id: Option<String>,
    processing: Option<PaymentProcessingDetails>,
    action_id: Option<String>,
    status: CheckoutPaymentStatus,
    #[serde(rename = "_links")]
    links: Links,
    balances: Option<Balances>,
    reference: Option<String>,
    response_code: Option<String>,
    response_summary: Option<String>,
    approved: Option<bool>,
    processed_on: Option<String>,
    source: Option<Source>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentProcessingDetails {
    /// The Merchant Advice Code (MAC) provided by Mastercard, which contains additional information about the transaction.
    pub partner_merchant_advice_code: Option<String>,
    /// The original authorization response code sent by the scheme.
    pub partner_response_code: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PaymentsResponseEnum {
    ActionResponse(Vec<ActionResponse>),
    PaymentResponse(Box<PaymentsResponse>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct Balances {
    available_to_capture: i32,
}

fn get_connector_meta(
    capture_method: enums::CaptureMethod,
) -> CustomResult<serde_json::Value, errors::ConnectorError> {
    match capture_method {
        enums::CaptureMethod::Automatic | enums::CaptureMethod::SequentialAutomatic => {
            Ok(serde_json::json!(CheckoutMeta {
                psync_flow: CheckoutPaymentIntent::Capture,
            }))
        }
        enums::CaptureMethod::Manual | enums::CaptureMethod::ManualMultiple => {
            Ok(serde_json::json!(CheckoutMeta {
                psync_flow: CheckoutPaymentIntent::Authorize,
            }))
        }
        enums::CaptureMethod::Scheduled => {
            Err(errors::ConnectorError::CaptureMethodNotSupported.into())
        }
    }
}

impl TryFrom<PaymentsResponseRouterData<PaymentsResponse>> for PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PaymentsResponseRouterData<PaymentsResponse>) -> Result<Self, Self::Error> {
        let status =
            get_attempt_status_cap((item.response.status, item.data.request.capture_method));

        if status == AttemptStatus::Failure {
            let error_response = ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            };

            return Ok(Self {
                status,
                response: Err(error_response),
                ..item.data
            });
        }

        let connector_meta =
            get_connector_meta(item.data.request.capture_method.unwrap_or_default())?;

        let redirection_data = item
            .response
            .links
            .redirect
            .map(|href| RedirectForm::from((href.redirection_url, Method::Get)));

        let mandate_reference = if item.data.request.is_mandate_payment() {
            item.response
                .source
                .as_ref()
                .and_then(|src| src.id.clone())
                .map(|id| MandateReference {
                    connector_mandate_id: Some(id),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: Some(item.response.id.clone()),
                })
        } else {
            None
        };

        let additional_information =
            convert_to_additional_payment_method_connector_response(item.response.source.as_ref())
                .map(ConnectorResponseData::with_additional_payment_method_data);

        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data: Box::new(redirection_data),
            mandate_reference: Box::new(mandate_reference),
            connector_metadata: Some(connector_meta),
            network_txn_id: item.response.scheme_id.clone(),
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
            incremental_authorization_allowed: None,
            charges: None,
        };

        let (amount_captured, minor_amount_capturable) = match item.data.request.capture_method {
            Some(enums::CaptureMethod::Manual) | Some(enums::CaptureMethod::ManualMultiple) => {
                (None, item.response.amount)
            }
            _ => (item.response.amount.map(MinorUnit::get_amount_as_i64), None),
        };

        let authorized_amount = item
            .data
            .request
            .enable_partial_authorization
            .filter(|flag| flag.is_true())
            .and(item.response.amount);

        Ok(Self {
            status,
            response: Ok(payments_response_data),
            connector_response: additional_information,
            authorized_amount,
            amount_captured,
            minor_amount_capturable,
            ..item.data
        })
    }
}

impl
    TryFrom<
        ResponseRouterData<
            SetupMandate,
            PaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            SetupMandate,
            PaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let connector_meta =
            get_connector_meta(item.data.request.capture_method.unwrap_or_default())?;
        let redirection_data = item
            .response
            .links
            .redirect
            .map(|href| RedirectForm::from((href.redirection_url, Method::Get)));
        let status =
            get_attempt_status_cap((item.response.status, item.data.request.capture_method));
        let network_advice_code = item
            .response
            .processing
            .as_ref()
            .and_then(|processing| {
                processing
                    .partner_merchant_advice_code
                    .as_ref()
                    .or(processing.partner_response_code.as_ref())
            })
            .cloned();
        let error_response = if status == AttemptStatus::Failure {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                network_advice_code,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            None
        };

        let mandate_reference = item
            .response
            .source
            .as_ref()
            .and_then(|src| src.id.clone())
            .map(|id| MandateReference {
                connector_mandate_id: Some(id),
                payment_method_id: None,
                mandate_metadata: None,
                connector_mandate_request_reference_id: Some(item.response.id.clone()),
            });

        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data: Box::new(redirection_data),
            mandate_reference: Box::new(mandate_reference),
            connector_metadata: Some(connector_meta),
            network_txn_id: item.response.scheme_id.clone(),
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<PaymentsResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<PaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item
            .response
            .links
            .redirect
            .map(|href| RedirectForm::from((href.redirection_url, Method::Get)));
        let checkout_meta: CheckoutMeta =
            utils::to_connector_meta(item.data.request.connector_meta.clone())?;
        let status = get_attempt_status_intent((item.response.status, checkout_meta.psync_flow));
        let error_response = if status == AttemptStatus::Failure {
            Some(ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
                attempt_status: None,
                connector_transaction_id: Some(item.response.id.clone()),
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            None
        };

        let mandate_reference = if item.data.request.is_mandate_payment() {
            item.response
                .source
                .as_ref()
                .and_then(|src| src.id.clone())
                .map(|id| MandateReference {
                    connector_mandate_id: Some(id),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: Some(item.response.id.clone()),
                })
        } else {
            None
        };

        let additional_information =
            convert_to_additional_payment_method_connector_response(item.response.source.as_ref())
                .map(ConnectorResponseData::with_additional_payment_method_data);

        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data: Box::new(redirection_data),
            mandate_reference: Box::new(mandate_reference),
            connector_metadata: None,
            network_txn_id: item.response.scheme_id.clone(),
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            connector_response: additional_information,
            ..item.data
        })
    }
}

impl TryFrom<PaymentsSyncResponseRouterData<PaymentsResponseEnum>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: PaymentsSyncResponseRouterData<PaymentsResponseEnum>,
    ) -> Result<Self, Self::Error> {
        let capture_sync_response_list = match item.response {
            PaymentsResponseEnum::PaymentResponse(payments_response) => {
                // for webhook consumption flow
                utils::construct_captures_response_hashmap(vec![payments_response])?
            }
            PaymentsResponseEnum::ActionResponse(action_list) => {
                // for captures sync
                utils::construct_captures_response_hashmap(action_list)?
            }
        };
        Ok(Self {
            response: Ok(PaymentsResponseData::MultipleCaptureResponse {
                capture_sync_response_list,
            }),
            ..item.data
        })
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize)]
pub struct PaymentVoidRequest {
    reference: String,
}
#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentVoidResponse {
    #[serde(skip)]
    pub(super) status: u16,
    action_id: String,
    reference: String,
    scheme_id: Option<String>,
}

impl From<&PaymentVoidResponse> for AttemptStatus {
    fn from(item: &PaymentVoidResponse) -> Self {
        if item.status == 202 {
            Self::Voided
        } else {
            Self::VoidFailed
        }
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<PaymentVoidResponse>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<PaymentVoidResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;
        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(response.action_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: item.response.scheme_id.clone(),
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status: response.into(),
            ..item.data
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for PaymentVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            reference: item.request.connector_transaction_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub enum CaptureType {
    Final,
    NonFinal,
}

#[derive(Debug, Serialize)]
pub struct PaymentCaptureRequest {
    pub amount: Option<MinorUnit>,
    pub capture_type: Option<CaptureType>,
    pub processing_channel_id: Secret<String>,
    pub reference: Option<String>,
}

impl TryFrom<&CheckoutRouterData<&PaymentsCaptureRouterData>> for PaymentCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CheckoutRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let connector_auth = &item.router_data.connector_auth_type;
        let auth_type: CheckoutAuthType = connector_auth.try_into()?;
        let processing_channel_id = auth_type.processing_channel_id;
        let capture_type = if item.router_data.request.is_multiple_capture() {
            CaptureType::NonFinal
        } else {
            CaptureType::Final
        };
        let reference = item
            .router_data
            .request
            .multiple_capture_data
            .as_ref()
            .map(|multiple_capture_data| multiple_capture_data.capture_reference.clone());
        Ok(Self {
            amount: Some(item.amount.to_owned()),
            capture_type: Some(capture_type),
            processing_channel_id,
            reference, // hyperswitch's reference for this capture
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaymentCaptureResponse {
    pub action_id: String,
    pub reference: Option<String>,
    pub scheme_id: Option<String>,
}

impl TryFrom<PaymentsCaptureResponseRouterData<PaymentCaptureResponse>>
    for PaymentsCaptureRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<PaymentCaptureResponse>,
    ) -> Result<Self, Self::Error> {
        let connector_meta = serde_json::json!(CheckoutMeta {
            psync_flow: CheckoutPaymentIntent::Capture,
        });
        let (status, amount_captured) = if item.http_code == 202 {
            (
                AttemptStatus::Charged,
                Some(item.data.request.amount_to_capture),
            )
        } else {
            (AttemptStatus::Pending, None)
        };

        // if multiple capture request, return capture action_id so that it will be updated in the captures table.
        // else return previous connector_transaction_id.
        let resource_id = if item.data.request.is_multiple_capture() {
            item.response.action_id
        } else {
            item.data.request.connector_transaction_id.to_owned()
        };

        Ok(Self {
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(resource_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: Some(connector_meta),
                network_txn_id: item.response.scheme_id.clone(),
                connector_response_reference_id: item.response.reference,
                incremental_authorization_allowed: None,
                charges: None,
            }),
            status,
            amount_captured,
            ..item.data
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundRequest {
    amount: Option<MinorUnit>,
    reference: String,
}

impl<F> TryFrom<&CheckoutRouterData<&RefundsRouterData<F>>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CheckoutRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let reference = item.router_data.request.refund_id.clone();
        Ok(Self {
            amount: Some(item.amount.to_owned()),
            reference,
        })
    }
}
#[allow(dead_code)]
#[derive(Deserialize, Debug, Serialize)]
pub struct RefundResponse {
    action_id: String,
    reference: String,
}

#[derive(Deserialize)]
pub struct CheckoutRefundResponse {
    pub(super) status: u16,
    pub(super) response: RefundResponse,
}

impl From<&CheckoutRefundResponse> for enums::RefundStatus {
    fn from(item: &CheckoutRefundResponse) -> Self {
        if item.status == 202 {
            Self::Success
        } else {
            Self::Failure
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, CheckoutRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, CheckoutRefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CheckoutRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(&item.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct CheckoutErrorResponse {
    pub request_id: Option<String>,
    pub error_type: Option<String>,
    pub error_codes: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, PartialEq, Serialize)]
pub enum ActionType {
    Authorization,
    Void,
    Capture,
    Refund,
    Payout,
    Return,
    #[serde(rename = "Card Verification")]
    CardVerification,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ActionResponse {
    #[serde(rename = "id")]
    pub action_id: String,
    pub amount: MinorUnit,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub approved: Option<bool>,
    pub reference: Option<String>,
}

impl From<&ActionResponse> for enums::RefundStatus {
    fn from(item: &ActionResponse) -> Self {
        match item.approved {
            Some(true) => Self::Success,
            Some(false) => Self::Failure,
            None => Self::Pending,
        }
    }
}

impl utils::MultipleCaptureSyncResponse for ActionResponse {
    fn get_connector_capture_id(&self) -> String {
        self.action_id.clone()
    }

    fn get_capture_attempt_status(&self) -> AttemptStatus {
        match self.approved {
            Some(true) => AttemptStatus::Charged,
            Some(false) => AttemptStatus::Failure,
            None => AttemptStatus::Pending,
        }
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }

    fn is_capture_response(&self) -> bool {
        self.action_type == ActionType::Capture
    }

    fn get_amount_captured(&self) -> Result<Option<MinorUnit>, error_stack::Report<ParsingError>> {
        Ok(Some(self.amount))
    }
}

impl utils::MultipleCaptureSyncResponse for Box<PaymentsResponse> {
    fn get_connector_capture_id(&self) -> String {
        self.action_id.clone().unwrap_or("".into())
    }

    fn get_capture_attempt_status(&self) -> AttemptStatus {
        get_attempt_status_bal((self.status.clone(), self.balances.clone()))
    }

    fn get_connector_reference_id(&self) -> Option<String> {
        self.reference.clone()
    }

    fn is_capture_response(&self) -> bool {
        self.status == CheckoutPaymentStatus::Captured
    }
    fn get_amount_captured(&self) -> Result<Option<MinorUnit>, error_stack::Report<ParsingError>> {
        Ok(self.amount)
    }
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckoutRedirectResponseStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, serde::Deserialize, Eq, PartialEq)]
pub struct CheckoutRedirectResponse {
    pub status: Option<CheckoutRedirectResponseStatus>,
    #[serde(rename = "cko-session-id")]
    pub cko_session_id: Option<String>,
}

impl TryFrom<RefundsResponseRouterData<Execute, &ActionResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, &ActionResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, &ActionResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response);
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.action_id.clone(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl From<CheckoutRedirectResponseStatus> for AttemptStatus {
    fn from(item: CheckoutRedirectResponseStatus) -> Self {
        match item {
            CheckoutRedirectResponseStatus::Success => Self::AuthenticationSuccessful,
            CheckoutRedirectResponseStatus::Failure => Self::Failure,
        }
    }
}

pub fn is_refund_event(event_code: &CheckoutWebhookEventType) -> bool {
    matches!(
        event_code,
        CheckoutWebhookEventType::PaymentRefunded | CheckoutWebhookEventType::PaymentRefundDeclined
    )
}

pub fn is_chargeback_event(event_code: &CheckoutWebhookEventType) -> bool {
    matches!(
        event_code,
        CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeExpired
            | CheckoutWebhookEventType::DisputeAccepted
            | CheckoutWebhookEventType::DisputeCanceled
            | CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutWebhookEventType::DisputeEvidenceRequired
            | CheckoutWebhookEventType::DisputeArbitrationLost
            | CheckoutWebhookEventType::DisputeArbitrationWon
            | CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeLost
    )
}

#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutWebhookEventType {
    AuthenticationStarted,
    AuthenticationApproved,
    AuthenticationAttempted,
    AuthenticationExpired,
    AuthenticationFailed,
    PaymentApproved,
    PaymentCaptured,
    PaymentDeclined,
    PaymentRefunded,
    PaymentRefundDeclined,
    PaymentAuthenticationFailed,
    PaymentCanceled,
    PaymentCaptureDeclined,
    PaymentVoided,
    DisputeReceived,
    DisputeExpired,
    DisputeAccepted,
    DisputeCanceled,
    DisputeEvidenceSubmitted,
    DisputeEvidenceAcknowledgedByScheme,
    DisputeEvidenceRequired,
    DisputeArbitrationLost,
    DisputeArbitrationWon,
    DisputeWon,
    DisputeLost,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookEventTypeBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutWebhookEventType,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookData {
    pub id: String,
    pub payment_id: Option<String>,
    pub action_id: Option<String>,
    pub reference: Option<String>,
    pub amount: MinorUnit,
    pub balances: Option<Balances>,
    pub response_code: Option<String>,
    pub response_summary: Option<String>,
    pub currency: String,
    pub processed_on: Option<String>,
    pub approved: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutWebhookEventType,
    pub data: CheckoutWebhookData,
    #[serde(rename = "_links")]
    pub links: Links,
    pub source: Option<Source>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckoutDisputeWebhookData {
    pub id: String,
    pub payment_id: Option<String>,
    pub action_id: Option<String>,
    pub amount: MinorUnit,
    pub currency: enums::Currency,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub evidence_required_by: Option<PrimitiveDateTime>,
    pub reason_code: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub date: Option<PrimitiveDateTime>,
}
#[derive(Debug, Deserialize)]
pub struct CheckoutDisputeWebhookBody {
    #[serde(rename = "type")]
    pub transaction_type: CheckoutDisputeTransactionType,
    pub data: CheckoutDisputeWebhookData,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_on: Option<PrimitiveDateTime>,
}
#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutDisputeTransactionType {
    DisputeReceived,
    DisputeExpired,
    DisputeAccepted,
    DisputeCanceled,
    DisputeEvidenceSubmitted,
    DisputeEvidenceAcknowledgedByScheme,
    DisputeEvidenceRequired,
    DisputeArbitrationLost,
    DisputeArbitrationWon,
    DisputeWon,
    DisputeLost,
}

impl From<CheckoutWebhookEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(transaction_type: CheckoutWebhookEventType) -> Self {
        match transaction_type {
            CheckoutWebhookEventType::AuthenticationStarted
            | CheckoutWebhookEventType::AuthenticationApproved
            | CheckoutWebhookEventType::AuthenticationAttempted => Self::EventNotSupported,
            CheckoutWebhookEventType::AuthenticationExpired
            | CheckoutWebhookEventType::AuthenticationFailed
            | CheckoutWebhookEventType::PaymentAuthenticationFailed => {
                Self::PaymentIntentAuthorizationFailure
            }
            CheckoutWebhookEventType::PaymentApproved => Self::EventNotSupported,
            CheckoutWebhookEventType::PaymentCaptured => Self::PaymentIntentSuccess,
            CheckoutWebhookEventType::PaymentDeclined => Self::PaymentIntentFailure,
            CheckoutWebhookEventType::PaymentRefunded => Self::RefundSuccess,
            CheckoutWebhookEventType::PaymentRefundDeclined => Self::RefundFailure,
            CheckoutWebhookEventType::PaymentCanceled => Self::PaymentIntentCancelFailure,
            CheckoutWebhookEventType::PaymentCaptureDeclined => Self::PaymentIntentCaptureFailure,
            CheckoutWebhookEventType::PaymentVoided => Self::PaymentIntentCancelled,
            CheckoutWebhookEventType::DisputeReceived
            | CheckoutWebhookEventType::DisputeEvidenceRequired => Self::DisputeOpened,
            CheckoutWebhookEventType::DisputeExpired => Self::DisputeExpired,
            CheckoutWebhookEventType::DisputeAccepted => Self::DisputeAccepted,
            CheckoutWebhookEventType::DisputeCanceled => Self::DisputeCancelled,
            CheckoutWebhookEventType::DisputeEvidenceSubmitted
            | CheckoutWebhookEventType::DisputeEvidenceAcknowledgedByScheme => {
                Self::DisputeChallenged
            }
            CheckoutWebhookEventType::DisputeWon
            | CheckoutWebhookEventType::DisputeArbitrationWon => Self::DisputeWon,
            CheckoutWebhookEventType::DisputeLost
            | CheckoutWebhookEventType::DisputeArbitrationLost => Self::DisputeLost,
            CheckoutWebhookEventType::Unknown => Self::EventNotSupported,
        }
    }
}

impl From<CheckoutDisputeTransactionType> for api_models::enums::DisputeStage {
    fn from(code: CheckoutDisputeTransactionType) -> Self {
        match code {
            CheckoutDisputeTransactionType::DisputeArbitrationLost
            | CheckoutDisputeTransactionType::DisputeArbitrationWon => Self::PreArbitration,
            CheckoutDisputeTransactionType::DisputeReceived
            | CheckoutDisputeTransactionType::DisputeExpired
            | CheckoutDisputeTransactionType::DisputeAccepted
            | CheckoutDisputeTransactionType::DisputeCanceled
            | CheckoutDisputeTransactionType::DisputeEvidenceSubmitted
            | CheckoutDisputeTransactionType::DisputeEvidenceAcknowledgedByScheme
            | CheckoutDisputeTransactionType::DisputeEvidenceRequired
            | CheckoutDisputeTransactionType::DisputeWon
            | CheckoutDisputeTransactionType::DisputeLost => Self::Dispute,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckoutWebhookObjectResource {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CheckoutFileRequest {
    pub purpose: &'static str,
    #[serde(skip)]
    pub file: Vec<u8>,
    #[serde(skip)]
    pub file_key: String,
    #[serde(skip)]
    pub file_type: String,
}

pub fn construct_file_upload_request(
    file_upload_router_data: UploadFileRouterData,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let request = file_upload_router_data.request;
    let checkout_file_request = CheckoutFileRequest {
        purpose: "dispute_evidence",
        file: request.file.clone(),
        file_key: request.file_key.clone(),
        file_type: request.file_type.to_string(),
    };
    let mut multipart = reqwest::multipart::Form::new();
    multipart = multipart.text("purpose", "dispute_evidence");
    let file_data = reqwest::multipart::Part::bytes(request.file)
        .file_name(format!(
            "{}.{}",
            request.file_key,
            request
                .file_type
                .as_ref()
                .split('/')
                .next_back()
                .unwrap_or_default()
        ))
        .mime_str(request.file_type.as_ref())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("Failure in constructing file data")?;
    multipart = multipart.part("file", file_data);
    Ok(RequestContent::FormData((
        multipart,
        Box::new(checkout_file_request),
    )))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileUploadResponse {
    #[serde(rename = "id")]
    pub file_id: String,
}

#[derive(Default, Debug, Serialize)]
pub struct Evidence {
    pub proof_of_delivery_or_service_file: Option<String>,
    pub invoice_or_receipt_file: Option<String>,
    pub invoice_showing_distinct_transactions_file: Option<String>,
    pub customer_communication_file: Option<String>,
    pub refund_or_cancellation_policy_file: Option<String>,
    pub recurring_transaction_agreement_file: Option<String>,
    pub additional_evidence_file: Option<String>,
}

impl TryFrom<&webhooks::IncomingWebhookRequestDetails<'_>> for PaymentsResponse {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> Result<Self, Self::Error> {
        let details: CheckoutWebhookBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let data = details.data;
        let psync_struct = Self {
            id: data.payment_id.unwrap_or(data.id),
            amount: Some(data.amount),
            status: CheckoutPaymentStatus::try_from(details.transaction_type)?,
            links: details.links,
            balances: data.balances,
            reference: data.reference,
            response_code: data.response_code,
            response_summary: data.response_summary,
            action_id: data.action_id,
            currency: Some(data.currency),
            processed_on: data.processed_on,
            approved: data.approved,
            source: Some(Source {
                id: details.source.clone().and_then(|src| src.id),
                avs_check: details.source.clone().and_then(|src| src.avs_check),
                cvv_check: details.source.clone().and_then(|src| src.cvv_check),
            }),
            scheme_id: None,
            processing: None,
        };

        Ok(psync_struct)
    }
}

impl TryFrom<&webhooks::IncomingWebhookRequestDetails<'_>> for RefundResponse {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> Result<Self, Self::Error> {
        let details: CheckoutWebhookBody = request
            .body
            .parse_struct("CheckoutWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let data = details.data;
        let refund_struct = Self {
            action_id: data
                .action_id
                .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?,
            reference: data
                .reference
                .ok_or(errors::ConnectorError::WebhookBodyDecodingFailed)?,
        };

        Ok(refund_struct)
    }
}

impl TryFrom<&SubmitEvidenceRouterData> for Evidence {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SubmitEvidenceRouterData) -> Result<Self, Self::Error> {
        let submit_evidence_request_data = item.request.clone();
        Ok(Self {
            proof_of_delivery_or_service_file: submit_evidence_request_data
                .shipping_documentation_provider_file_id,
            invoice_or_receipt_file: submit_evidence_request_data.receipt_provider_file_id,
            invoice_showing_distinct_transactions_file: submit_evidence_request_data
                .invoice_showing_distinct_transactions_provider_file_id,
            customer_communication_file: submit_evidence_request_data
                .customer_communication_provider_file_id,
            refund_or_cancellation_policy_file: submit_evidence_request_data
                .refund_policy_provider_file_id,
            recurring_transaction_agreement_file: submit_evidence_request_data
                .recurring_transaction_agreement_provider_file_id,
            additional_evidence_file: submit_evidence_request_data
                .uncategorized_file_provider_file_id,
        })
    }
}

impl From<String> for utils::ErrorCodeAndMessage {
    fn from(error: String) -> Self {
        Self {
            error_code: error.clone(),
            error_message: error,
        }
    }
}

fn convert_to_additional_payment_method_connector_response(
    source: Option<&Source>,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    source.map(|code| {
        let payment_checks = serde_json::json!({
            "avs_result": code.avs_check,
            "card_validation_result": code.cvv_check,
        });
        AdditionalPaymentMethodConnectorResponse::Card {
            authentication_data: None,
            payment_checks: Some(payment_checks),
            card_network: None,
            domestic_network: None,
        }
    })
}
