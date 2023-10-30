use common_utils::pii;
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::{
        self as conn_utils, CardData, PaymentsAuthorizeRequestData, RouterData, WalletData,
    },
    core::errors,
    services,
    types::{self, api, storage::enums, transformers::ForeignFrom, ErrorResponse},
    utils,
};

// These needs to be accepted from SDK, need to be done after 1.0.0 stability as API contract will change
const GOOGLEPAY_API_VERSION_MINOR: u8 = 0;
const GOOGLEPAY_API_VERSION: u8 = 2;

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonChannels {
    Web,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonSubscriptionType {
    Unscheduled,
}

#[derive(Debug, Serialize)]
pub struct NoonSubscriptionData {
    #[serde(rename = "type")]
    subscription_type: NoonSubscriptionType,
    //Short description about the subscription.
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonBillingAddress {
    street: Option<Secret<String>>,
    street2: Option<Secret<String>>,
    city: Option<String>,
    state_province: Option<Secret<String>>,
    country: Option<api_models::enums::CountryAlpha2>,
    postal_code: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonBilling {
    address: NoonBillingAddress,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonOrder {
    amount: String,
    currency: Option<diesel_models::enums::Currency>,
    channel: NoonChannels,
    category: Option<String>,
    reference: String,
    //Short description of the order.
    name: String,
    ip_address: Option<Secret<String, pii::IpAddress>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonPaymentActions {
    Authorize,
    Sale,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonConfiguration {
    tokenize_c_c: Option<bool>,
    payment_action: NoonPaymentActions,
    return_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonSubscription {
    subscription_identifier: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonCard {
    name_on_card: Secret<String>,
    number_plain: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonApplePayPaymentMethod {
    pub display_name: String,
    pub network: String,
    #[serde(rename = "type")]
    pub pm_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonApplePayHeader {
    ephemeral_public_key: Secret<String>,
    public_key_hash: Secret<String>,
    transaction_id: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoonApplePaymentData {
    version: Secret<String>,
    data: Secret<String>,
    signature: Secret<String>,
    header: NoonApplePayHeader,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonApplePayData {
    payment_data: NoonApplePaymentData,
    payment_method: NoonApplePayPaymentMethod,
    transaction_identifier: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonApplePayTokenData {
    token: NoonApplePayData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonApplePay {
    payment_info: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonGooglePay {
    api_version_minor: u8,
    api_version: u8,
    payment_method_data: conn_utils::GooglePayWalletData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPayPal {
    return_url: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum NoonPaymentData {
    Card(NoonCard),
    Subscription(NoonSubscription),
    ApplePay(NoonApplePay),
    GooglePay(NoonGooglePay),
    PayPal(NoonPayPal),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NoonApiOperations {
    Initiate,
    Capture,
    Reverse,
    Refund,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsRequest {
    api_operation: NoonApiOperations,
    order: NoonOrder,
    configuration: NoonConfiguration,
    payment_data: NoonPaymentData,
    subscription: Option<NoonSubscriptionData>,
    billing: Option<NoonBilling>,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for NoonPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let (payment_data, currency, category) = match item.request.connector_mandate_id() {
            Some(subscription_identifier) => (
                NoonPaymentData::Subscription(NoonSubscription {
                    subscription_identifier,
                }),
                None,
                None,
            ),
            _ => (
                match item.request.payment_method_data.clone() {
                    api::PaymentMethodData::Card(req_card) => Ok(NoonPaymentData::Card(NoonCard {
                        name_on_card: req_card.card_holder_name.clone(),
                        number_plain: req_card.card_number.clone(),
                        expiry_month: req_card.card_exp_month.clone(),
                        expiry_year: req_card.get_expiry_year_4_digit(),
                        cvv: req_card.card_cvc,
                    })),
                    api::PaymentMethodData::Wallet(wallet_data) => match wallet_data.clone() {
                        api_models::payments::WalletData::GooglePay(google_pay_data) => {
                            Ok(NoonPaymentData::GooglePay(NoonGooglePay {
                                api_version_minor: GOOGLEPAY_API_VERSION_MINOR,
                                api_version: GOOGLEPAY_API_VERSION,
                                payment_method_data: conn_utils::GooglePayWalletData::from(
                                    google_pay_data,
                                ),
                            }))
                        }
                        api_models::payments::WalletData::ApplePay(apple_pay_data) => {
                            let payment_token_data = NoonApplePayTokenData {
                                token: NoonApplePayData {
                                    payment_data: wallet_data.get_wallet_token_as_json()?,
                                    payment_method: NoonApplePayPaymentMethod {
                                        display_name: apple_pay_data.payment_method.display_name,
                                        network: apple_pay_data.payment_method.network,
                                        pm_type: apple_pay_data.payment_method.pm_type,
                                    },
                                    transaction_identifier: Secret::new(
                                        apple_pay_data.transaction_identifier,
                                    ),
                                },
                            };
                            let payment_token =
                                utils::Encode::<NoonApplePayTokenData>::encode_to_string_of_json(
                                    &payment_token_data,
                                )
                                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                            Ok(NoonPaymentData::ApplePay(NoonApplePay {
                                payment_info: Secret::new(payment_token),
                            }))
                        }
                        api_models::payments::WalletData::PaypalRedirect(_) => {
                            Ok(NoonPaymentData::PayPal(NoonPayPal {
                                return_url: item.request.get_router_return_url()?,
                            }))
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
                                message: conn_utils::SELECTED_PAYMENT_METHOD.to_string(),
                                connector: "Noon",
                            })
                        }
                    },
                    api::PaymentMethodData::CardRedirect(_)
                    | api::PaymentMethodData::PayLater(_)
                    | api::PaymentMethodData::BankRedirect(_)
                    | api::PaymentMethodData::BankDebit(_)
                    | api::PaymentMethodData::BankTransfer(_)
                    | api::PaymentMethodData::Crypto(_)
                    | api::PaymentMethodData::MandatePayment {}
                    | api::PaymentMethodData::Reward {}
                    | api::PaymentMethodData::Upi(_)
                    | api::PaymentMethodData::Voucher(_)
                    | api::PaymentMethodData::GiftCard(_) => {
                        Err(errors::ConnectorError::NotSupported {
                            message: conn_utils::SELECTED_PAYMENT_METHOD.to_string(),
                            connector: "Noon",
                        })
                    }
                }?,
                Some(item.request.currency),
                item.request.order_category.clone(),
            ),
        };

        // The description should not have leading or trailing whitespaces, also it should not have double whitespaces and a max 50 chars according to Noon's Docs
        let name: String = item
            .get_description()?
            .trim()
            .replace("  ", " ")
            .chars()
            .take(50)
            .collect();

        let ip_address = item.request.get_ip_address_as_optional();

        let channel = NoonChannels::Web;

        let billing = item
            .address
            .billing
            .clone()
            .and_then(|billing_address| billing_address.address)
            .map(|address| NoonBilling {
                address: NoonBillingAddress {
                    street: address.line1,
                    street2: address.line2,
                    city: address.city,
                    // If state is passed in request, country becomes mandatory, keep a check while debugging failed payments
                    state_province: address.state,
                    country: address.country,
                    postal_code: address.zip,
                },
            });

        let (subscription, tokenize_c_c) =
            match item.request.setup_future_usage.is_some().then_some((
                NoonSubscriptionData {
                    subscription_type: NoonSubscriptionType::Unscheduled,
                    name: name.clone(),
                },
                true,
            )) {
                Some((a, b)) => (Some(a), Some(b)),
                None => (None, None),
            };
        let order = NoonOrder {
            amount: conn_utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency,
            channel,
            category,
            reference: item.connector_request_reference_id.clone(),
            name,
            ip_address,
        };
        let payment_action = if item.request.is_auto_capture()? {
            NoonPaymentActions::Sale
        } else {
            NoonPaymentActions::Authorize
        };
        Ok(Self {
            api_operation: NoonApiOperations::Initiate,
            order,
            billing,
            configuration: NoonConfiguration {
                payment_action,
                return_url: item.request.router_return_url.clone(),
                tokenize_c_c,
            },
            payment_data,
            subscription,
        })
    }
}

// Auth Struct
pub struct NoonAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) application_identifier: Secret<String>,
    pub(super) business_identifier: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for NoonAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_key: api_key.to_owned(),
                application_identifier: api_secret.to_owned(),
                business_identifier: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
#[derive(Default, Debug, Deserialize, Serialize, strum::Display)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum NoonPaymentStatus {
    Initiated,
    Authorized,
    Captured,
    PartiallyCaptured,
    PartiallyRefunded,
    PaymentInfoAdded,
    #[serde(rename = "3DS_ENROLL_INITIATED")]
    ThreeDsEnrollInitiated,
    #[serde(rename = "3DS_ENROLL_CHECKED")]
    ThreeDsEnrollChecked,
    #[serde(rename = "3DS_RESULT_VERIFIED")]
    ThreeDsResultVerified,
    MarkedForReview,
    Authenticated,
    PartiallyReversed,
    #[default]
    Pending,
    Cancelled,
    Failed,
    Refunded,
    Expired,
    Reversed,
    Rejected,
    Locked,
}

impl ForeignFrom<(NoonPaymentStatus, Self)> for enums::AttemptStatus {
    fn foreign_from(data: (NoonPaymentStatus, Self)) -> Self {
        let (item, current_status) = data;
        match item {
            NoonPaymentStatus::Authorized => Self::Authorized,
            NoonPaymentStatus::Captured
            | NoonPaymentStatus::PartiallyCaptured
            | NoonPaymentStatus::PartiallyRefunded
            | NoonPaymentStatus::Refunded => Self::Charged,
            NoonPaymentStatus::Reversed | NoonPaymentStatus::PartiallyReversed => Self::Voided,
            NoonPaymentStatus::Cancelled | NoonPaymentStatus::Expired => Self::AuthenticationFailed,
            NoonPaymentStatus::ThreeDsEnrollInitiated | NoonPaymentStatus::ThreeDsEnrollChecked => {
                Self::AuthenticationPending
            }
            NoonPaymentStatus::ThreeDsResultVerified => Self::AuthenticationSuccessful,
            NoonPaymentStatus::Failed | NoonPaymentStatus::Rejected => Self::Failure,
            NoonPaymentStatus::Pending | NoonPaymentStatus::MarkedForReview => Self::Pending,
            NoonPaymentStatus::Initiated
            | NoonPaymentStatus::PaymentInfoAdded
            | NoonPaymentStatus::Authenticated => Self::Started,
            NoonPaymentStatus::Locked => current_status,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoonSubscriptionResponse {
    identifier: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsOrderResponse {
    status: NoonPaymentStatus,
    id: u64,
    error_code: u64,
    error_message: Option<String>,
    reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonCheckoutData {
    post_url: url::Url,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsResponseResult {
    order: NoonPaymentsOrderResponse,
    checkout_data: Option<NoonCheckoutData>,
    subscription: Option<NoonSubscriptionResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NoonPaymentsResponse {
    result: NoonPaymentsResponseResult,
}

impl<F, T>
    TryFrom<types::ResponseRouterData<F, NoonPaymentsResponse, T, types::PaymentsResponseData>>
    for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::ResponseRouterData<F, NoonPaymentsResponse, T, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.result.checkout_data.map(|redirection_data| {
            services::RedirectForm::Form {
                endpoint: redirection_data.post_url.to_string(),
                method: services::Method::Post,
                form_fields: std::collections::HashMap::new(),
            }
        });
        let mandate_reference =
            item.response
                .result
                .subscription
                .map(|subscription_data| types::MandateReference {
                    connector_mandate_id: Some(subscription_data.identifier),
                    payment_method_id: None,
                });
        let order = item.response.result.order;
        Ok(Self {
            status: enums::AttemptStatus::foreign_from((order.status, item.data.status)),
            response: match order.error_message {
                Some(error_message) => Err(ErrorResponse {
                    code: order.error_code.to_string(),
                    message: error_message.clone(),
                    reason: Some(error_message),
                    status_code: item.http_code,
                }),
                _ => {
                    let connector_response_reference_id =
                        order.reference.or(Some(order.id.to_string()));
                    Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::ConnectorTransactionId(
                            order.id.to_string(),
                        ),
                        redirection_data,
                        mandate_reference,
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id,
                    })
                }
            },
            ..item.data
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonActionTransaction {
    amount: String,
    currency: diesel_models::enums::Currency,
    transaction_reference: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonActionOrder {
    id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsActionRequest {
    api_operation: NoonApiOperations,
    order: NoonActionOrder,
    transaction: NoonActionTransaction,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NoonPaymentsActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        let transaction = NoonActionTransaction {
            amount: conn_utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?,
            currency: item.request.currency,
            transaction_reference: None,
        };
        Ok(Self {
            api_operation: NoonApiOperations::Capture,
            order,
            transaction,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsCancelRequest {
    api_operation: NoonApiOperations,
    order: NoonActionOrder,
}

impl TryFrom<&types::PaymentsCancelRouterData> for NoonPaymentsCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        Ok(Self {
            api_operation: NoonApiOperations::Reverse,
            order,
        })
    }
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for NoonPaymentsActionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let order = NoonActionOrder {
            id: item.request.connector_transaction_id.clone(),
        };
        let transaction = NoonActionTransaction {
            amount: conn_utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?,
            currency: item.request.currency,
            transaction_reference: Some(item.request.refund_id.clone()),
        };
        Ok(Self {
            api_operation: NoonApiOperations::Refund,
            order,
            transaction,
        })
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum RefundStatus {
    Success,
    Failed,
    #[default]
    Pending,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonPaymentsTransactionResponse {
    id: String,
    status: RefundStatus,
}

#[derive(Default, Debug, Deserialize)]
pub struct NoonRefundResponseResult {
    transaction: NoonPaymentsTransactionResponse,
}

#[derive(Default, Debug, Deserialize)]
pub struct RefundResponse {
    result: NoonRefundResponseResult,
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
                connector_refund_id: item.response.result.transaction.id,
                refund_status: enums::RefundStatus::from(item.response.result.transaction.status),
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonRefundResponseTransactions {
    id: String,
    status: RefundStatus,
    transaction_reference: Option<String>,
}

#[derive(Default, Debug, Deserialize)]
pub struct NoonRefundSyncResponseResult {
    transactions: Vec<NoonRefundResponseTransactions>,
}

#[derive(Default, Debug, Deserialize)]
pub struct RefundSyncResponse {
    result: NoonRefundSyncResponseResult,
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundSyncResponse>,
    ) -> Result<Self, Self::Error> {
        let noon_transaction: &NoonRefundResponseTransactions = item
            .response
            .result
            .transactions
            .iter()
            .find(|transaction| {
                transaction
                    .transaction_reference
                    .clone()
                    .map_or(false, |transaction_instance| {
                        transaction_instance == item.data.request.refund_id
                    })
            })
            .ok_or(errors::ConnectorError::ResponseHandlingFailed)?;

        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: noon_transaction.id.to_owned(),
                refund_status: enums::RefundStatus::from(noon_transaction.status.to_owned()),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, strum::Display)]
pub enum NoonWebhookEventTypes {
    Authenticate,
    Authorize,
    Capture,
    Fail,
    Refund,
    Sale,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonWebhookBody {
    pub order_id: u64,
    pub order_status: NoonPaymentStatus,
    pub event_type: NoonWebhookEventTypes,
    pub event_id: String,
    pub time_stamp: String,
}

#[derive(Debug, Deserialize)]
pub struct NoonWebhookSignature {
    pub signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonWebhookOrderId {
    pub order_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonWebhookEvent {
    pub order_status: NoonPaymentStatus,
    pub event_type: NoonWebhookEventTypes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonWebhookObject {
    pub order_status: NoonPaymentStatus,
    pub order_id: u64,
}

/// This from will ensure that webhook body would be properly parsed into PSync response
impl From<NoonWebhookObject> for NoonPaymentsResponse {
    fn from(value: NoonWebhookObject) -> Self {
        Self {
            result: NoonPaymentsResponseResult {
                order: NoonPaymentsOrderResponse {
                    status: value.order_status,
                    id: value.order_id,
                    //For successful payments Noon Always populates error_code as 0.
                    error_code: 0,
                    error_message: None,
                    reference: None,
                },
                checkout_data: None,
                subscription: None,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoonErrorResponse {
    pub result_code: u32,
    pub message: String,
    pub class_description: String,
}
