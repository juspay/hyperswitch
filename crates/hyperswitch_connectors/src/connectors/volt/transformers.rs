use common_enums::enums;
use common_utils::{id_type, pii::Email, request::Method, types::MinorUnit};
use hyperswitch_domain_models::{
    payment_method_data::{BankRedirectData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types,
};
use hyperswitch_interfaces::{consts, errors};
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{
        PaymentsCancelResponseRouterData, RefreshTokenRouterData, RefundsResponseRouterData,
        ResponseRouterData,
    },
    utils::{
        self, is_payment_failure, AddressDetailsData, PaymentsAuthorizeRequestData, RouterData as _,
    },
};

const PASSWORD: &str = "password";

pub struct VoltRouterData<T> {
    pub amount: MinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for VoltRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub mod webhook_headers {
    pub const X_VOLT_SIGNED: &str = "X-Volt-Signed";
    pub const X_VOLT_TIMED: &str = "X-Volt-Timed";
    pub const USER_AGENT: &str = "User-Agent";
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsRequest {
    amount: MinorUnit,
    currency: enums::Currency,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_banking_u_k: Option<OpenBankingUk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_banking_e_u: Option<OpenBankingEu>,
    internal_reference: String,
    payer: PayerDetails,
    payment_system: PaymentSystem,
    communication: CommunicationDetails,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    Bills,
    Goods,
    PersonToPerson,
    Other,
    Services,
}

#[derive(Debug, Serialize)]
pub struct OpenBankingUk {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
pub struct OpenBankingEu {
    #[serde(rename = "type")]
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayerDetails {
    reference: id_type::CustomerId,
    email: Option<Email>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentSystem {
    OpenBankingEu,
    OpenBankingUk,
    NppPayToAu,
}

#[derive(Debug, Serialize)]
pub struct CommunicationDetails {
    #[serde[rename = "return"]]
    return_urls: ReturnUrls,
}

#[derive(Debug, Serialize)]
pub struct ReturnUrls {
    success: Link,
    failure: Link,
    pending: Link,
    cancel: Link,
}

#[derive(Debug, Serialize)]
pub struct Link {
    link: Option<String>,
}

impl TryFrom<&VoltRouterData<&types::PaymentsAuthorizeRouterData>> for VoltPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &VoltRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::BankRedirect(ref bank_redirect) => {
                let transaction_type = TransactionType::Services; //transaction_type is a form of enum, it is pre defined and value for this can not be taken from user so we are keeping it as Services as this transaction is type of service.
                let currency = item.router_data.request.currency;

                let (payment_system, open_banking_u_k, open_banking_e_u) = match bank_redirect {
                    BankRedirectData::OpenBankingUk { .. } => Ok((
                        PaymentSystem::OpenBankingUk,
                        Some(OpenBankingUk { transaction_type }),
                        None,
                    )),
                    BankRedirectData::OpenBanking {} => {
                        if matches!(currency, common_enums::Currency::GBP) {
                            Ok((
                                PaymentSystem::OpenBankingUk,
                                Some(OpenBankingUk { transaction_type }),
                                None,
                            ))
                        } else {
                            Ok((
                                PaymentSystem::OpenBankingEu,
                                None,
                                Some(OpenBankingEu { transaction_type }),
                            ))
                        }
                    }
                    BankRedirectData::BancontactCard { .. }
                    | BankRedirectData::Bizum {}
                    | BankRedirectData::Blik { .. }
                    | BankRedirectData::Eft { .. }
                    | BankRedirectData::Eps { .. }
                    | BankRedirectData::Giropay { .. }
                    | BankRedirectData::Ideal { .. }
                    | BankRedirectData::Interac { .. }
                    | BankRedirectData::OnlineBankingCzechRepublic { .. }
                    | BankRedirectData::OnlineBankingFinland { .. }
                    | BankRedirectData::OnlineBankingPoland { .. }
                    | BankRedirectData::OnlineBankingSlovakia { .. }
                    | BankRedirectData::Przelewy24 { .. }
                    | BankRedirectData::Sofort { .. }
                    | BankRedirectData::Trustly { .. }
                    | BankRedirectData::OnlineBankingFpx { .. }
                    | BankRedirectData::OnlineBankingThailand { .. }
                    | BankRedirectData::LocalBankRedirect {} => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Volt"),
                        ))
                    }
                }?;

                let amount = item.amount;
                let internal_reference = item.router_data.connector_request_reference_id.clone();
                let communication = CommunicationDetails {
                    return_urls: ReturnUrls {
                        success: Link {
                            link: item.router_data.request.router_return_url.clone(),
                        },
                        failure: Link {
                            link: item.router_data.request.router_return_url.clone(),
                        },
                        pending: Link {
                            link: item.router_data.request.router_return_url.clone(),
                        },
                        cancel: Link {
                            link: item.router_data.request.router_return_url.clone(),
                        },
                    },
                };
                let address = item.router_data.get_billing_address()?;
                let first_name = address.get_first_name()?;
                let payer = PayerDetails {
                    email: item.router_data.request.get_optional_email(),
                    first_name: first_name.to_owned(),
                    last_name: address.get_last_name().unwrap_or(first_name).to_owned(),
                    reference: item.router_data.get_customer_id()?.to_owned(),
                };

                Ok(Self {
                    amount,
                    currency,
                    internal_reference,
                    communication,
                    payer,
                    payment_system,
                    open_banking_u_k,
                    open_banking_e_u,
                })
            }
            PaymentMethodData::Card(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::Wallet(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_)
            | PaymentMethodData::CardWithLimitedDetails(_)
            | PaymentMethodData::DecryptedWalletTokenDetailsForNetworkTransactionId(_)
            | PaymentMethodData::NetworkTokenDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Volt"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VoltAuthUpdateRequest {
    grant_type: String,
    client_id: Secret<String>,
    client_secret: Secret<String>,
    username: Secret<String>,
    password: Secret<String>,
}

impl TryFrom<&RefreshTokenRouterData> for VoltAuthUpdateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefreshTokenRouterData) -> Result<Self, Self::Error> {
        let auth = VoltAuthType::try_from(&item.connector_auth_type)?;
        Ok(Self {
            grant_type: PASSWORD.to_string(),
            username: auth.username,
            password: auth.password,
            client_id: auth.client_id,
            client_secret: auth.client_secret,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VoltAuthUpdateResponse {
    pub access_token: Secret<String>,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Secret<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, VoltAuthUpdateResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VoltAuthUpdateResponse, T, AccessToken>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.access_token,
                expires: item.response.expires_in,
            }),
            ..item.data
        })
    }
}

pub struct VoltAuthType {
    pub(super) username: Secret<String>,
    pub(super) password: Secret<String>,
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for VoltAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                username: api_key.to_owned(),
                password: api_secret.to_owned(),
                client_id: key1.to_owned(),
                client_secret: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

fn get_attempt_status(
    (item, current_status): (VoltPaymentStatus, enums::AttemptStatus),
) -> enums::AttemptStatus {
    match item {
        VoltPaymentStatus::Received | VoltPaymentStatus::Settled => enums::AttemptStatus::Charged,
        VoltPaymentStatus::Completed
        | VoltPaymentStatus::DelayedAtBank
        | VoltPaymentStatus::AuthorisedByUser
        | VoltPaymentStatus::ApprovedByRisk => enums::AttemptStatus::Pending,
        VoltPaymentStatus::NewPayment
        | VoltPaymentStatus::BankRedirect
        | VoltPaymentStatus::AwaitingCheckoutAuthorisation
        | VoltPaymentStatus::AdditionalAuthorizationRequired => {
            enums::AttemptStatus::AuthenticationPending
        }
        VoltPaymentStatus::RefusedByBank
        | VoltPaymentStatus::RefusedByRisk
        | VoltPaymentStatus::NotReceived
        | VoltPaymentStatus::ErrorAtBank
        | VoltPaymentStatus::CancelledByUser
        | VoltPaymentStatus::AbandonedByUser
        | VoltPaymentStatus::Failed
        | VoltPaymentStatus::ProviderCommunicationError => enums::AttemptStatus::Failure,
        VoltPaymentStatus::Unknown => current_status,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsResponse {
    id: String,
    amount: MinorUnit,
    currency: enums::Currency,
    status: VoltPaymentStatus,
    payment_initiation_flow: VoltPaymentInitiationFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentInitiationFlow {
    status: VoltPaymentInitiationFlowStatus,
    details: VoltPaymentInitiationFlowDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoltPaymentInitiationFlowStatus {
    Processing,
    Finished,
    Aborted,
    Exception,
    WaitingForInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentInitiationFlowDetails {
    reason: String,
    redirect: VoltRedirect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltRedirect {
    url: Secret<url::Url>,
    direct_url: Secret<url::Url>,
}

impl<F, T> TryFrom<ResponseRouterData<F, VoltPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VoltPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let url = item
            .response
            .payment_initiation_flow
            .details
            .redirect
            .url
            .clone()
            .expose();
        let redirection_data = Some(RedirectForm::Form {
            endpoint: url.to_string(),
            method: Method::Get,
            form_fields: Default::default(),
        });
        Ok(Self {
            status: enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(strum::Display)]
pub enum VoltPaymentStatus {
    NewPayment,
    ApprovedByRisk,
    AdditionalAuthorizationRequired,
    AuthorisedByUser,
    ProviderCommunicationError,
    Completed,
    Received,
    NotReceived,
    BankRedirect,
    DelayedAtBank,
    AwaitingCheckoutAuthorisation,
    RefusedByBank,
    RefusedByRisk,
    ErrorAtBank,
    CancelledByUser,
    AbandonedByUser,
    Failed,
    Settled,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoltPaymentsResponseData {
    PsyncResponse(VoltPsyncResponse),
    WebhookResponse(VoltPaymentWebhookObjectResource),
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPsyncResponse {
    status: VoltPaymentStatus,
    id: String,
    merchant_internal_reference: Option<String>,
    amount: MinorUnit,
    currency: enums::Currency,
}

impl<F, T> TryFrom<ResponseRouterData<F, VoltPaymentsResponseData, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, VoltPaymentsResponseData, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            VoltPaymentsResponseData::PsyncResponse(payment_response) => {
                let status =
                    get_attempt_status((payment_response.status.clone(), item.data.status));
                Ok(Self {
                    status,
                    response: if is_payment_failure(status) {
                        Err(ErrorResponse {
                            code: payment_response.status.clone().to_string(),
                            message: payment_response.status.clone().to_string(),
                            reason: Some(payment_response.status.to_string()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(payment_response.id),
                            connector_response_reference_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    } else {
                        Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                payment_response.id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: payment_response
                                .merchant_internal_reference
                                .or(Some(payment_response.id)),
                            incremental_authorization_allowed: None,
                            authentication_data: None,
                            charges: None,
                        })
                    },
                    ..item.data
                })
            }
            VoltPaymentsResponseData::WebhookResponse(webhook_response) => {
                let detailed_status = webhook_response.detailed_status.clone();
                let status = enums::AttemptStatus::from(webhook_response.status);
                Ok(Self {
                    status,
                    response: if is_payment_failure(status) {
                        Err(ErrorResponse {
                            code: detailed_status
                                .clone()
                                .map(|volt_status| volt_status.to_string())
                                .unwrap_or_else(|| consts::NO_ERROR_CODE.to_owned()),
                            message: detailed_status
                                .clone()
                                .map(|volt_status| volt_status.to_string())
                                .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_owned()),
                            reason: detailed_status
                                .clone()
                                .map(|volt_status| volt_status.to_string()),
                            status_code: item.http_code,
                            attempt_status: None,
                            connector_transaction_id: Some(webhook_response.payment.clone()),
                            connector_response_reference_id: None,
                            network_advice_code: None,
                            network_decline_code: None,
                            network_error_message: None,
                            connector_metadata: None,
                        })
                    } else {
                        Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                webhook_response.payment.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: webhook_response
                                .merchant_internal_reference
                                .or(Some(webhook_response.payment)),
                            incremental_authorization_allowed: None,
                            authentication_data: None,
                            charges: None,
                        })
                    },
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltCancelResponse {
    payment_id: String,
    status: VoltPaymentStatus,
}

impl TryFrom<PaymentsCancelResponseRouterData<VoltCancelResponse>>
    for types::PaymentsCancelRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<VoltCancelResponse>,
    ) -> Result<Self, Self::Error> {
        let status = get_attempt_status((item.response.status.clone(), item.data.status));
        Ok(Self {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.payment_id),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            }),
            ..item.data
        })
    }
}

impl From<VoltWebhookPaymentStatus> for enums::AttemptStatus {
    fn from(status: VoltWebhookPaymentStatus) -> Self {
        match status {
            VoltWebhookPaymentStatus::Received => Self::Charged,
            VoltWebhookPaymentStatus::Failed | VoltWebhookPaymentStatus::NotReceived => {
                Self::Failure
            }
            VoltWebhookPaymentStatus::Completed | VoltWebhookPaymentStatus::Pending => {
                Self::Pending
            }
        }
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltRefundRequest {
    pub amount: MinorUnit,
    pub external_reference: String,
}

impl<F> TryFrom<&VoltRouterData<&types::RefundsRouterData<F>>> for VoltRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &VoltRouterData<&types::RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            external_reference: item.router_data.request.refund_id.clone(),
        })
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct RefundResponse {
    id: String,
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
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::Pending, //We get Refund Status only by Webhooks
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentWebhookBodyReference {
    pub payment: String,
    pub merchant_internal_reference: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltRefundWebhookBodyReference {
    pub refund: String,
    pub external_reference: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum WebhookResponse {
    // the enum order shouldn't be changed as this is being used during serialization and deserialization
    Refund(VoltRefundWebhookBodyReference),
    Payment(VoltPaymentWebhookBodyReference),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VoltWebhookBodyEventType {
    Payment(VoltPaymentsWebhookBodyEventType),
    Refund(VoltRefundsWebhookBodyEventType),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentsWebhookBodyEventType {
    pub status: VoltWebhookPaymentStatus,
    pub detailed_status: Option<VoltDetailedStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltRefundsWebhookBodyEventType {
    pub status: VoltWebhookRefundsStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VoltWebhookObjectResource {
    Payment(VoltPaymentWebhookObjectResource),
    Refund(VoltRefundWebhookObjectResource),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltPaymentWebhookObjectResource {
    #[serde(alias = "id")]
    pub payment: String,
    pub merchant_internal_reference: Option<String>,
    pub status: VoltWebhookPaymentStatus,
    pub detailed_status: Option<VoltDetailedStatus>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoltRefundWebhookObjectResource {
    pub refund: String,
    pub external_reference: Option<String>,
    pub status: VoltWebhookRefundsStatus,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoltWebhookPaymentStatus {
    Completed,
    Failed,
    Pending,
    Received,
    NotReceived,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoltWebhookRefundsStatus {
    RefundConfirmed,
    RefundFailed,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(strum::Display)]
pub enum VoltDetailedStatus {
    RefusedByRisk,
    RefusedByBank,
    ErrorAtBank,
    CancelledByUser,
    AbandonedByUser,
    Failed,
    Completed,
    BankRedirect,
    DelayedAtBank,
    AwaitingCheckoutAuthorisation,
}

impl From<VoltWebhookBodyEventType> for api_models::webhooks::IncomingWebhookEvent {
    fn from(status: VoltWebhookBodyEventType) -> Self {
        match status {
            VoltWebhookBodyEventType::Payment(payment_data) => match payment_data.status {
                VoltWebhookPaymentStatus::Received => Self::PaymentIntentSuccess,
                VoltWebhookPaymentStatus::Failed | VoltWebhookPaymentStatus::NotReceived => {
                    Self::PaymentIntentFailure
                }
                VoltWebhookPaymentStatus::Completed | VoltWebhookPaymentStatus::Pending => {
                    Self::PaymentIntentProcessing
                }
            },
            VoltWebhookBodyEventType::Refund(refund_data) => match refund_data.status {
                VoltWebhookRefundsStatus::RefundConfirmed => Self::RefundSuccess,
                VoltWebhookRefundsStatus::RefundFailed => Self::RefundFailure,
            },
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoltErrorResponse {
    pub code: Option<String>,
    pub message: String,
    pub errors: Option<Vec<Errors>>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Errors {
    #[serde(rename = "type")]
    pub error_type: String,
    pub property_path: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VoltAuthErrorResponse {
    pub code: u64,
    pub message: String,
}
