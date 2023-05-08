use base64::Engine;
use common_utils::{
    ext_traits::{StringExt, ValueExt},
    pii::Email,
};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils,
    consts,
    core::errors,
    pii::Secret,
    types::{self, api, storage::enums, transformers::ForeignTryFrom},
    utils::Encode,
};

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsRequest {
    amount: String,
    #[serde(flatten)]
    payment_method: PaymentMethodDetails,
    currency: enums::Currency,
    card_transaction_type: BluesnapTxnType,
    three_d_secure: Option<BluesnapThreeDSecureInfo>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDSecureInfo {
    three_d_secure_reference_id: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PaymentMethodDetails {
    CreditCard(Card),
    Wallet(BluesnapWallet),
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    card_number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Secret<String>,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWallet {
    wallet_type: BluesnapWalletTypes,
    encoded_payment_token: String,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapGooglePayObject {
    payment_method_data: api_models::payments::GooglePayWalletData,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapApplePayObject {
    token: api_models::payments::ApplePayWalletData,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapWalletTypes {
    GooglePay,
    ApplePay,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BluesnapPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth_mode = match item.request.capture_method {
            Some(enums::CaptureMethod::Manual) => BluesnapTxnType::AuthOnly,
            _ => BluesnapTxnType::AuthCapture,
        };
        let payment_method = match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => Ok(PaymentMethodDetails::CreditCard(Card {
                card_number: ccard.card_number,
                expiration_month: ccard.card_exp_month.clone(),
                expiration_year: ccard.card_exp_year.clone(),
                security_code: ccard.card_cvc,
            })),
            api::PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                api_models::payments::WalletData::GooglePay(payment_method_data) => {
                    let gpay_object = Encode::<BluesnapGooglePayObject>::encode_to_string_of_json(
                        &BluesnapGooglePayObject {
                            payment_method_data,
                        },
                    )
                    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok(PaymentMethodDetails::Wallet(BluesnapWallet {
                        wallet_type: BluesnapWalletTypes::GooglePay,
                        encoded_payment_token: consts::BASE64_ENGINE.encode(gpay_object),
                    }))
                }
                api_models::payments::WalletData::ApplePay(payment_method_data) => {
                    let apple_pay_object =
                        Encode::<BluesnapApplePayObject>::encode_to_string_of_json(
                            &BluesnapApplePayObject {
                                token: payment_method_data,
                            },
                        )
                        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
                    Ok(PaymentMethodDetails::Wallet(BluesnapWallet {
                        wallet_type: BluesnapWalletTypes::ApplePay,
                        encoded_payment_token: consts::BASE64_ENGINE.encode(apple_pay_object),
                    }))
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    "Wallets".to_string(),
                )),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            )),
        }?;
        Ok(Self {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            payment_method,
            currency: item.request.currency,
            card_transaction_type: auth_mode,
            three_d_secure: None,
        })
    }
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for BluesnapPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let redirection_response: BluesnapRedirectionResponse = item
            .request
            .payload
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "request.payload",
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
                card_number: ccard.card_number,
                expiration_month: ccard.card_exp_month.clone(),
                expiration_year: ccard.card_exp_year.clone(),
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
        })
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct BluesnapRedirectionResponse {
    pub authentication_response: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDsResult {
    three_d_secure: Option<BluesnapThreeDsReference>,
    pub status: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapThreeDsReference {
    three_d_secure_reference_id: String,
}

#[derive(Debug, Serialize, PartialEq)]
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

#[derive(Debug, Serialize, PartialEq)]
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
    pub(super) api_key: String,
    pub(super) key1: String,
}

impl TryFrom<&types::ConnectorAuthType> for BluesnapAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
                key1: key1.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
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

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapCustomerResponse {
    vaulted_shopper_id: u64,
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
                connector_customer_id: item.response.vaulted_shopper_id.to_string(),
            }),
            ..item.data
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BluesnapTxnType {
    AuthOnly,
    AuthCapture,
    AuthReversal,
    Capture,
    Refund,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapPaymentsResponse {
    processing_info: ProcessingInfoResponse,
    transaction_id: String,
    card_transaction_type: BluesnapTxnType,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    refund_transaction_id: String,
    amount: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInfoResponse {
    processing_status: BluesnapProcessingStatus,
    authorization_code: Option<String>,
    network_transaction_id: Option<String>,
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
                    item.response.transaction_id,
                ),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Eq, PartialEq, Serialize)]
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
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookBody {
    pub auth_key: String,
    pub contract_id: String,
    pub reference_number: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectEventType {
    pub transaction_type: String,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapWebhookObjectResource {
    pub auth_key: String,
    pub contract_id: String,
    pub reference_number: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorDetails {
    pub code: String,
    pub description: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapErrorResponse {
    pub message: Vec<ErrorDetails>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BluesnapAuthErrorResponse {
    pub error_code: String,
    pub error_description: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BluesnapErrors {
    PaymentError(BluesnapErrorResponse),
    AuthError(BluesnapAuthErrorResponse),
}
