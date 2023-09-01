use api_models::{enums as api_enums, payments};
use base64::Engine;
use common_utils::{
    errors::CustomResult,
    ext_traits::{ByteSliceExt, StringExt, ValueExt},
    pii::Email,
};
use error_stack::{IntoReport, ResultExt};
use masking::ExposeInterface;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self, AddressDetailsData, ApplePay, CardData, PaymentsAuthorizeRequestData,
        PaymentsCompleteAuthorizeRequestData, RouterData,
    },
    consts,
    core::errors,
    pii::Secret,
    types::{self, api, storage::enums, transformers::ForeignTryFrom},
    utils::{Encode, OptionExt},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsRequest {
    amount: String,
    #[serde(flatten)]
    payment_method: PaymentMethodDetails,
    currency: enums::Currency,
    card_transaction_type: BluesnapTxnType,
    three_d_secure: Option<BluesnapThreeDSecureInfo>,
    transaction_fraud_info: Option<TransactionFraudInfo>,
    card_holder_info: Option<BluesnapCardHolderInfo>,
    merchant_transaction_id: Option<String>,
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
#[serde(rename_all = "camelCase")]
pub struct BluesnapApplePayObject {
    token: api_models::payments::ApplePayWalletData,
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
    country_code: Option<api_enums::CountryAlpha2>,
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
    pub merchant_id: String,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BluesnapPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth_mode = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => BluesnapTxnType::AuthOnly,
            _ => BluesnapTxnType::AuthCapture,
        };
        let (payment_method, card_holder_info) = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ref ccard) => Ok((
                PaymentMethodDetails::CreditCard(Card {
                    card_number: ccard.card_number.clone(),
                    expiration_month: ccard.card_exp_month.clone(),
                    expiration_year: ccard.get_expiry_year_4_digit(),
                    security_code: ccard.card_cvc.clone(),
                }),
                get_card_holder_info(item.get_billing_address()?, item.request.get_email()?)?,
            )),
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                api_models::payments::WalletData::GooglePay(payment_method_data) => {
                    let gpay_object = Encode::<BluesnapGooglePayObject>::encode_to_string_of_json(
                        &BluesnapGooglePayObject {
                            payment_method_data: utils::GooglePayWalletData::from(
                                payment_method_data,
                            ),
                        },
                    )
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok((
                        PaymentMethodDetails::Wallet(BluesnapWallet {
                            wallet_type: BluesnapWalletTypes::GooglePay,
                            encoded_payment_token: Secret::new(
                                consts::BASE64_ENGINE.encode(gpay_object),
                            ),
                        }),
                        None,
                    ))
                }
                api_models::payments::WalletData::ApplePay(payment_method_data) => {
                    let apple_pay_payment_data = payment_method_data
                        .get_applepay_decoded_payment_data()
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    let apple_pay_payment_data: ApplePayEncodedPaymentData = apple_pay_payment_data
                        .expose()[..]
                        .as_bytes()
                        .parse_struct("ApplePayEncodedPaymentData")
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                    let billing = item
                        .address
                        .billing
                        .to_owned()
                        .get_required_value("billing")
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "billing",
                        })?;

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

                    let apple_pay_object = Encode::<EncodedPaymentToken>::encode_to_string_of_json(
                        &EncodedPaymentToken {
                            token: ApplepayPaymentData {
                                payment_data: apple_pay_payment_data,
                                payment_method: payment_method_data
                                    .payment_method
                                    .to_owned()
                                    .into(),
                                transaction_identifier: payment_method_data.transaction_identifier,
                            },
                            billing_contact: BillingDetails {
                                country_code: billing_address.country,
                                address_lines: Some(address),
                                family_name: billing_address.last_name.to_owned(),
                                given_name: billing_address.first_name.to_owned(),
                                postal_code: billing_address.zip,
                            },
                        },
                    )
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                    Ok((
                        PaymentMethodDetails::Wallet(BluesnapWallet {
                            wallet_type: BluesnapWalletTypes::ApplePay,
                            encoded_payment_token: Secret::new(
                                consts::BASE64_ENGINE.encode(apple_pay_object),
                            ),
                        }),
                        get_card_holder_info(
                            item.get_billing_address()?,
                            item.request.get_email()?,
                        )?,
                    ))
                }
                payments::WalletData::AliPayQr(_)
                | payments::WalletData::AliPayRedirect(_)
                | payments::WalletData::AliPayHkRedirect(_)
                | payments::WalletData::MomoRedirect(_)
                | payments::WalletData::KakaoPayRedirect(_)
                | payments::WalletData::GoPayRedirect(_)
                | payments::WalletData::GcashRedirect(_)
                | payments::WalletData::ApplePayRedirect(_)
                | payments::WalletData::ApplePayThirdPartySdk(_)
                | payments::WalletData::DanaRedirect {}
                | payments::WalletData::GooglePayRedirect(_)
                | payments::WalletData::GooglePayThirdPartySdk(_)
                | payments::WalletData::MbWayRedirect(_)
                | payments::WalletData::MobilePayRedirect(_)
                | payments::WalletData::PaypalRedirect(_)
                | payments::WalletData::PaypalSdk(_)
                | payments::WalletData::SamsungPay(_)
                | payments::WalletData::TwintRedirect {}
                | payments::WalletData::VippsRedirect {}
                | payments::WalletData::TouchNGoRedirect(_)
                | payments::WalletData::WeChatPayRedirect(_)
                | payments::WalletData::CashappQr(_)
                | payments::WalletData::SwishQr(_)
                | payments::WalletData::WeChatPayQr(_) => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("bluesnap"),
                    ))
                }
            },
            payments::PaymentMethodData::PayLater(_)
            | payments::PaymentMethodData::BankRedirect(_)
            | payments::PaymentMethodData::BankDebit(_)
            | payments::PaymentMethodData::BankTransfer(_)
            | payments::PaymentMethodData::Crypto(_)
            | payments::PaymentMethodData::MandatePayment
            | payments::PaymentMethodData::Reward
            | payments::PaymentMethodData::Upi(_)
            | payments::PaymentMethodData::CardRedirect(_)
            | payments::PaymentMethodData::Voucher(_)
            | payments::PaymentMethodData::GiftCard(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("bluesnap"),
                ))
            }
        }?;
        Ok(Self {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            payment_method,
            currency: item.request.currency,
            card_transaction_type: auth_mode,
            three_d_secure: None,
            transaction_fraud_info: Some(TransactionFraudInfo {
                fraud_session_id: item.payment_id.clone(),
            }),
            card_holder_info,
            merchant_transaction_id: Some(item.connector_request_reference_id.clone()),
        })
    }
}

impl From<api_models::payments::ApplepayPaymentMethod> for ApplepayPaymentMethod {
    fn from(item: api_models::payments::ApplepayPaymentMethod) -> Self {
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
            .parse_value::<api_models::payments::ApplepaySessionTokenData>(
                "ApplepaySessionTokenData",
            )
            .change_context(errors::ConnectorError::ParsingFailed)?;
        Ok(Self {
            wallet_type: "APPLE_PAY".to_string(),
            validation_url: consts::APPLEPAY_VALIDATION_URL.to_string().into(),
            domain_name: applepay_metadata.data.session_token_data.initiative_context,
            display_name: Some(applepay_metadata.data.session_token_data.display_name),
        })
    }
}

impl TryFrom<types::PaymentsSessionResponseRouterData<BluesnapWalletTokenResponse>>
    for types::PaymentsSessionRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsSessionResponseRouterData<BluesnapWalletTokenResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;

        let wallet_token = consts::BASE64_ENGINE
            .decode(response.wallet_token.clone().expose())
            .into_report()
            .change_context(errors::ConnectorError::ResponseHandlingFailed)?;

        let session_response: api_models::payments::NoThirdPartySdkSessionResponse =
            wallet_token[..]
                .parse_struct("NoThirdPartySdkSessionResponse")
                .change_context(errors::ConnectorError::ParsingFailed)?;

        let metadata = item.data.get_connector_meta()?.expose();
        let applepay_metadata = metadata
            .parse_value::<api_models::payments::ApplepaySessionTokenData>(
                "ApplepaySessionTokenData",
            )
            .change_context(errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            response: Ok(types::PaymentsResponseData::SessionResponse {
                session_token: types::api::SessionToken::ApplePay(Box::new(
                    api_models::payments::ApplepaySessionTokenResponse {
                        session_token_data:
                            api_models::payments::ApplePaySessionResponse::NoThirdPartySdk(
                                session_response,
                            ),
                        payment_request_data: Some(api_models::payments::ApplePayPaymentRequest {
                            country_code: item.data.get_billing_country()?,
                            currency_code: item.data.request.currency,
                            total: api_models::payments::AmountInfo {
                                label: applepay_metadata.data.payment_request_data.label,
                                total_type: Some("final".to_string()),
                                amount: item.data.request.amount.to_string(),
                            },
                            merchant_capabilities: Some(
                                applepay_metadata
                                    .data
                                    .payment_request_data
                                    .merchant_capabilities,
                            ),
                            supported_networks: Some(
                                applepay_metadata
                                    .data
                                    .payment_request_data
                                    .supported_networks,
                            ),
                            merchant_identifier: Some(
                                applepay_metadata
                                    .data
                                    .session_token_data
                                    .merchant_identifier,
                            ),
                        }),
                        connector: "bluesnap".to_string(),
                        delayed_session_token: false,
                        sdk_next_action: {
                            payments::SdkNextAction {
                                next_action: payments::NextActionCall::Confirm,
                            }
                        },
                        connector_reference_id: None,
                        connector_sdk_public_key: None,
                    },
                )),
            }),
            ..item.data
        })
    }
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for BluesnapPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let redirection_response: BluesnapRedirectionResponse = item
            .request
            .redirect_response
            .as_ref()
            .and_then(|res| res.payload.to_owned())
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.redirect_response.payload",
            })?
            .parse_value("BluesnapRedirectionResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let redirection_result: BluesnapThreeDsResult = redirection_response
            .authentication_response
            .parse_struct("BluesnapThreeDsResult")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let auth_mode = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => BluesnapTxnType::AuthOnly,
            _ => BluesnapTxnType::AuthCapture,
        };
        let payment_method = if let Some(api::PaymentMethodData::Card(ccard)) =
            item.request.payment_method_data.clone()
        {
            PaymentMethodDetails::CreditCard(Card {
                card_number: ccard.card_number.clone(),
                expiration_month: ccard.card_exp_month.clone(),
                expiration_year: ccard.get_expiry_year_4_digit(),
                security_code: ccard.card_cvc,
            })
        } else {
            Err(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.payment_method_data",
            })?
        };
        Ok(Self {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            payment_method,
            currency: item.request.currency,
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
                fraud_session_id: item.payment_id.clone(),
            }),
            card_holder_info: get_card_holder_info(
                item.get_billing_address()?,
                item.request.get_email()?,
            )?,
            merchant_transaction_id: Some(item.connector_request_reference_id.clone()),
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
    amount: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for BluesnapCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let card_transaction_type = BluesnapTxnType::Capture;
        let transaction_id = item.request.connector_transaction_id.to_string();
        let amount =
            utils::to_currency_base_unit(item.request.amount_to_capture, item.request.currency)?;
        Ok(Self {
            card_transaction_type,
            transaction_id,
            amount: Some(amount),
        })
    }
}

// Auth Struct
pub struct BluesnapAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) key1: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for BluesnapAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
                key1: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCustomerRequest {
    email: Option<Email>,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for BluesnapCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: item.request.email.to_owned(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCustomerResponse {
    vaulted_shopper_id: Secret<u64>,
}
impl<F, T>
    TryFrom<types::ResponseRouterData<F, BluesnapCustomerResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BluesnapCustomerResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::PaymentsResponseData::ConnectorCustomerResponse {
                connector_customer_id: item.response.vaulted_shopper_id.expose().to_string(),
            }),
            ..item.data
        })
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
    amount: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInfoResponse {
    pub processing_status: BluesnapProcessingStatus,
    pub authorization_code: Option<String>,
    pub network_transaction_id: Option<Secret<String>>,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, BluesnapPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            BluesnapPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::foreign_try_from((
                item.response.card_transaction_type,
                item.response.processing_info.processing_status,
            ))?,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct BluesnapRefundRequest {
    amount: Option<String>,
    reason: Option<String>,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BluesnapRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            reason: item.request.reason.clone(),
            amount: Some(utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?),
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    refund_transaction_id: i32,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, BluesnapPaymentsResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, BluesnapPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.transaction_id.clone(),
                refund_status: enums::RefundStatus::from(
                    item.response.processing_info.processing_status,
                ),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.refund_transaction_id.to_string(),
                refund_status: enums::RefundStatus::Pending,
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectEventType {
    pub transaction_type: BluesnapWebhookEvents,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapWebhookEvents {
    Decline,
    CcChargeFailed,
    Charge,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectResource {
    pub reference_number: String,
    pub transaction_type: BluesnapWebhookEvents,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub code: String,
    pub description: String,
    pub error_name: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapErrorResponse {
    pub message: Vec<ErrorDetails>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapAuthErrorResponse {
    pub error_code: String,
    pub error_description: String,
    pub error_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BluesnapErrors {
    Payment(BluesnapErrorResponse),
    Auth(BluesnapAuthErrorResponse),
    General(String),
}

fn get_card_holder_info(
    address: &api::AddressDetails,
    email: Email,
) -> CustomResult<Option<BluesnapCardHolderInfo>, errors::ConnectorError> {
    Ok(Some(BluesnapCardHolderInfo {
        first_name: address.get_first_name()?.clone(),
        last_name: address.get_last_name()?.clone(),
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
