use common_enums::enums;
use common_utils::types::StringMajorUnit;
use common_utils::{pii, request::Method};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, PaymentsAuthorizeRequestData},
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

#[derive(Default, Debug, Deserialize, PartialEq)]
pub struct TokenioConnectorMeta {
    // FasterPayments fields
    pub faster_payments_sort_code: Option<Secret<String>>,
    pub faster_payments_account_number: Option<Secret<String>>,

    pub sepa_iban: Option<Secret<String>>,
    pub sepa_instant_iban: Option<Secret<String>>,

    pub elixir_account_number: Option<Secret<String>>,
    pub elixir_iban: Option<Secret<String>>,

    pub bankgiro_number: Option<Secret<String>>,
    pub plusgiro_number: Option<Secret<String>>,
}
impl TryFrom<&Option<pii::SecretSerdeValue>> for TokenioConnectorMeta {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        utils::to_connector_meta_from_secret(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig { config: "metadata" })
    }
}

#[derive(Debug,,Deserialize Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenioPaymentsRequest {
    pub initiation: PaymentInitiation,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInitiation {
    pub ref_id: String,
    pub remittance_information_primary: String,
    pub amount: Amount,
    pub local_instrument: LocalInstrument,
    pub creditor: Creditor,
    pub callback_url: String,
    pub flow_type: FlowType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    pub value: StringMajorUnit,
    pub currency: enums::Currency,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum LocalInstrument {
    Sepa,
    SepaInstant,
    FasterPayments,
    Elixir,
    Bankgiro,
    Plusgiro,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Creditor {
    FasterPayments {
        sort_code: Secret<String>,
        account_number: Secret<String>,
    },
    Sepa {
        iban: Secret<String>,
    },
    SepaInstant {
        iban: Secret<String>,
    },
    Elixir {
        // either iban or Polish domestic accountNumber required, choose one:
        iban: Option<Secret<String>>,
        account_number: Option<Secret<String>>,
    },
    Bankgiro {
        bankgiro_number: Secret<String>,
    },
    Plusgiro {
        plusgiro_number: Secret<String>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
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
        let auth_type = TokenioAuthType::try_from(&item.router_data.connector_auth_type)?;
        let connector_meta = TokenioConnectorMeta::try_from(&item.connector_meta_data)
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "Merchant connector account metadata",
            })?;

        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::OpenBanking(_) => {
                let (local_instrument, creditor) = match connector_meta {
                    TokenioConnectorMeta {
                        faster_payments_sort_code: Some(sort_code),
                        faster_payments_account_number: Some(account_number),
                        ..
                    } => (
                        LocalInstrument::FasterPayments,
                        Creditor::FasterPayments {
                            sort_code,
                            account_number,
                        },
                    ),

                    TokenioConnectorMeta {
                        sepa_instant_iban: Some(iban),
                        ..
                    } => (LocalInstrument::SepaInstant, Creditor::SepaInstant { iban }),

                    TokenioConnectorMeta {
                        sepa_iban: Some(iban),
                        ..
                    } => (LocalInstrument::Sepa, Creditor::Sepa { iban }),

                    TokenioConnectorMeta {
                        elixir_iban,
                        elixir_account_number,
                        ..
                    } if elixir_iban.is_some() || elixir_account_number.is_some() => (
                        LocalInstrument::Elixir,
                        Creditor::Elixir {
                            iban: elixir_iban,
                            account_number: elixir_account_number,
                        },
                    ),

                    TokenioConnectorMeta {
                        bankgiro_number: Some(bankgiro_number),
                        ..
                    } => (
                        LocalInstrument::Bankgiro,
                        Creditor::Bankgiro { bankgiro_number },
                    ),

                    TokenioConnectorMeta {
                        plusgiro_number: Some(plusgiro_number),
                        ..
                    } => (
                        LocalInstrument::Plusgiro,
                        Creditor::Plusgiro { plusgiro_number },
                    ),

                    _ => {
                        return Err(errors::ConnectorError::InvalidConnectorConfig {
                            config: "No valid payment method found in connector metadata",
                        }
                        .into())
                    }
                };
                Ok(Self {
                    initiation: PaymentInitiation {
                        ref_id: utils::generate_12_digit_number(),
                        remittance_information_primary: item.router_data.merchant_id.clone(),
                        amount: Amount {
                            value: item.amount,
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
    pub(super) key_algorithm: Secret<String>,
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
                merchant_id: api_key.to_owned(),
                private_key: api_secret.to_owned(),
                key_id: key1.to_owned(),
                key_algorithm: key2.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenioPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<TokenioPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: TokenioPaymentStatus) -> Self {
        match item {
            TokenioPaymentStatus::Succeeded => Self::Charged,
            TokenioPaymentStatus::Failed => Self::Failure,
            TokenioPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum TokenioPaymentsResponse {
    Success(PaymentResponse),
    Error(TokenioErrorResponse),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResponse {
    pub id: String,
    pub status: PaymentStatus,
    pub status_reason_information: Option<String>,
    pub authentication: Option<Authentication>,
    pub error_info: Option<ErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authentication {
    RedirectUrl { redirect_url: String },
}

#[derive(Debug, Serialize, Deserialize)]
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
            TokenioPaymentsResponse::Success(payment) => match payment.status {
                // Pending statuses - payment is still in progress
                PaymentStatus::InitiationPending
                | PaymentStatus::InitiationPendingRedirectAuth
                | PaymentStatus::InitiationPendingRedirectAuthVerification
                | PaymentStatus::InitiationPendingRedirectHp
                | PaymentStatus::InitiationPendingRedemption
                | PaymentStatus::InitiationPendingRedemptionVerification => {
                    common_enums::AttemptStatus::AuthenticationPending
                }

                // Success statuses
                PaymentStatus::SettlementCompleted => common_enums::AttemptStatus::Charged,

                // Settlement in progress - could map to different status based on business logic
                PaymentStatus::SettlementInProgress => common_enums::AttemptStatus::Pending,

                // Failure statuses
                PaymentStatus::InitiationRejected
                | PaymentStatus::InitiationFailed
                | PaymentStatus::InitiationExpired
                | PaymentStatus::InitiationRejectedInsufficientFunds
                | PaymentStatus::InitiationDeclined => common_enums::AttemptStatus::Failure,

                // Uncertain status
                PaymentStatus::InitiationCompleted
                | PaymentStatus::InitiationProcessing
                | PaymentStatus::InitiationNoFinalStatusAvailable
                | PaymentStatus::SettlementIncomplete => common_enums::AttemptStatus::Pending,
            },
            TokenioPaymentsResponse::Error(_) => common_enums::AttemptStatus::Failure,
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
            TokenioPaymentsResponse::Success(payment) => {
                if let common_enums::AttemptStatus::Failure = status {
                    // This case should ideally not be reached if the From impl for TokenioPaymentsResponse to AttemptStatus is correct
                    // but adding a fallback error response just in case.
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
                    })
                } else {
                    Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(payment.id.clone()),
                        redirection_data: Box::new(payment.authentication.as_ref().and_then(
                            |auth| match auth {
                                Authentication::RedirectUrl { redirect_url } => {
                                    Some(RedirectForm::from((redirect_url.clone(), Method::Get)))
                                }
                            },
                        )),
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
                code: error_response
                    .error_code
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
                message: error_response
                    .message
                    .clone()
                    .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
                reason: error_response
                    .message
                    .clone()
                    .or_else(|| Some(NO_ERROR_MESSAGE.to_string())),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
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
    pub amount: StringMinorUnit,
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

//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenioErrorResponse {
    pub error_code: Option<String>,
    pub message: Option<String>,
}
