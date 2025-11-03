use api_models::payments::{
    ApplePayCombinedMetadata, ApplepayCombinedSessionTokenData, ApplepaySessionTokenData,
    ApplepaySessionTokenMetadata,
};
use base64::Engine;
use common_enums::{enums, Currency};
use common_utils::{
    consts::BASE64_ENGINE, ext_traits::ValueExt, pii, request::Method, types::FloatMajorUnit,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{ser::Serializer, Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{
        self, CardData as _, PaymentsCancelRequestData, PaymentsSyncRequestData, RouterData as _,
    },
};

#[derive(Debug, Serialize)]
pub struct FiservRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> TryFrom<(FloatMajorUnit, T)> for FiservRouterData<T> {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from((amount, router_data): (FloatMajorUnit, T)) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
        })
    }
}

impl Serialize for FiservCheckoutChargesRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Checkout(inner) => inner.serialize(serializer),
            Self::Charges(inner) => inner.serialize(serializer),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentsRequest {
    amount: Amount,
    merchant_details: MerchantDetails,
    #[serde(flatten)]
    checkout_charges_request: FiservCheckoutChargesRequest,
}

#[derive(Debug)]
pub enum FiservCheckoutChargesRequest {
    Checkout(CheckoutPaymentsRequest),
    Charges(ChargesPaymentRequest),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckoutPaymentsRequest {
    order: FiservOrder,
    payment_method: FiservPaymentMethod,
    interactions: FiservInteractions,
    transaction_details: TransactionDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservChannel {
    Web,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservPaymentInitiator {
    Merchant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FiservCustomerConfirmation {
    ReviewAndPay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservInteractions {
    channel: FiservChannel,
    customer_confirmation: FiservCustomerConfirmation,
    payment_initiator: FiservPaymentInitiator,
    return_urls: FiservReturnUrls,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservReturnUrls {
    success_url: String,
    cancel_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservPaymentMethod {
    provider: FiservWallet,
    #[serde(rename = "type")]
    wallet_type: FiservWalletType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservOrder {
    intent: FiservIntent,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservIntent {
    Authorize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargesPaymentRequest {
    source: Source,
    transaction_interaction: Option<TransactionInteraction>,
    transaction_details: TransactionDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservWallet {
    ApplePay,
    GooglePay,
    PayPal,
}

#[derive(Debug, Serialize)]
#[serde(tag = "sourceType")]
pub enum Source {
    #[serde(rename = "GooglePay")]
    GooglePay(GooglePayData),
    #[serde(rename = "PaymentCard")]
    PaymentCard { card: CardData },
    #[serde(rename = "ApplePay")]
    ApplePay(ApplePayWalletDetails),
    #[serde(rename = "DecryptedWallet")]
    DecryptedWallet(DecryptedWalletDetails),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayWalletDetails {
    pub data: Secret<String>,
    pub header: ApplePayHeader,
    pub signature: Secret<String>,
    pub version: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_data: Option<Secret<String>>,
    pub apple_pay_merchant_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayHeader {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_data_hash: Option<Secret<String>>,
    pub ephemeral_public_key: Secret<String>,
    pub public_key_hash: Secret<String>,
    pub transaction_id: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecryptedWalletDetails {
    pub card: WalletCardData,
    #[serde(rename = "cavv")]
    pub cryptogram: Secret<String>,
    #[serde(rename = "xid")]
    pub transaction_id: Secret<String>,
    pub wallet_type: FiservWalletType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservWalletType {
    ApplePay,
    GooglePay,
    PaypalWallet,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayData {
    data: Secret<String>,
    signature: Secret<String>,
    version: String,
    intermediate_signing_key: IntermediateSigningKey,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardData {
    card_data: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    security_code: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletCardData {
    card_data: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct Amount {
    total: FloatMajorUnit,
    currency: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    capture_flag: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reversal_reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_transaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_type: Option<OperationType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperationType {
    Create,
    Capture,
    Authorize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDetails {
    merchant_id: Secret<String>,
    terminal_id: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInteraction {
    origin: TransactionInteractionOrigin,
    eci_indicator: TransactionInteractionEciIndicator,
    pos_condition_code: TransactionInteractionPosConditionCode,
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionInteractionOrigin {
    #[default]
    Ecom,
}
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionEciIndicator {
    #[default]
    ChannelEncrypted,
}
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionInteractionPosConditionCode {
    #[default]
    CardNotPresentEcom,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntermediateSigningKey {
    signed_key: Secret<String>,
    signatures: Vec<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedKey {
    key_value: String,
    key_expiration: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessage {
    encrypted_message: String,
    ephemeral_public_key: String,
    tag: String,
}

#[derive(Debug, Default)]
pub struct FullyParsedGooglePayToken {
    pub signature: Secret<String>,
    pub protocol_version: String,
    pub encrypted_message: String,
    pub ephemeral_public_key: String,
    pub tag: String,
    pub key_value: String,
    pub key_expiration: String,
    pub signatures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawGooglePayToken {
    pub signature: Secret<String>,
    pub protocol_version: String,
    pub signed_message: Secret<String>,
    pub intermediate_signing_key: IntermediateSigningKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplePayDecryptedData {
    pub data: Secret<String>,
    pub signature: Secret<String>,
    pub version: Secret<String>,
    pub header: ApplePayHeader,
}

pub fn parse_googlepay_token_safely(token_json_str: &str) -> FullyParsedGooglePayToken {
    let mut result = FullyParsedGooglePayToken::default();

    if let Ok(raw_token) = serde_json::from_str::<RawGooglePayToken>(token_json_str) {
        result.signature = raw_token.signature;
        result.protocol_version = raw_token.protocol_version;
        result.signatures = raw_token
            .intermediate_signing_key
            .signatures
            .into_iter()
            .map(|s| s.expose().to_owned())
            .collect();

        if let Ok(key) = serde_json::from_str::<SignedKey>(
            &raw_token.intermediate_signing_key.signed_key.expose(),
        ) {
            result.key_value = key.key_value;
            result.key_expiration = key.key_expiration;
        }

        if let Ok(message) =
            serde_json::from_str::<SignedMessage>(&raw_token.signed_message.expose())
        {
            result.encrypted_message = message.encrypted_message;
            result.ephemeral_public_key = message.ephemeral_public_key;
            result.tag = message.tag;
        }
    }

    result
}

impl TryFrom<&FiservRouterData<&types::PaymentsAuthorizeRouterData>> for FiservPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &FiservRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Cards 3DS".to_string(),
                connector: "Fiserv",
            })?
        }

        let auth: FiservAuthType = FiservAuthType::try_from(&item.router_data.connector_auth_type)?;
        let amount = Amount {
            total: item.amount,
            currency: item.router_data.request.currency.to_string(),
        };
        let metadata = item.router_data.get_connector_meta()?.clone();
        let session: FiservSessionObject = metadata
            .expose()
            .parse_value("FiservSessionObject")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "Merchant connector account metadata",
            })?;

        let merchant_details = MerchantDetails {
            merchant_id: auth.merchant_account,
            terminal_id: Some(session.terminal_id),
        };

        let checkout_charges_request = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ref ccard) => {
                Ok(FiservCheckoutChargesRequest::Charges(
                    ChargesPaymentRequest {
                        source: Source::PaymentCard {
                            card: CardData {
                                card_data: ccard.card_number.clone(),
                                expiration_month: ccard.card_exp_month.clone(),
                                expiration_year: ccard.get_expiry_year_4_digit(),
                                security_code: Some(ccard.card_cvc.clone()),
                            },
                        },
                        transaction_details: TransactionDetails {
                            capture_flag: Some(matches!(
                                item.router_data.request.capture_method,
                                Some(enums::CaptureMethod::Automatic)
                                    | Some(enums::CaptureMethod::SequentialAutomatic)
                                    | None
                            )),
                            reversal_reason_code: None,
                            merchant_transaction_id: Some(
                                item.router_data.connector_request_reference_id.clone(),
                            ),
                            operation_type: None,
                        },
                        transaction_interaction: Some(TransactionInteraction {
                            //Payment is being made in online mode, card not present
                            origin: TransactionInteractionOrigin::Ecom,
                            // transaction encryption such as SSL/TLS, but authentication was not performed
                            eci_indicator: TransactionInteractionEciIndicator::ChannelEncrypted,
                            //card not present in online transaction
                            pos_condition_code:
                                TransactionInteractionPosConditionCode::CardNotPresentEcom,
                        }),
                    },
                ))
            }
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(data) => {
                    let token_string = data
                        .tokenization_data
                        .get_encrypted_google_pay_token()
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "gpay wallet_token",
                        })?
                        .to_owned();

                    let parsed = parse_googlepay_token_safely(&token_string);

                    Ok(FiservCheckoutChargesRequest::Charges(
                        ChargesPaymentRequest {
                            source: Source::GooglePay(GooglePayData {
                                data: Secret::new(parsed.encrypted_message),
                                signature: Secret::new(parsed.signature.expose().to_owned()),
                                version: parsed.protocol_version,
                                intermediate_signing_key: IntermediateSigningKey {
                                    signed_key: Secret::new(
                                        serde_json::json!({
                                            "keyValue": parsed.key_value,
                                            "keyExpiration": parsed.key_expiration
                                        })
                                        .to_string(),
                                    ),
                                    signatures: parsed
                                        .signatures
                                        .into_iter()
                                        .map(|s| Secret::new(s.to_owned()))
                                        .collect(),
                                },
                            }),
                            transaction_details: TransactionDetails {
                                capture_flag: Some(matches!(
                                    item.router_data.request.capture_method,
                                    Some(enums::CaptureMethod::Automatic)
                                        | Some(enums::CaptureMethod::SequentialAutomatic)
                                        | None
                                )),
                                reversal_reason_code: None,
                                merchant_transaction_id: Some(
                                    item.router_data.connector_request_reference_id.clone(),
                                ),
                                operation_type: None,
                            },
                            transaction_interaction: None,
                        },
                    ))
                }
                WalletData::PaypalRedirect(_) => {
                    let return_url = item
                        .router_data
                        .request
                        .complete_authorize_url
                        .clone()
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "return_url",
                        })?;
                    Ok(FiservCheckoutChargesRequest::Checkout(
                        CheckoutPaymentsRequest {
                            payment_method: FiservPaymentMethod {
                                provider: FiservWallet::PayPal,
                                wallet_type: FiservWalletType::PaypalWallet,
                            },
                            order: FiservOrder {
                                intent: FiservIntent::Authorize,
                            },
                            interactions: FiservInteractions {
                                channel: FiservChannel::Web,
                                customer_confirmation: FiservCustomerConfirmation::ReviewAndPay,
                                payment_initiator: FiservPaymentInitiator::Merchant,
                                return_urls: FiservReturnUrls {
                                    success_url: return_url.clone(),
                                    cancel_url: return_url,
                                },
                            },
                            transaction_details: TransactionDetails {
                                operation_type: Some(OperationType::Create),
                                capture_flag: Some(matches!(
                                    item.router_data.request.capture_method,
                                    Some(enums::CaptureMethod::Automatic)
                                        | Some(enums::CaptureMethod::SequentialAutomatic)
                                        | None
                                )),
                                reversal_reason_code: None,
                                merchant_transaction_id: Some(
                                    item.router_data.connector_request_reference_id.clone(),
                                ),
                            },
                        },
                    ))
                }
                WalletData::ApplePay(apple_pay_data) => match item
                    .router_data
                    .payment_method_token
                    .clone()
                {
                    Some(PaymentMethodToken::ApplePayDecrypt(pre_decrypt_data)) => Ok(
                        FiservCheckoutChargesRequest::Charges(ChargesPaymentRequest {
                            source: Source::DecryptedWallet(DecryptedWalletDetails {
                                wallet_type: FiservWalletType::ApplePay,
                                cryptogram: pre_decrypt_data
                                    .payment_data
                                    .online_payment_cryptogram
                                    .clone(),
                                transaction_id: Secret::new(apple_pay_data.transaction_identifier),
                                card: WalletCardData {
                                    card_data: pre_decrypt_data
                                        .application_primary_account_number
                                        .clone(),
                                    expiration_month: pre_decrypt_data
                                        .get_expiry_month()
                                        .change_context(
                                            errors::ConnectorError::MissingRequiredField {
                                                field_name: "apple_pay_expiry_month",
                                            },
                                        )?,
                                    expiration_year: pre_decrypt_data.get_four_digit_expiry_year(),
                                },
                            }),
                            transaction_details: TransactionDetails {
                                capture_flag: Some(matches!(
                                    item.router_data.request.capture_method,
                                    Some(enums::CaptureMethod::Automatic)
                                        | Some(enums::CaptureMethod::SequentialAutomatic)
                                        | None
                                )),
                                reversal_reason_code: None,
                                merchant_transaction_id: Some(
                                    item.router_data.connector_request_reference_id.clone(),
                                ),
                                operation_type: None,
                            },
                            transaction_interaction: None,
                        }),
                    ),
                    _ => {
                        let decoded_bytes = match apple_pay_data.payment_data {
                            common_types::payments::ApplePayPaymentData::Encrypted(
                                ref encrypted_str,
                            ) => BASE64_ENGINE
                                .decode(encrypted_str)
                                .change_context(errors::ConnectorError::ParsingFailed)?,
                            _ => {
                                return Err(errors::ConnectorError::ParsingFailed.into());
                            }
                        };

                        let payment_data_decoded: ApplePayDecryptedData =
                            serde_json::from_slice(&decoded_bytes)
                                .change_context(errors::ConnectorError::ParsingFailed)?;

                        let data = payment_data_decoded.data;
                        let signature = payment_data_decoded.signature;
                        let version = payment_data_decoded.version;

                        let header = ApplePayHeader {
                            ephemeral_public_key: payment_data_decoded.header.ephemeral_public_key,
                            public_key_hash: payment_data_decoded.header.public_key_hash,
                            transaction_id: payment_data_decoded.header.transaction_id,
                            application_data_hash: None,
                        };

                        let apple_pay_metadata = item.router_data.get_connector_meta()?.expose();
                        let applepay_metadata = apple_pay_metadata
                            .clone()
                            .parse_value::<ApplepayCombinedSessionTokenData>(
                                "ApplepayCombinedSessionTokenData",
                            )
                            .map(|combined_metadata| {
                                ApplepaySessionTokenMetadata::ApplePayCombined(
                                    combined_metadata.apple_pay_combined,
                                )
                            })
                            .or_else(|_| {
                                apple_pay_metadata
                                    .parse_value::<ApplepaySessionTokenData>(
                                        "ApplepaySessionTokenData",
                                    )
                                    .map(|old_metadata| {
                                        ApplepaySessionTokenMetadata::ApplePay(
                                            old_metadata.apple_pay,
                                        )
                                    })
                            })
                            .change_context(errors::ConnectorError::ParsingFailed)?;

                        let merchant_identifier = match applepay_metadata {
                            ApplepaySessionTokenMetadata::ApplePayCombined(ref combined) => {
                                match combined {
                                    ApplePayCombinedMetadata::Simplified { .. } => {
                                        return Err(
                                            errors::ConnectorError::MissingApplePayTokenData.into(),
                                        )
                                    }
                                    ApplePayCombinedMetadata::Manual {
                                        session_token_data, ..
                                    } => &session_token_data.merchant_identifier,
                                }
                            }
                            ApplepaySessionTokenMetadata::ApplePay(ref data) => {
                                &data.session_token_data.merchant_identifier
                            }
                        };

                        Ok(FiservCheckoutChargesRequest::Charges(
                            ChargesPaymentRequest {
                                source: Source::ApplePay(ApplePayWalletDetails {
                                    data,
                                    header,
                                    signature,
                                    version,
                                    application_data: None,
                                    apple_pay_merchant_id: Secret::new(
                                        merchant_identifier.to_owned(),
                                    ),
                                }),
                                transaction_details: TransactionDetails {
                                    capture_flag: Some(matches!(
                                        item.router_data.request.capture_method,
                                        Some(enums::CaptureMethod::Automatic)
                                            | Some(enums::CaptureMethod::SequentialAutomatic)
                                            | None
                                    )),
                                    reversal_reason_code: None,
                                    merchant_transaction_id: Some(
                                        item.router_data.connector_request_reference_id.clone(),
                                    ),
                                    operation_type: None,
                                },
                                transaction_interaction: None,
                            },
                        ))
                    }
                },
                _ => Err(error_stack::report!(
                    errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("fiserv"),
                    )
                )),
            },
            PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => Err(
                error_stack::report!(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("fiserv"),
                )),
            ),
        }?;

        Ok(Self {
            amount,
            checkout_charges_request,
            merchant_details,
        })
    }
}

pub struct FiservAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for FiservAuthType {
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
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservCancelRequest {
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl TryFrom<&types::PaymentsCancelRouterData> for FiservCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        let metadata = item.get_connector_meta()?.clone();
        let session: FiservSessionObject = metadata
            .expose()
            .parse_value("FiservSessionObject")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "Merchant connector account metadata",
            })?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item.request.connector_transaction_id.to_string(),
            },
            transaction_details: TransactionDetails {
                capture_flag: None,
                reversal_reason_code: Some(item.request.get_cancellation_reason()?),
                merchant_transaction_id: Some(item.connector_request_reference_id.clone()),
                operation_type: None,
            },
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: Option<Vec<ErrorDetails>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub code: Option<String>,
    pub field: Option<String>,
    pub message: Option<String>,
    pub additional_info: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservPaymentStatus {
    Succeeded,
    Failed,
    Captured,
    Declined,
    Voided,
    Authorized,
    #[default]
    Processing,
    Created,
}

impl From<FiservPaymentStatus> for enums::AttemptStatus {
    fn from(item: FiservPaymentStatus) -> Self {
        match item {
            FiservPaymentStatus::Captured | FiservPaymentStatus::Succeeded => Self::Charged,
            FiservPaymentStatus::Declined | FiservPaymentStatus::Failed => Self::Failure,
            FiservPaymentStatus::Processing => Self::Authorizing,
            FiservPaymentStatus::Voided => Self::Voided,
            FiservPaymentStatus::Authorized => Self::Authorized,
            FiservPaymentStatus::Created => Self::AuthenticationPending,
        }
    }
}

impl From<FiservPaymentStatus> for enums::RefundStatus {
    fn from(item: FiservPaymentStatus) -> Self {
        match item {
            FiservPaymentStatus::Succeeded
            | FiservPaymentStatus::Authorized
            | FiservPaymentStatus::Captured => Self::Success,
            FiservPaymentStatus::Declined | FiservPaymentStatus::Failed => Self::Failure,
            FiservPaymentStatus::Voided
            | FiservPaymentStatus::Processing
            | FiservPaymentStatus::Created => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessorResponseDetails {
    pub approval_status: Option<String>,
    pub approval_code: Option<String>,
    pub reference_number: Option<String>,
    pub processor: Option<String>,
    pub host: Option<String>,
    pub network_routed: Option<String>,
    pub network_international_id: Option<String>,
    pub response_code: Option<String>,
    pub response_message: Option<String>,
    pub host_response_code: Option<String>,
    pub host_response_message: Option<String>,
    pub additional_info: Option<Vec<AdditionalInfo>>,
    pub bank_association_details: Option<BankAssociationDetails>,
    pub response_indicators: Option<ResponseIndicators>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalInfo {
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAssociationDetails {
    pub association_response_code: Option<String>,
    pub avs_security_code_response: Option<AvsSecurityCodeResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvsSecurityCodeResponse {
    pub street_match: Option<String>,
    pub postal_code_match: Option<String>,
    pub security_code_match: Option<String>,
    pub association: Option<Association>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Association {
    pub avs_code: Option<String>,
    pub security_code_response: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseIndicators {
    pub alternate_route_debit_indicator: Option<bool>,
    pub signature_line_indicator: Option<bool>,
    pub signature_debit_route_indicator: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservChargesResponse {
    pub gateway_response: GatewayResponse,
    pub payment_receipt: PaymentReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservCheckoutResponse {
    pub gateway_response: GatewayResponse,
    pub payment_receipt: PaymentReceipt,
    pub interactions: FiservResponseInteractions,
    pub order: Option<FiservResponseOrders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservResponseInteractions {
    actions: FiservResponseActions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservResponseActions {
    #[serde(rename = "type")]
    action_type: String,
    url: url::Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservResponseOrders {
    intent: FiservIntent,
    order_id: String,
    order_status: FiservOrderStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FiservOrderStatus {
    PayerActionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FiservPaymentsResponse {
    Charges(FiservChargesResponse),
    Checkout(FiservCheckoutResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentReceipt {
    pub approved_amount: ApprovedAmount,
    pub processor_response_details: Option<ProcessorResponseDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedAmount {
    pub total: FloatMajorUnit,
    pub currency: Currency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(transparent)]
pub struct FiservSyncResponse {
    pub sync_responses: Vec<FiservPaymentsResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayResponse {
    gateway_transaction_id: Option<String>,
    transaction_state: FiservPaymentStatus,
    transaction_processing_details: TransactionProcessingDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionProcessingDetails {
    order_id: String,
    transaction_id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, FiservPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FiservPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let (gateway_resp, redirect_url, order_id) = match &item.response {
            FiservPaymentsResponse::Charges(res) => (res.gateway_response.clone(), None, None),
            FiservPaymentsResponse::Checkout(res) => (
                res.gateway_response.clone(),
                Some(res.interactions.actions.url.clone()),
                res.order.as_ref().map(|o| o.order_id.clone()),
            ),
        };

        let redirection_data = redirect_url.map(|url| RedirectForm::from((url, Method::Get)));

        let connector_metadata: Option<serde_json::Value> = Some(serde_json::json!({
            "order_id": order_id,
        }));

        Ok(Self {
            status: enums::AttemptStatus::from(gateway_resp.transaction_state),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(
                    gateway_resp.transaction_processing_details.transaction_id,
                ),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: Some(
                    gateway_resp.transaction_processing_details.order_id,
                ),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, FiservSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, FiservSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let gateway_resp = match item.response.sync_responses.first() {
            Some(gateway_response) => gateway_response,
            None => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        };

        let connector_response_reference_id = match gateway_resp {
            FiservPaymentsResponse::Charges(res) => {
                &res.gateway_response.transaction_processing_details.order_id
            }
            FiservPaymentsResponse::Checkout(res) => {
                &res.gateway_response.transaction_processing_details.order_id
            }
        };

        let transaction_id = match gateway_resp {
            FiservPaymentsResponse::Charges(res) => {
                &res.gateway_response
                    .transaction_processing_details
                    .transaction_id
            }
            FiservPaymentsResponse::Checkout(res) => {
                &res.gateway_response
                    .transaction_processing_details
                    .transaction_id
            }
        };

        let transaction_state = match gateway_resp {
            FiservPaymentsResponse::Charges(res) => &res.gateway_response.transaction_state,
            FiservPaymentsResponse::Checkout(res) => &res.gateway_response.transaction_state,
        };

        Ok(Self {
            status: enums::AttemptStatus::from(transaction_state.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(transaction_id.to_string()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(connector_response_reference_id.to_string()),
                incremental_authorization_allowed: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservCaptureRequest {
    amount: Amount,
    transaction_details: TransactionDetails,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    order: Option<FiservOrderRequest>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservOrderRequest {
    order_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTransactionDetails {
    reference_transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiservSessionObject {
    pub terminal_id: Secret<String>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for FiservSessionObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

impl TryFrom<&FiservRouterData<&types::PaymentsCaptureRouterData>> for FiservCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FiservRouterData<&types::PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.router_data.connector_auth_type)?;
        let metadata = item
            .router_data
            .connector_meta_data
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let session: FiservSessionObject = metadata
            .expose()
            .parse_value("FiservSessionObject")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "Merchant connector account metadata",
            })?;

        let order_id = item
            .router_data
            .request
            .connector_meta
            .as_ref()
            .and_then(|v| v.get("order_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Self {
            amount: Amount {
                total: item.amount,
                currency: item.router_data.request.currency.to_string(),
            },
            order: Some(FiservOrderRequest { order_id }),
            transaction_details: TransactionDetails {
                capture_flag: Some(true),
                reversal_reason_code: None,
                merchant_transaction_id: Some(
                    item.router_data.connector_request_reference_id.clone(),
                ),
                operation_type: Some(OperationType::Capture),
            },
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .router_data
                    .request
                    .connector_transaction_id
                    .to_string(),
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservSyncRequest {
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl TryFrom<&types::PaymentsSyncRouterData> for FiservSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: None,
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .request
                    .get_connector_transaction_id()
                    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?,
            },
        })
    }
}

impl TryFrom<&types::RefundSyncRouterData> for FiservSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: None,
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .request
                    .connector_refund_id
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?,
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiservRefundRequest {
    amount: Amount,
    merchant_details: MerchantDetails,
    reference_transaction_details: ReferenceTransactionDetails,
}

impl<F> TryFrom<&FiservRouterData<&types::RefundsRouterData<F>>> for FiservRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &FiservRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let auth: FiservAuthType = FiservAuthType::try_from(&item.router_data.connector_auth_type)?;
        let metadata = item
            .router_data
            .connector_meta_data
            .clone()
            .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
        let session: FiservSessionObject = metadata
            .expose()
            .parse_value("FiservSessionObject")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "Merchant connector account metadata",
            })?;
        Ok(Self {
            amount: Amount {
                total: item.amount,
                currency: item.router_data.request.currency.to_string(),
            },
            merchant_details: MerchantDetails {
                merchant_id: auth.merchant_account,
                terminal_id: Some(session.terminal_id),
            },
            reference_transaction_details: ReferenceTransactionDetails {
                reference_transaction_id: item
                    .router_data
                    .request
                    .connector_transaction_id
                    .to_string(),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    pub gateway_response: GatewayResponse,
    pub payment_receipt: PaymentReceipt,
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
                connector_refund_id: item
                    .response
                    .gateway_response
                    .transaction_processing_details
                    .transaction_id,
                refund_status: enums::RefundStatus::from(
                    item.response.gateway_response.transaction_state,
                ),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, FiservSyncResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, FiservSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let gateway_resp = item
            .response
            .sync_responses
            .first()
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;
        let transaction_id = match gateway_resp {
            FiservPaymentsResponse::Charges(res) => {
                &res.gateway_response
                    .transaction_processing_details
                    .transaction_id
            }
            FiservPaymentsResponse::Checkout(res) => {
                &res.gateway_response
                    .transaction_processing_details
                    .transaction_id
            }
        };

        let transaction_state = match gateway_resp {
            FiservPaymentsResponse::Charges(res) => &res.gateway_response.transaction_state,
            FiservPaymentsResponse::Checkout(res) => &res.gateway_response.transaction_state,
        };
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: transaction_id.clone(),
                refund_status: enums::RefundStatus::from(transaction_state.clone()),
            }),
            ..item.data
        })
    }
}
