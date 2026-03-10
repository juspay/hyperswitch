use common_enums::{enums, AttemptStatus};
use common_utils::{errors::CustomResult, types::StringMajorUnit};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    payment_method_data::{BankDebitData, PaymentMethodData},
    router_data::{AccessToken, ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::refunds::RSync,
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types,
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{self, CustomerData, RouterData as _},
};

pub struct DwollaAuthType {
    pub(super) client_id: Secret<String>,
    pub(super) client_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DwollaAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                client_id: api_key.to_owned(),
                client_secret: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DwollaAccessTokenRequest {
    pub grant_type: String,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct DwollaAccessTokenResponse {
    access_token: Secret<String>,
    expires_in: i64,
    token_type: String,
}

pub fn extract_token_from_body(body: &[u8]) -> CustomResult<String, errors::ConnectorError> {
    let parsed: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| report!(errors::ConnectorError::ResponseDeserializationFailed))?;

    parsed
        .get("_links")
        .and_then(|links| links.get("about"))
        .and_then(|about| about.get("href"))
        .and_then(|href| href.as_str())
        .and_then(|url| url.rsplit('/').next())
        .map(|id| id.to_string())
        .ok_or_else(|| report!(errors::ConnectorError::ResponseHandlingFailed))
}

fn map_topic_to_status(topic: &str) -> DwollaPaymentStatus {
    match topic {
        "customer_transfer_created" | "customer_bank_transfer_created" => {
            DwollaPaymentStatus::Pending
        }
        "customer_transfer_completed" | "customer_bank_transfer_completed" => {
            DwollaPaymentStatus::Succeeded
        }
        "customer_transfer_failed" | "customer_bank_transfer_failed" => DwollaPaymentStatus::Failed,
        _ => DwollaPaymentStatus::Pending,
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, DwollaAccessTokenResponse, T, AccessToken>>
    for RouterData<F, T, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DwollaAccessTokenResponse, T, AccessToken>,
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

#[derive(Debug)]
pub struct DwollaRouterData<'a, T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
    pub base_url: &'a str,
}

impl<'a, T> TryFrom<(StringMajorUnit, T, &'a str)> for DwollaRouterData<'a, T> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (amount, router_data, base_url): (StringMajorUnit, T, &'a str),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount,
            router_data,
            base_url,
        })
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DwollaCustomerRequest {
    first_name: Secret<String>,
    last_name: Secret<String>,
    email: common_utils::pii::Email,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaCustomerResponse {}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaFundingSourceResponse {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DwollaFundingSourceRequest {
    routing_number: Secret<String>,
    account_number: Secret<String>,
    #[serde(rename = "type")]
    account_type: common_enums::BankType,
    name: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaPaymentsRequest {
    #[serde(rename = "_links")]
    links: DwollaPaymentLinks,
    amount: DwollaAmount,
    correlation_id: String,
}

#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaPaymentLinks {
    source: DwollaRequestLink,
    destination: DwollaRequestLink,
}

#[derive(Default, Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaRequestLink {
    href: String,
}

#[derive(Debug, Serialize, PartialEq, Deserialize, Clone)]
pub struct DwollaAmount {
    pub currency: common_enums::Currency,
    pub value: StringMajorUnit,
}

#[derive(Debug, Serialize, PartialEq, Deserialize, Clone)]
#[serde(untagged)]
pub enum DwollaPSyncResponse {
    Payment(DwollaPaymentSyncResponse),
    Webhook(DwollaWebhookDetails),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct DwollaPaymentSyncResponse {
    pub id: String,
    pub status: DwollaPaymentStatus,
    pub amount: DwollaAmount,
}

#[derive(Debug, Serialize, PartialEq, Deserialize, Clone)]
#[serde(untagged)]
pub enum DwollaRSyncResponse {
    Payment(DwollaRefundSyncResponse),
    Webhook(DwollaWebhookDetails),
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct DwollaRefundSyncResponse {
    id: String,
    status: DwollaPaymentStatus,
    amount: DwollaAmount,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DwollaMetaData {
    pub merchant_funding_source: Secret<String>,
}

#[derive(Debug, Serialize, PartialEq, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DwollaRefundsRequest {
    #[serde(rename = "_links")]
    links: DwollaPaymentLinks,
    amount: DwollaAmount,
    correlation_id: String,
}

impl TryFrom<&types::ConnectorCustomerRouterData> for DwollaCustomerRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::ConnectorCustomerRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            first_name: item.get_billing_first_name()?,
            last_name: item.get_billing_last_name()?,
            email: item
                .request
                .get_email()
                .or_else(|_| item.get_billing_email())?,
        })
    }
}

impl TryFrom<&types::TokenizationRouterData> for DwollaFundingSourceRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::TokenizationRouterData) -> Result<Self, Self::Error> {
        match item.request.payment_method_data.clone() {
            PaymentMethodData::BankDebit(BankDebitData::AchBankDebit {
                ref routing_number,
                ref account_number,
                ref bank_type,
                ref bank_account_holder_name,
                ..
            }) => {
                let account_type =
                    (*bank_type).ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                        field_name: "bank_type",
                    })?;

                let name = bank_account_holder_name.clone().ok_or_else(|| {
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "bank_account_holder_name",
                    }
                })?;

                let request = Self {
                    routing_number: routing_number.clone(),
                    account_number: account_number.clone(),
                    account_type,
                    name,
                };
                Ok(request)
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("dwolla"),
            ))?,
        }
    }
}

impl<'a> TryFrom<&DwollaRouterData<'a, &PaymentsAuthorizeRouterData>> for DwollaPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DwollaRouterData<'a, &PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let source_funding = match item.router_data.get_payment_method_token().ok() {
            Some(PaymentMethodToken::Token(pm_token)) => pm_token,
            _ => {
                return Err(report!(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method_token",
                }))
            }
        };

        let metadata = utils::to_connector_meta_from_secret::<DwollaMetaData>(
            item.router_data.connector_meta_data.clone(),
        )
        .change_context(errors::ConnectorError::InvalidConnectorConfig { config: "metadata" })?;

        let source_url = format!(
            "{}/funding-sources/{}",
            item.base_url,
            source_funding.expose()
        );

        let destination_url = format!(
            "{}/funding-sources/{}",
            item.base_url,
            metadata.merchant_funding_source.expose()
        );

        let request = Self {
            links: DwollaPaymentLinks {
                source: DwollaRequestLink { href: source_url },
                destination: DwollaRequestLink {
                    href: destination_url,
                },
            },
            amount: DwollaAmount {
                currency: item.router_data.request.currency,
                value: item.amount.to_owned(),
            },
            correlation_id: format!(
                "payment_{}",
                item.router_data.connector_request_reference_id
            ),
        };

        Ok(request)
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, DwollaPSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DwollaPSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_metadata =
            item.data
                .payment_method_token
                .as_ref()
                .and_then(|token| match token {
                    PaymentMethodToken::Token(t) => {
                        Some(serde_json::json!({ "payment_token": t.clone().expose() }))
                    }
                    _ => None,
                });
        match item.response {
            DwollaPSyncResponse::Payment(payment_response) => {
                let payment_id = payment_response.id.clone();
                let status = payment_response.status;
                Ok(Self {
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(payment_id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(payment_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    status: AttemptStatus::from(status),
                    ..item.data
                })
            }
            DwollaPSyncResponse::Webhook(webhook_response) => {
                let payment_id = webhook_response.resource_id.clone();

                Ok(Self {
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(payment_id.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata,
                        network_txn_id: None,
                        connector_response_reference_id: Some(payment_id.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                    }),
                    status: AttemptStatus::from(map_topic_to_status(
                        webhook_response.topic.as_str(),
                    )),
                    ..item.data
                })
            }
        }
    }
}

impl<'a, F> TryFrom<&DwollaRouterData<'a, &RefundsRouterData<F>>> for DwollaRefundsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &DwollaRouterData<'a, &RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let destination_funding = item
            .router_data
            .request
            .connector_metadata
            .as_ref()
            .and_then(|meta| {
                meta.get("payment_token")
                    .and_then(|token| token.as_str().map(|s| s.to_string()))
            })
            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                field_name: "payment_token",
            })?;

        let metadata = utils::to_connector_meta_from_secret::<DwollaMetaData>(
            item.router_data.connector_meta_data.clone(),
        )
        .change_context(errors::ConnectorError::InvalidConnectorConfig { config: "metadata" })?;

        let source_url = format!(
            "{}/funding-sources/{}",
            item.base_url,
            metadata.merchant_funding_source.expose()
        );

        let destination_url = format!("{}/funding-sources/{}", item.base_url, destination_funding);

        let request = Self {
            links: DwollaPaymentLinks {
                source: DwollaRequestLink { href: source_url },
                destination: DwollaRequestLink {
                    href: destination_url,
                },
            },
            amount: DwollaAmount {
                currency: item.router_data.request.currency,
                value: item.amount.to_owned(),
            },
            correlation_id: format!("refund_{}", item.router_data.connector_request_reference_id),
        };

        Ok(request)
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, DwollaRSyncResponse>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, DwollaRSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            DwollaRSyncResponse::Payment(refund_response) => {
                let refund_id = refund_response.id.clone();
                let status = refund_response.status;
                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund_id,
                        refund_status: enums::RefundStatus::from(status),
                    }),
                    ..item.data
                })
            }
            DwollaRSyncResponse::Webhook(webhook_response) => {
                let refund_id = webhook_response.resource_id.clone();
                let status = map_topic_to_status(webhook_response.topic.as_str());
                Ok(Self {
                    response: Ok(RefundsResponseData {
                        connector_refund_id: refund_id,
                        refund_status: enums::RefundStatus::from(status),
                    }),
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DwollaPaymentStatus {
    Succeeded,
    Failed,
    Pending,
    #[default]
    Processing,
    Processed,
}

impl From<DwollaPaymentStatus> for AttemptStatus {
    fn from(item: DwollaPaymentStatus) -> Self {
        match item {
            DwollaPaymentStatus::Succeeded => Self::Charged,
            DwollaPaymentStatus::Processed => Self::Charged,
            DwollaPaymentStatus::Failed => Self::Failure,
            DwollaPaymentStatus::Processing => Self::Pending,
            DwollaPaymentStatus::Pending => Self::Pending,
        }
    }
}

impl From<DwollaPaymentStatus> for enums::RefundStatus {
    fn from(item: DwollaPaymentStatus) -> Self {
        match item {
            DwollaPaymentStatus::Succeeded => Self::Success,
            DwollaPaymentStatus::Processed => Self::Success,
            DwollaPaymentStatus::Failed => Self::Failure,
            DwollaPaymentStatus::Processing => Self::Pending,
            DwollaPaymentStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaErrorResponse {
    pub code: String,
    pub message: String,
    pub _embedded: Option<Vec<DwollaErrorDetails>>,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaErrorDetails {
    pub errors: Vec<DwollaErrorDetail>,
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DwollaErrorDetail {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DwollaWebhookDetails {
    pub id: String,
    pub resource_id: String,
    pub topic: String,
    pub correlation_id: Option<String>,
}

impl From<&str> for DwollaWebhookEventType {
    fn from(topic: &str) -> Self {
        match topic {
            "customer_created" => Self::CustomerCreated,
            "customer_verified" => Self::CustomerVerified,
            "customer_funding_source_added" => Self::CustomerFundingSourceAdded,
            "customer_funding_source_removed" => Self::CustomerFundingSourceRemoved,
            "customer_funding_source_verified" => Self::CustomerFundingSourceVerified,
            "customer_funding_source_unverified" => Self::CustomerFundingSourceUnverified,
            "customer_microdeposits_added" => Self::CustomerMicrodepositsAdded,
            "customer_microdeposits_failed" => Self::CustomerMicrodepositsFailed,
            "customer_microdeposits_completed" => Self::CustomerMicrodepositsCompleted,
            "customer_microdeposits_maxattempts" => Self::CustomerMicrodepositsMaxAttempts,
            "customer_bank_transfer_creation_failed" => Self::CustomerBankTransferCreationFailed,
            "customer_bank_transfer_created" => Self::CustomerBankTransferCreated,
            "customer_transfer_created" => Self::CustomerTransferCreated,
            "customer_bank_transfer_failed" => Self::CustomerBankTransferFailed,
            "customer_bank_transfer_completed" => Self::CustomerBankTransferCompleted,
            "customer_transfer_completed" => Self::CustomerTransferCompleted,
            "customer_transfer_failed" => Self::CustomerTransferFailed,
            "transfer_created" => Self::TransferCreated,
            "transfer_pending" => Self::TransferPending,
            "transfer_completed" => Self::TransferCompleted,
            "transfer_failed" => Self::TransferFailed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DwollaWebhookEventType {
    CustomerCreated,
    CustomerVerified,
    CustomerFundingSourceAdded,
    CustomerFundingSourceRemoved,
    CustomerFundingSourceUnverified,
    CustomerFundingSourceVerified,
    CustomerMicrodepositsAdded,
    CustomerMicrodepositsFailed,
    CustomerMicrodepositsCompleted,
    CustomerMicrodepositsMaxAttempts,
    CustomerTransferCreated,
    CustomerBankTransferCreationFailed,
    CustomerBankTransferCreated,
    CustomerBankTransferCompleted,
    CustomerBankTransferFailed,
    CustomerTransferCompleted,
    CustomerTransferFailed,
    TransferCreated,
    TransferPending,
    TransferCompleted,
    TransferFailed,
    #[serde(other)]
    Unknown,
}

impl TryFrom<DwollaWebhookDetails> for api_models::webhooks::IncomingWebhookEvent {
    type Error = errors::ConnectorError;
    fn try_from(details: DwollaWebhookDetails) -> Result<Self, Self::Error> {
        let correlation_id = match details.correlation_id.as_deref() {
            Some(cid) => cid,
            None => {
                return Ok(Self::EventNotSupported);
            }
        };
        let event_type = DwollaWebhookEventType::from(details.topic.as_str());
        let is_refund = correlation_id.starts_with("refund_");
        Ok(match (event_type, is_refund) {
            (DwollaWebhookEventType::CustomerTransferCompleted, true)
            | (DwollaWebhookEventType::CustomerBankTransferCompleted, true) => Self::RefundSuccess,
            (DwollaWebhookEventType::CustomerTransferFailed, true)
            | (DwollaWebhookEventType::CustomerBankTransferFailed, true) => Self::RefundFailure,

            (DwollaWebhookEventType::CustomerTransferCreated, false)
            | (DwollaWebhookEventType::CustomerBankTransferCreated, false) => {
                Self::PaymentIntentProcessing
            }
            (DwollaWebhookEventType::CustomerTransferCompleted, false)
            | (DwollaWebhookEventType::CustomerBankTransferCompleted, false) => {
                Self::PaymentIntentSuccess
            }
            (DwollaWebhookEventType::CustomerTransferFailed, false)
            | (DwollaWebhookEventType::CustomerBankTransferFailed, false) => {
                Self::PaymentIntentFailure
            }
            _ => Self::EventNotSupported,
        })
    }
}
