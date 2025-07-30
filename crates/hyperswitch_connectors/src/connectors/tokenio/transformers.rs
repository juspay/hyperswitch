use std::collections::HashMap;

use api_models::admin::{AdditionalMerchantData, MerchantAccountData, MerchantRecipientData};
use common_enums::enums;
use common_utils::{id_type::MerchantId, request::Method, types::StringMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self},
};

//TODO: Fill the struct with respective fields
pub struct TokenioRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for TokenioRouterData<T> {
    fn from((amount, item): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenioPaymentsRequest {
    pub initiation: PaymentInitiation,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInitiation {
    pub ref_id: String,
    pub remittance_information_primary: MerchantId,
    pub amount: Amount,
    pub local_instrument: LocalInstrument,
    pub creditor: Creditor,
    pub callback_url: Option<String>,
    pub flow_type: FlowType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    pub value: StringMajorUnit,
    pub currency: enums::Currency,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LocalInstrument {
    Sepa,
    SepaInstant,
    FasterPayments,
    Elixir,
    Bankgiro,
    Plusgiro,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Creditor {
    FasterPayments {
        #[serde(rename = "sortCode")]
        sort_code: Secret<String>,
        #[serde(rename = "accountNumber")]
        account_number: Secret<String>,
        name: Secret<String>,
    },
    Sepa {
        iban: Secret<String>,
        name: Secret<String>,
    },
    SepaInstant {
        iban: Secret<String>,
        name: Secret<String>,
    },
    ElixirIban {
        iban: Secret<String>,
        name: Secret<String>,
    },
    ElixirAccount {
        #[serde(rename = "accountNumber")]
        account_number: Secret<String>,
        name: Secret<String>,
    },
    Bankgiro {
        #[serde(rename = "bankgiroNumber")]
        bankgiro_number: Secret<String>,
        name: Secret<String>,
    },
    Plusgiro {
        #[serde(rename = "plusgiroNumber")]
        plusgiro_number: Secret<String>,
        name: Secret<String>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlowType {
    ApiOnly,
    FullHostedPages,
    EmbeddedHostedPages,
}

impl TryFrom<&TokenioRouterData<&PaymentsAuthorizeRouterData>> for TokenioPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &TokenioRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::OpenBanking(_) => {
                let (local_instrument, creditor) = match item
                    .router_data
                    .additional_merchant_data
                    .as_ref()
                    .and_then(|data| match data {
                        AdditionalMerchantData::OpenBankingRecipientData(recipient_data) => {
                            match recipient_data {
                                MerchantRecipientData::AccountData(account_data) => {
                                    Some(account_data)
                                }
                                _ => None,
                            }
                        }
                    }) {
                    Some(MerchantAccountData::FasterPayments {
                        account_number,
                        sort_code,
                        name,
                        ..
                    }) => (
                        LocalInstrument::FasterPayments,
                        Creditor::FasterPayments {
                            sort_code: sort_code.clone(),
                            account_number: account_number.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::SepaInstant { iban, name, .. }) => (
                        LocalInstrument::SepaInstant,
                        Creditor::SepaInstant {
                            iban: iban.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::Sepa { iban, name, .. }) => (
                        LocalInstrument::Sepa,
                        Creditor::Sepa {
                            iban: iban.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::Iban { iban, name, .. }) => (
                        LocalInstrument::Sepa, // Assuming IBAN defaults to SEPA
                        Creditor::Sepa {
                            iban: iban.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::Elixir {
                        account_number,
                        iban,
                        name,
                        ..
                    }) => {
                        if !iban.peek().is_empty() {
                            (
                                LocalInstrument::Elixir,
                                Creditor::ElixirIban {
                                    iban: iban.clone(),
                                    name: name.clone().into(),
                                },
                            )
                        } else {
                            (
                                LocalInstrument::Elixir,
                                Creditor::ElixirAccount {
                                    account_number: account_number.clone(),
                                    name: name.clone().into(),
                                },
                            )
                        }
                    }
                    Some(MerchantAccountData::Bacs {
                        account_number,
                        sort_code,
                        name,
                        ..
                    }) => (
                        LocalInstrument::FasterPayments,
                        Creditor::FasterPayments {
                            sort_code: sort_code.clone(),
                            account_number: account_number.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::Bankgiro { number, name, .. }) => (
                        LocalInstrument::Bankgiro,
                        Creditor::Bankgiro {
                            bankgiro_number: number.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    Some(MerchantAccountData::Plusgiro { number, name, .. }) => (
                        LocalInstrument::Plusgiro,
                        Creditor::Plusgiro {
                            plusgiro_number: number.clone(),
                            name: name.clone().into(),
                        },
                    ),

                    None => {
                        return Err(errors::ConnectorError::InvalidConnectorConfig {
                            config: "No valid payment method found in additional merchant data",
                        }
                        .into())
                    }
                };
                Ok(Self {
                    initiation: PaymentInitiation {
                        ref_id: utils::generate_12_digit_number().to_string(),
                        remittance_information_primary: item.router_data.merchant_id.clone(),
                        amount: Amount {
                            value: item.amount.clone(),
                            currency: item.router_data.request.currency,
                        },
                        local_instrument,
                        creditor,
                        callback_url: item.router_data.request.router_return_url.clone(),
                        flow_type: FlowType::FullHostedPages,
                    },
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

pub struct TokenioAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) private_key: Secret<String>,
    pub(super) key_id: Secret<String>,
    pub(super) key_algorithm: CryptoAlgorithm,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum CryptoAlgorithm {
    RS256,
    ES256,
    #[serde(rename = "EdDSA")]
    EDDSA,
}

impl TryFrom<&str> for CryptoAlgorithm {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_uppercase().as_str() {
            "RS256" | "rs256" => Ok(Self::RS256),
            "ES256" | "es256" => Ok(Self::ES256),
            "EDDSA" | "eddsa" | "EdDSA" => Ok(Self::EDDSA),
            _ => Err(errors::ConnectorError::InvalidConnectorConfig {
                config: "Unsupported key algorithm. Select from RS256, ES256, EdDSA",
            }
            .into()),
        }
    }
}
impl TryFrom<&ConnectorAuthType> for TokenioAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                api_secret,
                key2,
            } => Ok(Self {
                merchant_id: key1.to_owned(),
                private_key: api_secret.to_owned(),
                key_id: api_key.to_owned(),
                key_algorithm: CryptoAlgorithm::try_from(key2.clone().expose().as_str())?,
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenioApiWrapper {
    pub payment: PaymentResponse,
}
#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(untagged)]
pub enum TokenioPaymentsResponse {
    Success(TokenioApiWrapper),
    Error(TokenioErrorResponse),
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub id: String,
    pub status: PaymentStatus,
    pub status_reason_information: Option<String>,
    pub authentication: Option<Authentication>,
    pub error_info: Option<ErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    InitiationPending,
    InitiationPendingRedirectAuth,
    InitiationPendingRedirectAuthVerification,
    InitiationPendingRedirectHp,
    InitiationPendingRedemption,
    InitiationPendingRedemptionVerification,
    InitiationProcessing,
    InitiationCompleted,
    InitiationRejected,
    InitiationRejectedInsufficientFunds,
    InitiationFailed,
    InitiationDeclined,
    InitiationExpired,
    InitiationNoFinalStatusAvailable,
    SettlementInProgress,
    SettlementCompleted,
    SettlementIncomplete,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(untagged)]
pub enum Authentication {
    RedirectUrl {
        #[serde(rename = "redirectUrl")]
        redirect_url: String,
    },
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorInfo {
    pub http_error_code: i32,
    pub message: Option<String>,
    pub token_external_error: Option<bool>,
    pub token_trace_id: Option<String>,
}

impl From<TokenioPaymentsResponse> for common_enums::AttemptStatus {
    fn from(response: TokenioPaymentsResponse) -> Self {
        match response {
            TokenioPaymentsResponse::Success(wrapper) => match wrapper.payment.status {
                // Pending statuses - payment is still in progress
                PaymentStatus::InitiationPending
                | PaymentStatus::InitiationPendingRedirectAuth
                | PaymentStatus::InitiationPendingRedirectAuthVerification
                | PaymentStatus::InitiationPendingRedirectHp
                | PaymentStatus::InitiationPendingRedemption
                | PaymentStatus::InitiationPendingRedemptionVerification => {
                    Self::AuthenticationPending
                }

                // Success statuses
                PaymentStatus::SettlementCompleted => Self::Charged,

                // Settlement in progress - could map to different status based on business logic
                PaymentStatus::SettlementInProgress => Self::Pending,

                // Failure statuses
                PaymentStatus::InitiationRejected
                | PaymentStatus::InitiationFailed
                | PaymentStatus::InitiationExpired
                | PaymentStatus::InitiationRejectedInsufficientFunds
                | PaymentStatus::InitiationDeclined => Self::Failure,

                // Uncertain status
                PaymentStatus::InitiationCompleted
                | PaymentStatus::InitiationProcessing
                | PaymentStatus::InitiationNoFinalStatusAvailable
                | PaymentStatus::SettlementIncomplete => Self::Pending,
            },
            TokenioPaymentsResponse::Error(_) => Self::Failure,
        }
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, TokenioPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, TokenioPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = common_enums::AttemptStatus::from(item.response.clone());
        let response = match item.response {
            TokenioPaymentsResponse::Success(wrapper) => {
                let payment = wrapper.payment;
                if let common_enums::AttemptStatus::Failure = status {
                    Err(ErrorResponse {
                        code: payment
                            .error_info
                            .as_ref()
                            .map(|ei| ei.http_error_code.to_string())
                            .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                        message: payment
                            .error_info
                            .as_ref()
                            .and_then(|ei| ei.message.clone())
                            .or_else(|| payment.status_reason_information.clone())
                            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                        reason: Some(
                            payment
                                .error_info
                                .as_ref()
                                .and_then(|ei| ei.message.clone())
                                .or_else(|| payment.status_reason_information.clone())
                                .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                        ),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(payment.id.clone()),
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(payment.id.clone()),
                        redirection_data: Box::new(payment.authentication.as_ref().map(|auth| {
                            match auth {
                                Authentication::RedirectUrl { redirect_url } => {
                                    RedirectForm::Form {
                                        endpoint: redirect_url.to_string(),
                                        method: Method::Get,
                                        form_fields: HashMap::new(),
                                    }
                                }
                            }
                        })),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: None,
                        incremental_authorization_allowed: None,
                        charges: None,
                    })
                }
            }
            TokenioPaymentsResponse::Error(error_response) => Err(ErrorResponse {
                code: error_response.get_error_code(),
                message: error_response.get_message(),
                reason: Some(error_response.get_message()),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            }),
        };
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct TokenioRefundRequest {
    pub amount: StringMajorUnit,
}

impl<F> TryFrom<&TokenioRouterData<&RefundsRouterData<F>>> for TokenioRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &TokenioRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(untagged)]
pub enum TokenioErrorResponse {
    Json {
        #[serde(rename = "errorCode")]
        error_code: Option<String>,
        message: Option<String>,
    },
    Text(String),
}
impl TokenioErrorResponse {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        // First try to parse as JSON
        if let Ok(json_response) = serde_json::from_slice::<Self>(bytes) {
            json_response
        } else {
            // If JSON parsing fails, treat as plain text
            let text = String::from_utf8_lossy(bytes).to_string();
            Self::Text(text)
        }
    }
    pub fn get_message(&self) -> String {
        match self {
            Self::Json {
                message,
                error_code,
            } => message
                .as_deref()
                .or(error_code.as_deref())
                .unwrap_or(NO_ERROR_MESSAGE)
                .to_string(),
            Self::Text(text) => text.clone(),
        }
    }

    pub fn get_error_code(&self) -> String {
        match self {
            Self::Json { error_code, .. } => {
                error_code.as_deref().unwrap_or(NO_ERROR_CODE).to_string()
            }
            Self::Text(_) => NO_ERROR_CODE.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenioWebhookEventType {
    PaymentStatusChanged,
    #[serde(other)]
    Unknown,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenioWebhookPaymentStatus {
    InitiationCompleted,
    PaymentCompleted,
    PaymentFailed,
    PaymentCancelled,
    InitiationRejected,
    InitiationProcessing,
    #[serde(other)]
    Unknown,
}

// Base webhook payload structure
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenioWebhookPayload {
    #[serde(rename = "eventType", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    pub id: String,
    #[serde(flatten)]
    pub event_data: TokenioWebhookEventData,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TokenioWebhookEventData {
    PaymentV2 { payment: TokenioPaymentObjectV2 },
}

// Payment v2 structures
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenioPaymentObjectV2 {
    pub id: String,
    pub status: TokenioPaymentStatus,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenioPaymentStatus {
    InitiationCompleted,
    PaymentCompleted,
    PaymentFailed,
    PaymentCancelled,
    InitiationRejected,
    InitiationProcessing,
    InitiationPendingRedirectHp,
    #[serde(other)]
    Other,
}
