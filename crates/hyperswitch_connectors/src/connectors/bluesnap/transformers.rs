use std::collections::HashMap;

use api_models::{
    payments::{
        AmountInfo, ApplePayPaymentRequest, ApplePaySessionResponse,
        ApplepayCombinedSessionTokenData, ApplepaySessionTokenData, ApplepaySessionTokenMetadata,
        ApplepaySessionTokenResponse, NextActionCall, NoThirdPartySdkSessionResponse,
        SdkNextAction, SessionToken,
    },
    webhooks::IncomingWebhookEvent,
};
use base64::Engine;
use common_enums::{enums, CountryAlpha2};
use common_utils::{
    consts::{APPLEPAY_VALIDATION_URL, BASE64_ENGINE},
    errors::CustomResult,
    ext_traits::{ByteSliceExt, Encode, OptionExt, StringExt, ValueExt},
    pii::Email,
    types::StringMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::AddressDetails,
    payment_method_data::{self, PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    types::{PaymentsSessionResponseRouterData, RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, AddressDetailsData, ApplePay, CardData as _, ForeignTryFrom,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData, RouterData as _,
    },
};

const DISPLAY_METADATA: &str = "Y";

#[derive(Debug, Serialize)]
pub struct BluesnapRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(StringMajorUnit, T)> for BluesnapRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from((amount, item): (StringMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsRequest {
    amount: StringMajorUnit,
    #[serde(flatten)]
    payment_method: PaymentMethodDetails,
    currency: enums::Currency,
    card_transaction_type: BluesnapTxnType,
    transaction_fraud_info: Option<TransactionFraudInfo>,
    card_holder_info: Option<BluesnapCardHolderInfo>,
    merchant_transaction_id: Option<String>,
    transaction_meta_data: Option<BluesnapMetadata>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapMetadata {
    meta_data: Vec<RequestMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestMetadata {
    meta_key: Option<String>,
    meta_value: Option<String>,
    is_visible: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCardHolderInfo {
    first_name: Secret<String>,
    last_name: Secret<String>,
    email: Email,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionFraudInfo {
    fraud_session_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCreateWalletToken {
    wallet_type: String,
    validation_url: Secret<String>,
    domain_name: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDSecureInfo {
    three_d_secure_reference_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentMethodDetails {
    CreditCard(Card),
    Wallet(BluesnapWallet),
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    card_number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWallet {
    wallet_type: BluesnapWalletTypes,
    encoded_payment_token: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapGooglePayObject {
    payment_method_data: utils::GooglePayWalletData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapWalletTypes {
    GooglePay,
    ApplePay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPaymentToken {
    billing_contact: BillingDetails,
    token: ApplepayPaymentData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingDetails {
    country_code: Option<CountryAlpha2>,
    address_lines: Option<Vec<Secret<String>>>,
    family_name: Option<Secret<String>>,
    given_name: Option<Secret<String>>,
    postal_code: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayPaymentData {
    payment_data: ApplePayEncodedPaymentData,
    payment_method: ApplepayPaymentMethod,
    transaction_identifier: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayPaymentMethod {
    display_name: String,
    network: String,
    #[serde(rename = "type")]
    pm_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplePayEncodedPaymentData {
    data: String,
    header: Option<ApplepayHeader>,
    signature: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayHeader {
    ephemeral_public_key: Secret<String>,
    public_key_hash: Secret<String>,
    transaction_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BluesnapConnectorMetaData {
    pub merchant_id: common_utils::id_type::MerchantId,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsTokenRequest {
    cc_number: cards::CardNumber,
    exp_date: Secret<String>,
}

impl TryFrom<&BluesnapRouterData<&types::PaymentsAuthorizeRouterData>>
    for BluesnapPaymentsTokenRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluesnapRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ref ccard) => Ok(Self {
                cc_number: ccard.card_number.clone(),
                exp_date: ccard.get_expiry_date_as_mmyyyy("/"),
            }),
            PaymentMethodData::Wallet(_)
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
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected payment method via Token flow through bluesnap".to_string(),
                )
                .into())
            }
        }
    }
}

impl TryFrom<&BluesnapRouterData<&types::PaymentsAuthorizeRouterData>> for BluesnapPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluesnapRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth_mode = match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual) => BluesnapTxnType::AuthOnly,
            _ => BluesnapTxnType::AuthCapture,
        };
        let transaction_meta_data =
            item.router_data
                .request
                .metadata
                .as_ref()
                .map(|metadata| BluesnapMetadata {
                    meta_data: convert_metadata_to_request_metadata(metadata.to_owned()),
                });

        let (payment_method, card_holder_info) = match item
            .router_data
            .request
            .payment_method_data
            .clone()
        {
            PaymentMethodData::Card(ref ccard) => Ok((
                PaymentMethodDetails::CreditCard(Card {
                    card_number: ccard.card_number.clone(),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.get_expiry_year_4_digit(),
                    security_code: ccard.card_cvc.clone(),
                }),
                get_card_holder_info(
                    item.router_data.get_billing_address()?,
                    item.router_data.request.get_email()?,
                )?,
            )),
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(payment_method_data) => {
                    let gpay_object = BluesnapGooglePayObject {
                        payment_method_data: utils::GooglePayWalletData::from(payment_method_data),
                    }
                    .encode_to_string_of_json()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok((
                        PaymentMethodDetails::Wallet(BluesnapWallet {
                            wallet_type: BluesnapWalletTypes::GooglePay,
                            encoded_payment_token: Secret::new(BASE64_ENGINE.encode(gpay_object)),
                        }),
                        None,
                    ))
                }
                WalletData::ApplePay(payment_method_data) => {
                    let apple_pay_payment_data =
                        payment_method_data.get_applepay_decoded_payment_data()?;
                    let apple_pay_payment_data: ApplePayEncodedPaymentData = apple_pay_payment_data
                        .expose()[..]
                        .as_bytes()
                        .parse_struct("ApplePayEncodedPaymentData")
                        .change_context(errors::ConnectorError::InvalidWalletToken {
                            wallet_name: "Apple Pay".to_string(),
                        })?;

                    let billing = item.router_data.get_billing()?.to_owned();

                    let billing_address = billing
                        .address
                        .get_required_value("billing_address")
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "billing",
                        })?;

                    let mut address = Vec::new();
                    if let Some(add) = billing_address.line1.to_owned() {
                        address.push(add)
                    }
                    if let Some(add) = billing_address.line2.to_owned() {
                        address.push(add)
                    }
                    if let Some(add) = billing_address.line3.to_owned() {
                        address.push(add)
                    }

                    let apple_pay_object = EncodedPaymentToken {
                        token: ApplepayPaymentData {
                            payment_data: apple_pay_payment_data,
                            payment_method: payment_method_data.payment_method.to_owned().into(),
                            transaction_identifier: payment_method_data.transaction_identifier,
                        },
                        billing_contact: BillingDetails {
                            country_code: billing_address.country,
                            address_lines: Some(address),
                            family_name: billing_address.last_name.to_owned(),
                            given_name: billing_address.first_name.to_owned(),
                            postal_code: billing_address.zip,
                        },
                    }
                    .encode_to_string_of_json()
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                    Ok((
                        PaymentMethodDetails::Wallet(BluesnapWallet {
                            wallet_type: BluesnapWalletTypes::ApplePay,
                            encoded_payment_token: Secret::new(
                                BASE64_ENGINE.encode(apple_pay_object),
                            ),
                        }),
                        get_card_holder_info(
                            item.router_data.get_billing_address()?,
                            item.router_data.request.get_email()?,
                        )?,
                    ))
                }
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
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
                | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("bluesnap"),
                )),
            },
            PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("bluesnap"),
                ))
            }
        }?;
        Ok(Self {
            amount: item.amount.to_owned(),
            payment_method,
            currency: item.router_data.request.currency,
            card_transaction_type: auth_mode,
            transaction_fraud_info: Some(TransactionFraudInfo {
                fraud_session_id: item.router_data.payment_id.clone(),
            }),
            card_holder_info,
            merchant_transaction_id: Some(item.router_data.connector_request_reference_id.clone()),
            transaction_meta_data,
        })
    }
}

impl From<payment_method_data::ApplepayPaymentMethod> for ApplepayPaymentMethod {
    fn from(item: payment_method_data::ApplepayPaymentMethod) -> Self {
        Self {
            display_name: item.display_name,
            network: item.network,
            pm_type: item.pm_type,
        }
    }
}

impl TryFrom<&types::PaymentsSessionRouterData> for BluesnapCreateWalletToken {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSessionRouterData) -> Result<Self, Self::Error> {
        let apple_pay_metadata = item.get_connector_meta()?.expose();
        let applepay_metadata = apple_pay_metadata
            .clone()
            .parse_value::<ApplepayCombinedSessionTokenData>("ApplepayCombinedSessionTokenData")
            .map(|combined_metadata| {
                ApplepaySessionTokenMetadata::ApplePayCombined(combined_metadata.apple_pay_combined)
            })
            .or_else(|_| {
                apple_pay_metadata
                    .parse_value::<ApplepaySessionTokenData>("ApplepaySessionTokenData")
                    .map(|old_metadata| {
                        ApplepaySessionTokenMetadata::ApplePay(old_metadata.apple_pay)
                    })
            })
            .change_context(errors::ConnectorError::ParsingFailed)?;
        let session_token_data = match applepay_metadata {
            ApplepaySessionTokenMetadata::ApplePay(apple_pay_data) => {
                Ok(apple_pay_data.session_token_data)
            }
            ApplepaySessionTokenMetadata::ApplePayCombined(_apple_pay_combined_data) => {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: "apple pay combined".to_string(),
                    connector: "bluesnap".to_string(),
                })
            }
        }?;
        let domain_name = session_token_data.initiative_context.ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "apple pay initiative_context",
            },
        )?;

        Ok(Self {
            wallet_type: "APPLE_PAY".to_string(),
            validation_url: APPLEPAY_VALIDATION_URL.to_string().into(),
            domain_name,
            display_name: Some(session_token_data.display_name),
        })
    }
}

impl
    ForeignTryFrom<(
        PaymentsSessionResponseRouterData<BluesnapWalletTokenResponse>,
        StringMajorUnit,
    )> for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, apple_pay_amount): (
            PaymentsSessionResponseRouterData<BluesnapWalletTokenResponse>,
            StringMajorUnit,
        ),
    ) -> Result<Self, Self::Error> {
        let response = &item.response;

        let wallet_token = BASE64_ENGINE
            .decode(response.wallet_token.clone().expose())
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let session_response: NoThirdPartySdkSessionResponse = wallet_token
            .parse_struct("NoThirdPartySdkSessionResponse")
            .change_context(errors::ConnectorError::ParsingFailed)?;

        let metadata = item.data.get_connector_meta()?.expose();
        let applepay_metadata = metadata
            .clone()
            .parse_value::<ApplepayCombinedSessionTokenData>("ApplepayCombinedSessionTokenData")
            .map(|combined_metadata| {
                ApplepaySessionTokenMetadata::ApplePayCombined(combined_metadata.apple_pay_combined)
            })
            .or_else(|_| {
                metadata
                    .parse_value::<ApplepaySessionTokenData>("ApplepaySessionTokenData")
                    .map(|old_metadata| {
                        ApplepaySessionTokenMetadata::ApplePay(old_metadata.apple_pay)
                    })
            })
            .change_context(errors::ConnectorError::ParsingFailed)?;

        let (payment_request_data, session_token_data) = match applepay_metadata {
            ApplepaySessionTokenMetadata::ApplePayCombined(_apple_pay_combined) => {
                Err(errors::ConnectorError::FlowNotSupported {
                    flow: "apple pay combined".to_string(),
                    connector: "bluesnap".to_string(),
                })
            }
            ApplepaySessionTokenMetadata::ApplePay(apple_pay) => {
                Ok((apple_pay.payment_request_data, apple_pay.session_token_data))
            }
        }?;

        Ok(Self {
            response: Ok(PaymentsResponseData::SessionResponse {
                session_token: SessionToken::ApplePay(Box::new(ApplepaySessionTokenResponse {
                    session_token_data: Some(ApplePaySessionResponse::NoThirdPartySdk(
                        session_response,
                    )),
                    payment_request_data: Some(ApplePayPaymentRequest {
                        country_code: item.data.get_billing_country()?,
                        currency_code: item.data.request.currency,
                        total: AmountInfo {
                            label: payment_request_data.label,
                            total_type: Some("final".to_string()),
                            amount: apple_pay_amount,
                        },
                        merchant_capabilities: Some(payment_request_data.merchant_capabilities),
                        supported_networks: Some(payment_request_data.supported_networks),
                        merchant_identifier: Some(session_token_data.merchant_identifier),
                        required_billing_contact_fields: None,
                        required_shipping_contact_fields: None,
                        recurring_payment_request: None,
                    }),
                    connector: "bluesnap".to_string(),
                    delayed_session_token: false,
                    sdk_next_action: {
                        SdkNextAction {
                            next_action: NextActionCall::Confirm,
                        }
                    },
                    connector_reference_id: None,
                    connector_sdk_public_key: None,
                    connector_merchant_id: None,
                })),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCompletePaymentsRequest {
    amount: StringMajorUnit,
    currency: enums::Currency,
    card_transaction_type: BluesnapTxnType,
    pf_token: Secret<String>,
    three_d_secure: Option<BluesnapThreeDSecureInfo>,
    transaction_fraud_info: Option<TransactionFraudInfo>,
    card_holder_info: Option<BluesnapCardHolderInfo>,
    merchant_transaction_id: Option<String>,
    transaction_meta_data: Option<BluesnapMetadata>,
}

impl TryFrom<&BluesnapRouterData<&types::PaymentsCompleteAuthorizeRouterData>>
    for BluesnapCompletePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluesnapRouterData<&types::PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let redirection_response: BluesnapRedirectionResponse = item
            .router_data
            .request
            .redirect_response
            .as_ref()
            .and_then(|res| res.payload.to_owned())
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .parse_value("BluesnapRedirectionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let transaction_meta_data =
            item.router_data
                .request
                .metadata
                .as_ref()
                .map(|metadata| BluesnapMetadata {
                    meta_data: convert_metadata_to_request_metadata(metadata.to_owned()),
                });

        let token = item
            .router_data
            .request
            .redirect_response
            .clone()
            .and_then(|res| res.params.to_owned())
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.params",
            })?
            .peek()
            .split_once('=')
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.params.paymentToken",
            })?
            .1
            .to_string();

        let redirection_result: BluesnapThreeDsResult = redirection_response
            .authentication_response
            .parse_struct("BluesnapThreeDsResult")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let auth_mode = match item.router_data.request.capture_method {
            Some(enums::CaptureMethod::Manual) => BluesnapTxnType::AuthOnly,
            _ => BluesnapTxnType::AuthCapture,
        };
        Ok(Self {
            amount: item.amount.to_owned(),
            currency: item.router_data.request.currency,
            card_transaction_type: auth_mode,
            three_d_secure: Some(BluesnapThreeDSecureInfo {
                three_d_secure_reference_id: redirection_result
                    .three_d_secure
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "three_d_secure_reference_id",
                    })?
                    .three_d_secure_reference_id,
            }),
            transaction_fraud_info: Some(TransactionFraudInfo {
                fraud_session_id: item.router_data.payment_id.clone(),
            }),
            card_holder_info: get_card_holder_info(
                item.router_data.get_billing_address()?,
                item.router_data.request.get_email()?,
            )?,
            merchant_transaction_id: Some(item.router_data.connector_request_reference_id.clone()),
            pf_token: Secret::new(token),
            transaction_meta_data,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct BluesnapRedirectionResponse {
    pub authentication_response: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDsResult {
    three_d_secure: Option<BluesnapThreeDsReference>,
    pub status: String,
    pub code: Option<String>,
    pub info: Option<RedirectErrorMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedirectErrorMessage {
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDsReference {
    three_d_secure_reference_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapVoidRequest {
    card_transaction_type: BluesnapTxnType,
    transaction_id: String,
}

impl TryFrom<&types::PaymentsCancelRouterData> for BluesnapVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = BluesnapTxnType::AuthReversal;
        let transaction_id = item.request.connector_transaction_id.to_string();
        Ok(Self {
            card_transaction_type,
            transaction_id,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCaptureRequest {
    card_transaction_type: BluesnapTxnType,
    transaction_id: String,
    amount: Option<StringMajorUnit>,
}

impl TryFrom<&BluesnapRouterData<&types::PaymentsCaptureRouterData>> for BluesnapCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluesnapRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let card_transaction_type = BluesnapTxnType::Capture;
        let transaction_id = item
            .router_data
            .request
            .connector_transaction_id
            .to_string();
        Ok(Self {
            card_transaction_type,
            transaction_id,
            amount: Some(item.amount.to_owned()),
        })
    }
}

// Auth Struct
pub struct BluesnapAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) key1: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BluesnapAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapTxnType {
    AuthOnly,
    AuthCapture,
    AuthReversal,
    Capture,
    Refund,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BluesnapProcessingStatus {
    #[serde(alias = "success")]
    Success,
    #[default]
    #[serde(alias = "pending")]
    Pending,
    #[serde(alias = "fail")]
    Fail,
    #[serde(alias = "pending_merchant_review")]
    PendingMerchantReview,
}

impl ForeignTryFrom<(BluesnapTxnType, BluesnapProcessingStatus)> for enums::AttemptStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        item: (BluesnapTxnType, BluesnapProcessingStatus),
    ) -> Result<Self, Self::Error> {
        let (item_txn_status, item_processing_status) = item;
        Ok(match item_processing_status {
            BluesnapProcessingStatus::Success => match item_txn_status {
                BluesnapTxnType::AuthOnly => Self::Authorized,
                BluesnapTxnType::AuthReversal => Self::Voided,
                BluesnapTxnType::AuthCapture | BluesnapTxnType::Capture => Self::Charged,
                BluesnapTxnType::Refund => Self::Charged,
            },
            BluesnapProcessingStatus::Pending | BluesnapProcessingStatus::PendingMerchantReview => {
                Self::Pending
            }
            BluesnapProcessingStatus::Fail => Self::Failure,
        })
    }
}

impl From<BluesnapProcessingStatus> for enums::RefundStatus {
    fn from(item: BluesnapProcessingStatus) -> Self {
        match item {
            BluesnapProcessingStatus::Success => Self::Success,
            BluesnapProcessingStatus::Pending => Self::Pending,
            BluesnapProcessingStatus::PendingMerchantReview => Self::ManualReview,
            BluesnapProcessingStatus::Fail => Self::Failure,
        }
    }
}

impl From<BluesnapRefundStatus> for enums::RefundStatus {
    fn from(item: BluesnapRefundStatus) -> Self {
        match item {
            BluesnapRefundStatus::Success => Self::Success,
            BluesnapRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsResponse {
    pub processing_info: ProcessingInfoResponse,
    pub transaction_id: String,
    pub card_transaction_type: BluesnapTxnType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWalletTokenResponse {
    wallet_type: String,
    wallet_token: Secret<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    refund_transaction_id: String,
    amount: StringMajorUnit,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInfoResponse {
    pub processing_status: BluesnapProcessingStatus,
    pub authorization_code: Option<String>,
    pub network_transaction_id: Option<Secret<String>>,
}

impl<F, T> TryFrom<ResponseRouterData<F, BluesnapPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BluesnapPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_try_from((
                item.response.card_transaction_type,
                item.response.processing_info.processing_status,
            ))?,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct BluesnapRefundRequest {
    amount: Option<StringMajorUnit>,
    reason: Option<String>,
}

impl<F> TryFrom<&BluesnapRouterData<&types::RefundsRouterData<F>>> for BluesnapRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BluesnapRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            reason: item.router_data.request.reason.clone(),
            amount: Some(item.amount.to_owned()),
        })
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BluesnapRefundStatus {
    Success,
    #[default]
    Pending,
}
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_transaction_id: i32,
    refund_status: BluesnapRefundStatus,
}

impl TryFrom<RefundsResponseRouterData<RSync, BluesnapPaymentsResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, BluesnapPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id.clone(),
                refund_status: enums::RefundStatus::from(
                    item.response.processing_info.processing_status,
                ),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.refund_transaction_id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.refund_status),
            }),
            ..item.data
        })
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookBody {
    pub merchant_transaction_id: String,
    pub reference_number: String,
    pub transaction_type: BluesnapWebhookEvents,
    pub reversal_ref_num: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectEventType {
    transaction_type: BluesnapWebhookEvents,
    cb_status: Option<BluesnapChargebackStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapChargebackStatus {
    #[serde(alias = "New")]
    New,
    #[serde(alias = "Working")]
    Working,
    #[serde(alias = "Closed")]
    Closed,
    #[serde(alias = "Completed_Lost")]
    CompletedLost,
    #[serde(alias = "Completed_Pending")]
    CompletedPending,
    #[serde(alias = "Completed_Won")]
    CompletedWon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapWebhookEvents {
    Decline,
    CcChargeFailed,
    Charge,
    Refund,
    Chargeback,
    ChargebackStatusChanged,
    #[serde(other)]
    Unknown,
}

impl TryFrom<BluesnapWebhookObjectEventType> for IncomingWebhookEvent {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(details: BluesnapWebhookObjectEventType) -> Result<Self, Self::Error> {
        match details.transaction_type {
            BluesnapWebhookEvents::Decline | BluesnapWebhookEvents::CcChargeFailed => {
                Ok(Self::PaymentIntentFailure)
            }
            BluesnapWebhookEvents::Charge => Ok(Self::PaymentIntentSuccess),
            BluesnapWebhookEvents::Refund => Ok(Self::RefundSuccess),
            BluesnapWebhookEvents::Chargeback | BluesnapWebhookEvents::ChargebackStatusChanged => {
                match details
                    .cb_status
                    .ok_or(errors::ConnectorError::WebhookEventTypeNotFound)?
                {
                    BluesnapChargebackStatus::New | BluesnapChargebackStatus::Working => {
                        Ok(Self::DisputeOpened)
                    }
                    BluesnapChargebackStatus::Closed => Ok(Self::DisputeExpired),
                    BluesnapChargebackStatus::CompletedLost => Ok(Self::DisputeLost),
                    BluesnapChargebackStatus::CompletedPending => Ok(Self::DisputeChallenged),
                    BluesnapChargebackStatus::CompletedWon => Ok(Self::DisputeWon),
                }
            }
            BluesnapWebhookEvents::Unknown => Ok(Self::EventNotSupported),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapDisputeWebhookBody {
    pub invoice_charge_amount: f64,
    pub currency: enums::Currency,
    pub reversal_reason: Option<String>,
    pub reversal_ref_num: String,
    pub cb_status: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectResource {
    reference_number: String,
    transaction_type: BluesnapWebhookEvents,
    reversal_ref_num: Option<String>,
}

impl TryFrom<BluesnapWebhookObjectResource> for Value {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(details: BluesnapWebhookObjectResource) -> Result<Self, Self::Error> {
        let (card_transaction_type, processing_status, transaction_id) = match details
            .transaction_type
        {
            BluesnapWebhookEvents::Decline | BluesnapWebhookEvents::CcChargeFailed => Ok((
                BluesnapTxnType::Capture,
                BluesnapProcessingStatus::Fail,
                details.reference_number,
            )),
            BluesnapWebhookEvents::Charge => Ok((
                BluesnapTxnType::Capture,
                BluesnapProcessingStatus::Success,
                details.reference_number,
            )),
            BluesnapWebhookEvents::Chargeback | BluesnapWebhookEvents::ChargebackStatusChanged => {
                //It won't be consumed in dispute flow, so currently does not hold any significance
                return serde_json::to_value(details)
                    .change_context(errors::ConnectorError::WebhookBodyDecodingFailed);
            }
            BluesnapWebhookEvents::Refund => Ok((
                BluesnapTxnType::Refund,
                BluesnapProcessingStatus::Success,
                details
                    .reversal_ref_num
                    .ok_or(errors::ConnectorError::WebhookResourceObjectNotFound)?,
            )),
            BluesnapWebhookEvents::Unknown => {
                Err(errors::ConnectorError::WebhookResourceObjectNotFound)
            }
        }?;
        let sync_struct = BluesnapPaymentsResponse {
            processing_info: ProcessingInfoResponse {
                processing_status,
                authorization_code: None,
                network_transaction_id: None,
            },
            transaction_id,
            card_transaction_type,
        };
        serde_json::to_value(sync_struct)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub code: String,
    pub description: String,
    pub error_name: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapErrorResponse {
    pub message: Vec<ErrorDetails>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapAuthErrorResponse {
    pub error_code: String,
    pub error_description: String,
    pub error_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BluesnapErrors {
    Payment(BluesnapErrorResponse),
    Auth(BluesnapAuthErrorResponse),
    General(String),
}

fn get_card_holder_info(
    address: &AddressDetails,
    email: Email,
) -> CustomResult<Option<BluesnapCardHolderInfo>, errors::ConnectorError> {
    let first_name = address.get_first_name()?;
    Ok(Some(BluesnapCardHolderInfo {
        first_name: first_name.clone(),
        last_name: address.get_last_name().unwrap_or(first_name).clone(),
        email,
    }))
}

impl From<ErrorDetails> for utils::ErrorCodeAndMessage {
    fn from(error: ErrorDetails) -> Self {
        Self {
            error_code: error.code.to_string(),
            error_message: error.error_name.unwrap_or(error.code),
        }
    }
}

fn convert_metadata_to_request_metadata(metadata: Value) -> Vec<RequestMetadata> {
    let hashmap: HashMap<Option<String>, Option<Value>> =
        serde_json::from_str(&metadata.to_string()).unwrap_or(HashMap::new());
    let mut vector = Vec::<RequestMetadata>::new();
    for (key, value) in hashmap {
        vector.push(RequestMetadata {
            meta_key: key,
            meta_value: value.map(|field_value| field_value.to_string()),
            is_visible: Some(DISPLAY_METADATA.to_string()),
        });
    }
    vector
}
