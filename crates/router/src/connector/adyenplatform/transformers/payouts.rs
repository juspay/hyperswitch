#[cfg(feature = "payouts")]
use api_models::webhooks;
use common_utils::pii;
use error_stack::{report, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::{AdyenPlatformRouterData, Error};
use crate::{
    connector::{
        adyen::transformers as adyen,
        utils::{self, PayoutsData, RouterData},
    },
    core::errors,
    types::{self, api::payouts, storage::enums, transformers::ForeignFrom},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdyenPlatformConnectorMetadataObject {
    source_balance_account: Option<Secret<String>>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for AdyenPlatformConnectorMetadataObject {
    type Error = Error;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenTransferRequest {
    amount: adyen::Amount,
    balance_account_id: Secret<String>,
    category: AdyenPayoutMethod,
    counterparty: AdyenPayoutMethodDetails,
    priority: AdyenPayoutPriority,
    reference: String,
    reference_for_beneficiary: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenPayoutMethod {
    Bank,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenPayoutMethodDetails {
    bank_account: AdyenBankAccountDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBankAccountDetails {
    account_holder: AdyenBankAccountHolder,
    account_identification: AdyenBankAccountIdentification,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBankAccountHolder {
    address: Option<adyen::Address>,
    full_name: Secret<String>,
    #[serde(rename = "reference")]
    customer_id: Option<String>,
    #[serde(rename = "type")]
    entity_type: Option<EntityType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdyenBankAccountIdentification {
    #[serde(rename = "type")]
    bank_type: String,
    #[serde(flatten)]
    account_details: AdyenBankAccountIdentificationDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdyenBankAccountIdentificationDetails {
    Sepa(SepaDetails),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SepaDetails {
    iban: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenPayoutPriority {
    Instant,
    Fast,
    Regular,
    Wire,
    CrossBorder,
    Internal,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Individual,
    Organization,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenTransferResponse {
    id: String,
    account_holder: AdyenPlatformAccountHolder,
    amount: adyen::Amount,
    balance_account: AdyenBalanceAccount,
    category: AdyenPayoutMethod,
    category_data: AdyenCategoryData,
    direction: AdyenTransactionDirection,
    reference: String,
    reference_for_beneficiary: String,
    status: AdyenTransferStatus,
    #[serde(rename = "type")]
    transaction_type: AdyenTransactionType,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdyenPlatformAccountHolder {
    description: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdyenCategoryData {
    priority: AdyenPayoutPriority,
    #[serde(rename = "type")]
    category: AdyenPayoutMethod,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdyenBalanceAccount {
    description: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdyenTransactionDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdyenTransferStatus {
    Authorised,
    Refused,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenTransactionType {
    BankTransfer,
    InternalTransfer,
    Payment,
    Refund,
}

impl<F> TryFrom<&AdyenPlatformRouterData<&types::PayoutsRouterData<F>>> for AdyenTransferRequest {
    type Error = Error;
    fn try_from(
        item: &AdyenPlatformRouterData<&types::PayoutsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let request = item.router_data.request.to_owned();
        match item.router_data.get_payout_method_data()? {
            payouts::PayoutMethodData::Card(_) | payouts::PayoutMethodData::Wallet(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Adyenplatform"),
                ))?
            }

            payouts::PayoutMethodData::Bank(bd) => {
                let bank_details = match bd {
                    payouts::BankPayout::Sepa(b) => AdyenBankAccountIdentification {
                        bank_type: "iban".to_string(),
                        account_details: AdyenBankAccountIdentificationDetails::Sepa(SepaDetails {
                            iban: b.iban,
                        }),
                    },
                    payouts::BankPayout::Ach(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via ACH is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                    payouts::BankPayout::Bacs(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Bacs is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                    payouts::BankPayout::Pix(..) => Err(errors::ConnectorError::NotSupported {
                        message: "Bank transfer via Pix is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                };
                let billing_address = item.router_data.get_optional_billing();
                let address = adyen::get_address_info(billing_address).transpose()?;
                let account_holder = AdyenBankAccountHolder {
                    address,
                    full_name: item.router_data.get_billing_full_name()?,
                    customer_id: Some(
                        item.router_data
                            .get_customer_id()?
                            .get_string_repr()
                            .to_owned(),
                    ),
                    entity_type: Some(EntityType::from(request.entity_type)),
                };
                let counterparty = AdyenPayoutMethodDetails {
                    bank_account: AdyenBankAccountDetails {
                        account_holder,
                        account_identification: bank_details,
                    },
                };

                let adyen_connector_metadata_object =
                    AdyenPlatformConnectorMetadataObject::try_from(
                        &item.router_data.connector_meta_data,
                    )?;
                let balance_account_id = adyen_connector_metadata_object
                    .source_balance_account
                    .ok_or(errors::ConnectorError::InvalidConnectorConfig {
                        config: "metadata.source_balance_account",
                    })?;
                let priority =
                    request
                        .priority
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "priority",
                        })?;
                let payout_type = request.get_payout_type()?;
                Ok(Self {
                    amount: adyen::Amount {
                        value: item.amount,
                        currency: request.destination_currency,
                    },
                    balance_account_id,
                    category: AdyenPayoutMethod::try_from(payout_type)?,
                    counterparty,
                    priority: AdyenPayoutPriority::from(priority),
                    reference: item.router_data.connector_request_reference_id.clone(),
                    reference_for_beneficiary: request.payout_id,
                    description: item.router_data.description.clone(),
                })
            }
        }
    }
}

impl<F> TryFrom<types::PayoutsResponseRouterData<F, AdyenTransferResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: types::PayoutsResponseRouterData<F, AdyenTransferResponse>,
    ) -> Result<Self, Self::Error> {
        let response: AdyenTransferResponse = item.response;
        let status = enums::PayoutStatus::from(response.status);

        let error_code = match status {
            enums::PayoutStatus::Ineligible => Some(response.reason),
            _ => None,
        };

        Ok(Self {
            response: Ok(types::PayoutsResponseData {
                status: Some(status),
                connector_payout_id: Some(response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code,
                error_message: None,
            }),
            ..item.data
        })
    }
}

impl From<AdyenTransferStatus> for enums::PayoutStatus {
    fn from(adyen_status: AdyenTransferStatus) -> Self {
        match adyen_status {
            AdyenTransferStatus::Authorised => Self::Initiated,
            AdyenTransferStatus::Refused => Self::Ineligible,
            AdyenTransferStatus::Error => Self::Failed,
        }
    }
}

impl From<enums::PayoutEntityType> for EntityType {
    fn from(entity: enums::PayoutEntityType) -> Self {
        match entity {
            enums::PayoutEntityType::Individual
            | enums::PayoutEntityType::Personal
            | enums::PayoutEntityType::NaturalPerson => Self::Individual,

            enums::PayoutEntityType::Company | enums::PayoutEntityType::Business => {
                Self::Organization
            }
            _ => Self::Unknown,
        }
    }
}

impl From<enums::PayoutSendPriority> for AdyenPayoutPriority {
    fn from(entity: enums::PayoutSendPriority) -> Self {
        match entity {
            enums::PayoutSendPriority::Instant => Self::Instant,
            enums::PayoutSendPriority::Fast => Self::Fast,
            enums::PayoutSendPriority::Regular => Self::Regular,
            enums::PayoutSendPriority::Wire => Self::Wire,
            enums::PayoutSendPriority::CrossBorder => Self::CrossBorder,
            enums::PayoutSendPriority::Internal => Self::Internal,
        }
    }
}

impl TryFrom<enums::PayoutType> for AdyenPayoutMethod {
    type Error = Error;
    fn try_from(payout_type: enums::PayoutType) -> Result<Self, Self::Error> {
        match payout_type {
            enums::PayoutType::Bank => Ok(Self::Bank),
            enums::PayoutType::Card | enums::PayoutType::Wallet => {
                Err(report!(errors::ConnectorError::NotSupported {
                    message: "Card or wallet payouts".to_string(),
                    connector: "Adyenplatform",
                }))
            }
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformIncomingWebhook {
    pub data: AdyenplatformIncomingWebhookData,
    #[serde(rename = "type")]
    pub webhook_type: AdyenplatformWebhookEventType,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformIncomingWebhookData {
    pub status: AdyenplatformWebhookStatus,
    pub reference: String,
    pub tracking: Option<AdyenplatformInstantStatus>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformInstantStatus {
    status: Option<InstantPriorityStatus>,
    estimated_arrival_time: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstantPriorityStatus {
    Pending,
    Credited,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
pub enum AdyenplatformWebhookEventType {
    #[serde(rename = "balancePlatform.transfer.created")]
    PayoutCreated,
    #[serde(rename = "balancePlatform.transfer.updated")]
    PayoutUpdated,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenplatformWebhookStatus {
    Authorised,
    Booked,
    Pending,
    Failed,
    Returned,
    Received,
}

#[cfg(feature = "payouts")]
impl
    ForeignFrom<(
        AdyenplatformWebhookEventType,
        AdyenplatformWebhookStatus,
        Option<AdyenplatformInstantStatus>,
    )> for webhooks::IncomingWebhookEvent
{
    fn foreign_from(
        (event_type, status, instant_status): (
            AdyenplatformWebhookEventType,
            AdyenplatformWebhookStatus,
            Option<AdyenplatformInstantStatus>,
        ),
    ) -> Self {
        match (event_type, status, instant_status) {
            (AdyenplatformWebhookEventType::PayoutCreated, _, _) => Self::PayoutCreated,
            (AdyenplatformWebhookEventType::PayoutUpdated, _, Some(instant_status)) => {
                match (instant_status.status, instant_status.estimated_arrival_time) {
                    (Some(InstantPriorityStatus::Credited), _) | (None, Some(_)) => {
                        Self::PayoutSuccess
                    }
                    _ => Self::PayoutProcessing,
                }
            }
            (AdyenplatformWebhookEventType::PayoutUpdated, status, _) => match status {
                AdyenplatformWebhookStatus::Authorised
                | AdyenplatformWebhookStatus::Booked
                | AdyenplatformWebhookStatus::Received => Self::PayoutCreated,
                AdyenplatformWebhookStatus::Pending => Self::PayoutProcessing,
                AdyenplatformWebhookStatus::Failed => Self::PayoutFailure,
                AdyenplatformWebhookStatus::Returned => Self::PayoutReversed,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenTransferErrorResponse {
    pub error_code: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub status: u16,
    pub title: String,
    pub detail: Option<String>,
    pub request_id: Option<String>,
}
