use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use url::Url;
use uuid::Uuid;

use crate::{
    connector::utils::{self, CardData},
    core::errors,
    pii::Secret,
    services,
    types::{self, api, storage::enums, PaymentsSyncData},
};

pub struct AirwallexAuthType {
    pub x_api_key: Secret<String>,
    pub x_client_id: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for AirwallexAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            Ok(Self {
                x_api_key: api_key.clone(),
                x_client_id: key1.clone(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexIntentRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: String,
    currency: enums::Currency,
    //ID created in merchant's order system that corresponds to this PaymentIntent.
    merchant_order_id: String,
}
impl TryFrom<&types::PaymentsInitRouterData> for AirwallexIntentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsInitRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            merchant_order_id: item.connector_request_reference_id.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AirwallexRouterData<T> {
    pub amount: String,
    pub router_data: T,
}

impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for AirwallexRouterData<T>
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
pub struct AirwallexPaymentsRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    payment_method: AirwallexPaymentMethod,
    payment_method_options: Option<AirwallexPaymentOptions>,
    return_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum AirwallexPaymentMethod {
    Card(AirwallexCard),
    Wallets(WalletData),
}

#[derive(Debug, Serialize)]
pub struct AirwallexCard {
    card: AirwallexCardDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}
#[derive(Debug, Serialize)]
pub struct AirwallexCardDetails {
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    number: cards::CardNumber,
    cvc: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum WalletData {
    GooglePay(GooglePayData),
}

#[derive(Debug, Serialize)]
pub struct GooglePayData {
    googlepay: GooglePayDetails,
    #[serde(rename = "type")]
    payment_method_type: AirwallexPaymentType,
}

#[derive(Debug, Serialize)]
pub struct GooglePayDetails {
    encrypted_payment_token: Secret<String>,
    payment_data_type: GpayPaymentDataType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentType {
    Card,
    Googlepay,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GpayPaymentDataType {
    EncryptedPaymentToken,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AirwallexPaymentOptions {
    Card(AirwallexCardPaymentOptions),
}
#[derive(Debug, Serialize)]
pub struct AirwallexCardPaymentOptions {
    auto_capture: bool,
}

impl TryFrom<&AirwallexRouterData<&types::PaymentsAuthorizeRouterData>>
    for AirwallexPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AirwallexRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let mut payment_method_options = None;
        let request = &item.router_data.request;
        let payment_method = match request.payment_method_data.clone() {
            api::PaymentMethodData::Card(ccard) => {
                payment_method_options =
                    Some(AirwallexPaymentOptions::Card(AirwallexCardPaymentOptions {
                        auto_capture: matches!(
                            request.capture_method,
                            Some(enums::CaptureMethod::Automatic) | None
                        ),
                    }));
                Ok(AirwallexPaymentMethod::Card(AirwallexCard {
                    card: AirwallexCardDetails {
                        number: ccard.card_number.clone(),
                        expiry_month: ccard.card_exp_month.clone(),
                        expiry_year: ccard.get_expiry_year_4_digit(),
                        cvc: ccard.card_cvc,
                    },
                    payment_method_type: AirwallexPaymentType::Card,
                }))
            }
            api::PaymentMethodData::Wallet(ref wallet_data) => get_wallet_details(wallet_data),
            api::PaymentMethodData::PayLater(_)
            | api::PaymentMethodData::BankRedirect(_)
            | api::PaymentMethodData::BankDebit(_)
            | api::PaymentMethodData::BankTransfer(_)
            | api::PaymentMethodData::CardRedirect(_)
            | api::PaymentMethodData::Crypto(_)
            | api::PaymentMethodData::MandatePayment
            | api::PaymentMethodData::Reward
            | api::PaymentMethodData::Upi(_)
            | api::PaymentMethodData::Voucher(_)
            | api::PaymentMethodData::GiftCard(_) => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("airwallex"),
            )),
        }?;

        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            payment_method,
            payment_method_options,
            return_url: request.complete_authorize_url.clone(),
        })
    }
}

fn get_wallet_details(
    wallet_data: &api_models::payments::WalletData,
) -> Result<AirwallexPaymentMethod, errors::ConnectorError> {
    let wallet_details: AirwallexPaymentMethod = match wallet_data {
        api_models::payments::WalletData::GooglePay(gpay_details) => {
            AirwallexPaymentMethod::Wallets(WalletData::GooglePay(GooglePayData {
                googlepay: GooglePayDetails {
                    encrypted_payment_token: Secret::new(
                        gpay_details.tokenization_data.token.clone(),
                    ),
                    payment_data_type: GpayPaymentDataType::EncryptedPaymentToken,
                },
                payment_method_type: AirwallexPaymentType::Googlepay,
            }))
        }
        api_models::payments::WalletData::AliPayQr(_)
        | api_models::payments::WalletData::AliPayRedirect(_)
        | api_models::payments::WalletData::AliPayHkRedirect(_)
        | api_models::payments::WalletData::MomoRedirect(_)
        | api_models::payments::WalletData::KakaoPayRedirect(_)
        | api_models::payments::WalletData::GoPayRedirect(_)
        | api_models::payments::WalletData::GcashRedirect(_)
        | api_models::payments::WalletData::ApplePay(_)
        | api_models::payments::WalletData::ApplePayRedirect(_)
        | api_models::payments::WalletData::ApplePayThirdPartySdk(_)
        | api_models::payments::WalletData::DanaRedirect {}
        | api_models::payments::WalletData::GooglePayRedirect(_)
        | api_models::payments::WalletData::GooglePayThirdPartySdk(_)
        | api_models::payments::WalletData::MbWayRedirect(_)
        | api_models::payments::WalletData::MobilePayRedirect(_)
        | api_models::payments::WalletData::PaypalRedirect(_)
        | api_models::payments::WalletData::PaypalSdk(_)
        | api_models::payments::WalletData::SamsungPay(_)
        | api_models::payments::WalletData::TwintRedirect {}
        | api_models::payments::WalletData::VippsRedirect {}
        | api_models::payments::WalletData::TouchNGoRedirect(_)
        | api_models::payments::WalletData::WeChatPayRedirect(_)
        | api_models::payments::WalletData::WeChatPayQr(_)
        | api_models::payments::WalletData::CashappQr(_)
        | api_models::payments::WalletData::SwishQr(_) => {
            Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("airwallex"),
            ))?
        }
    };
    Ok(wallet_details)
}

#[derive(Deserialize)]
pub struct AirwallexAuthUpdateResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    expires_at: PrimitiveDateTime,
    token: Secret<String>,
}

impl<F, T> TryFrom<types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>>
    for types::RouterData<F, T, types::AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, AirwallexAuthUpdateResponse, T, types::AccessToken>,
    ) -> Result<Self, Self::Error> {
        let expires = (item.response.expires_at - common_utils::date_time::now()).whole_seconds();
        Ok(Self {
            response: Ok(types::AccessToken {
                token: item.response.token,
                expires,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexCompleteRequest {
    request_id: String,
    three_ds: AirwallexThreeDsData,
    #[serde(rename = "type")]
    three_ds_type: AirwallexThreeDsType,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexThreeDsData {
    acs_response: Option<Secret<String>>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub enum AirwallexThreeDsType {
    #[default]
    #[serde(rename = "3ds_continue")]
    ThreeDSContinue,
}

impl TryFrom<&types::PaymentsCompleteAuthorizeRouterData> for AirwallexCompleteRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            three_ds: AirwallexThreeDsData {
                acs_response: item
                    .request
                    .redirect_response
                    .as_ref()
                    .map(|f| f.payload.to_owned())
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "redirect_response.payload",
                    })?
                    .as_ref()
                    .map(|data| serde_json::to_string(data.peek()))
                    .transpose()
                    .into_report()
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
                    .map(Secret::new),
            },
            three_ds_type: AirwallexThreeDsType::ThreeDSContinue,
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCaptureRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<String>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for AirwallexPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?),
        })
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AirwallexPaymentsCancelRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    cancellation_reason: Option<String>,
}

impl TryFrom<&types::PaymentsCancelRouterData> for AirwallexPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            cancellation_reason: item.request.cancellation_reason.clone(),
        })
    }
}

// PaymentsResponse
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Pending,
    RequiresPaymentMethod,
    RequiresCustomerAction,
    RequiresCapture,
    Cancelled,
}

fn get_payment_status(
    status: &AirwallexPaymentStatus,
    next_action: &Option<AirwallexPaymentsNextAction>,
) -> enums::AttemptStatus {
    match status.clone() {
        AirwallexPaymentStatus::Succeeded => enums::AttemptStatus::Charged,
        AirwallexPaymentStatus::Failed => enums::AttemptStatus::Failure,
        AirwallexPaymentStatus::Pending => enums::AttemptStatus::Pending,
        AirwallexPaymentStatus::RequiresPaymentMethod => enums::AttemptStatus::PaymentMethodAwaited,
        AirwallexPaymentStatus::RequiresCustomerAction => next_action.as_ref().map_or(
            enums::AttemptStatus::AuthenticationPending,
            |next_action| match next_action.stage {
                AirwallexNextActionStage::WaitingDeviceDataCollection => {
                    enums::AttemptStatus::DeviceDataCollectionPending
                }
                AirwallexNextActionStage::WaitingUserInfoInput => {
                    enums::AttemptStatus::AuthenticationPending
                }
            },
        ),
        AirwallexPaymentStatus::RequiresCapture => enums::AttemptStatus::Authorized,
        AirwallexPaymentStatus::Cancelled => enums::AttemptStatus::Voided,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexNextActionStage {
    WaitingDeviceDataCollection,
    WaitingUserInfoInput,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AirwallexRedirectFormData {
    #[serde(rename = "JWT")]
    jwt: Option<String>,
    #[serde(rename = "threeDSMethodData")]
    three_ds_method_data: Option<String>,
    token: Option<String>,
    provider: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AirwallexPaymentsNextAction {
    url: Url,
    method: services::Method,
    data: AirwallexRedirectFormData,
    stage: AirwallexNextActionStage,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct AirwallexPaymentsResponse {
    status: AirwallexPaymentStatus,
    //Unique identifier for the PaymentIntent
    id: String,
    amount: Option<f32>,
    //ID of the PaymentConsent related to this PaymentIntent
    payment_consent_id: Option<String>,
    next_action: Option<AirwallexPaymentsNextAction>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct AirwallexPaymentsSyncResponse {
    status: AirwallexPaymentStatus,
    //Unique identifier for the PaymentIntent
    id: String,
    amount: Option<f32>,
    //ID of the PaymentConsent related to this PaymentIntent
    payment_consent_id: Option<String>,
    next_action: Option<AirwallexPaymentsNextAction>,
}

fn get_redirection_form(
    response_url_data: AirwallexPaymentsNextAction,
) -> Option<services::RedirectForm> {
    Some(services::RedirectForm::Form {
        endpoint: response_url_data.url.to_string(),
        method: response_url_data.method,
        form_fields: std::collections::HashMap::from([
            //Some form fields might be empty based on the authentication type by the connector
            (
                "JWT".to_string(),
                response_url_data.data.jwt.unwrap_or_default(),
            ),
            (
                "threeDSMethodData".to_string(),
                response_url_data
                    .data
                    .three_ds_method_data
                    .unwrap_or_default(),
            ),
            (
                "token".to_string(),
                response_url_data.data.token.unwrap_or_default(),
            ),
            (
                "provider".to_string(),
                response_url_data.data.provider.unwrap_or_default(),
            ),
            (
                "version".to_string(),
                response_url_data.data.version.unwrap_or_default(),
            ),
        ]),
    })
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, AirwallexPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            AirwallexPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let (status, redirection_data) = item.response.next_action.clone().map_or(
            // If no next action is there, map the status and set redirection form as None
            (
                get_payment_status(&item.response.status, &item.response.next_action),
                None,
            ),
            |response_url_data| {
                // If the connector sends a customer action response that is already under
                // process from our end it can cause an infinite loop to break this this check
                // is added and fail the payment
                if matches!(
                    (
                        response_url_data.stage.clone(),
                        item.data.status,
                        item.response.status.clone(),
                    ),
                    // If the connector sends waiting for DDC and our status is already DDC Pending
                    // that means we initiated the call to collect the data and now we expect a different response
                    (
                        AirwallexNextActionStage::WaitingDeviceDataCollection,
                        enums::AttemptStatus::DeviceDataCollectionPending,
                        _
                    )
                    // If the connector sends waiting for Customer Action and our status is already Authenticaition Pending
                    // that means we initiated the call to authenticate and now we do not expect a requires_customer action
                    // it will start a loop
                    | (
                        _,
                        enums::AttemptStatus::AuthenticationPending,
                        AirwallexPaymentStatus::RequiresCustomerAction,
                    )
                ) {
                    // Fail the payment for above conditions
                    (enums::AttemptStatus::AuthenticationFailed, None)
                } else {
                    (
                        //Build the redirect form and update the payment status
                        get_payment_status(&item.response.status, &item.response.next_action),
                        get_redirection_form(response_url_data),
                    )
                }
            },
        );
        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}

impl
    TryFrom<
        types::ResponseRouterData<
            api::PSync,
            AirwallexPaymentsSyncResponse,
            PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    > for types::PaymentsSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<
            api::PSync,
            AirwallexPaymentsSyncResponse,
            PaymentsSyncData,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = get_payment_status(&item.response.status, &item.response.next_action);
        let redirection_data = if let Some(redirect_url_data) = item.response.next_action {
            get_redirection_form(redirect_url_data)
        } else {
            None
        };
        Ok(Self {
            status,
            reference_id: Some(item.response.id.clone()),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct AirwallexRefundRequest {
    // Unique ID to be sent for each transaction/operation request to the connector
    request_id: String,
    amount: Option<String>,
    reason: Option<String>,
    //Identifier for the PaymentIntent for which Refund is requested
    payment_intent_id: String,
}

impl<F> TryFrom<&AirwallexRouterData<&types::RefundsRouterData<F>>> for AirwallexRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &AirwallexRouterData<&types::RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            request_id: Uuid::new_v4().to_string(),
            amount: Some(item.amount.to_owned()),
            reason: item.router_data.request.reason.clone(),
            payment_intent_id: item.router_data.request.connector_transaction_id.clone(),
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Received,
    Accepted,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Received | RefundStatus::Accepted => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    //A unique number that tags a credit or debit card transaction when it goes from the merchant's bank through to the cardholder's bank.
    acquirer_reference_number: Option<String>,
    amount: f32,
    //Unique identifier for the Refund
    id: String,
    status: RefundStatus,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AirwallexWebhookData {
    pub source_id: Option<String>,
    pub name: AirwallexWebhookEventType,
    pub data: AirwallexObjectData,
}

#[derive(Debug, Deserialize, strum::Display, PartialEq)]
pub enum AirwallexWebhookEventType {
    #[serde(rename = "payment_intent.created")]
    PaymentIntentCreated,
    #[serde(rename = "payment_intent.requires_payment_method")]
    PaymentIntentRequiresPaymentMethod,
    #[serde(rename = "payment_intent.cancelled")]
    PaymentIntentCancelled,
    #[serde(rename = "payment_intent.succeeded")]
    PaymentIntentSucceeded,
    #[serde(rename = "payment_intent.requires_capture")]
    PaymentIntentRequiresCapture,
    #[serde(rename = "payment_intent.requires_customer_action")]
    PaymentIntentRequiresCustomerAction,
    #[serde(rename = "payment_attempt.authorized")]
    PaymentAttemptAuthorized,
    #[serde(rename = "payment_attempt.authorization_failed")]
    PaymentAttemptAuthorizationFailed,
    #[serde(rename = "payment_attempt.capture_requested")]
    PaymentAttemptCaptureRequested,
    #[serde(rename = "payment_attempt.capture_failed")]
    PaymentAttemptCaptureFailed,
    #[serde(rename = "payment_attempt.authentication_redirected")]
    PaymentAttemptAuthenticationRedirected,
    #[serde(rename = "payment_attempt.authentication_failed")]
    PaymentAttemptAuthenticationFailed,
    #[serde(rename = "payment_attempt.failed_to_process")]
    PaymentAttemptFailedToProcess,
    #[serde(rename = "payment_attempt.cancelled")]
    PaymentAttemptCancelled,
    #[serde(rename = "payment_attempt.expired")]
    PaymentAttemptExpired,
    #[serde(rename = "payment_attempt.risk_declined")]
    PaymentAttemptRiskDeclined,
    #[serde(rename = "payment_attempt.settled")]
    PaymentAttemptSettled,
    #[serde(rename = "payment_attempt.paid")]
    PaymentAttemptPaid,
    #[serde(rename = "refund.received")]
    RefundReceived,
    #[serde(rename = "refund.accepted")]
    RefundAccepted,
    #[serde(rename = "refund.succeeded")]
    RefundSucceeded,
    #[serde(rename = "refund.failed")]
    RefundFailed,
    #[serde(rename = "dispute.rfi_responded_by_merchant")]
    DisputeRfiRespondedByMerchant,
    #[serde(rename = "dispute.dispute.pre_chargeback_accepted")]
    DisputePreChargebackAccepted,
    #[serde(rename = "dispute.accepted")]
    DisputeAccepted,
    #[serde(rename = "dispute.dispute_received_by_merchant")]
    DisputeReceivedByMerchant,
    #[serde(rename = "dispute.dispute_responded_by_merchant")]
    DisputeRespondedByMerchant,
    #[serde(rename = "dispute.won")]
    DisputeWon,
    #[serde(rename = "dispute.lost")]
    DisputeLost,
    #[serde(rename = "dispute.dispute_reversed")]
    DisputeReversed,
    #[serde(other)]
    Unknown,
}

pub fn is_transaction_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::PaymentAttemptFailedToProcess
            | AirwallexWebhookEventType::PaymentAttemptAuthorized
    )
}

pub fn is_refund_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::RefundSucceeded | AirwallexWebhookEventType::RefundFailed
    )
}

pub fn is_dispute_event(event_code: &AirwallexWebhookEventType) -> bool {
    matches!(
        event_code,
        AirwallexWebhookEventType::DisputeAccepted
            | AirwallexWebhookEventType::DisputePreChargebackAccepted
            | AirwallexWebhookEventType::DisputeRespondedByMerchant
            | AirwallexWebhookEventType::DisputeWon
            | AirwallexWebhookEventType::DisputeLost
    )
}

#[derive(Debug, Deserialize)]
pub struct AirwallexObjectData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexDisputeObject {
    pub payment_intent_id: String,
    pub dispute_amount: i64,
    pub dispute_currency: String,
    pub stage: AirwallexDisputeStage,
    pub dispute_id: String,
    pub dispute_reason_type: Option<String>,
    pub dispute_original_reason_code: Option<String>,
    pub status: String,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize, strum::Display, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AirwallexDisputeStage {
    Rfi,
    Dispute,
    Arbitration,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookDataResource {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AirwallexWebhookObjectResource {
    pub data: AirwallexWebhookDataResource,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct AirwallexErrorResponse {
    pub code: String,
    pub message: String,
    pub source: Option<String>,
}

impl TryFrom<AirwallexWebhookEventType> for api_models::webhooks::IncomingWebhookEvent {
    type Error = errors::ConnectorError;
    fn try_from(value: AirwallexWebhookEventType) -> Result<Self, Self::Error> {
        Ok(match value {
            AirwallexWebhookEventType::PaymentAttemptFailedToProcess => Self::PaymentIntentFailure,
            AirwallexWebhookEventType::PaymentAttemptAuthorized => Self::PaymentIntentSuccess,
            AirwallexWebhookEventType::RefundSucceeded => Self::RefundSuccess,
            AirwallexWebhookEventType::RefundFailed => Self::RefundFailure,
            AirwallexWebhookEventType::DisputeAccepted
            | AirwallexWebhookEventType::DisputePreChargebackAccepted => Self::DisputeAccepted,
            AirwallexWebhookEventType::DisputeRespondedByMerchant => Self::DisputeChallenged,
            AirwallexWebhookEventType::DisputeWon | AirwallexWebhookEventType::DisputeReversed => {
                Self::DisputeWon
            }
            AirwallexWebhookEventType::DisputeLost => Self::DisputeLost,
            AirwallexWebhookEventType::Unknown
            | AirwallexWebhookEventType::PaymentIntentCreated
            | AirwallexWebhookEventType::PaymentIntentRequiresPaymentMethod
            | AirwallexWebhookEventType::PaymentIntentCancelled
            | AirwallexWebhookEventType::PaymentIntentSucceeded
            | AirwallexWebhookEventType::PaymentIntentRequiresCapture
            | AirwallexWebhookEventType::PaymentIntentRequiresCustomerAction
            | AirwallexWebhookEventType::PaymentAttemptAuthorizationFailed
            | AirwallexWebhookEventType::PaymentAttemptCaptureRequested
            | AirwallexWebhookEventType::PaymentAttemptCaptureFailed
            | AirwallexWebhookEventType::PaymentAttemptAuthenticationRedirected
            | AirwallexWebhookEventType::PaymentAttemptAuthenticationFailed
            | AirwallexWebhookEventType::PaymentAttemptCancelled
            | AirwallexWebhookEventType::PaymentAttemptExpired
            | AirwallexWebhookEventType::PaymentAttemptRiskDeclined
            | AirwallexWebhookEventType::PaymentAttemptSettled
            | AirwallexWebhookEventType::PaymentAttemptPaid
            | AirwallexWebhookEventType::RefundReceived
            | AirwallexWebhookEventType::RefundAccepted
            | AirwallexWebhookEventType::DisputeRfiRespondedByMerchant
            | AirwallexWebhookEventType::DisputeReceivedByMerchant => Self::EventNotSupported,
        })
    }
}

impl From<AirwallexDisputeStage> for api_models::enums::DisputeStage {
    fn from(code: AirwallexDisputeStage) -> Self {
        match code {
            AirwallexDisputeStage::Rfi => Self::PreDispute,
            AirwallexDisputeStage::Dispute => Self::Dispute,
            AirwallexDisputeStage::Arbitration => Self::PreArbitration,
        }
    }
}
