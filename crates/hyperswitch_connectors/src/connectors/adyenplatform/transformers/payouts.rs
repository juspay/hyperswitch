use api_models::{payouts, webhooks};
use common_enums::enums;
use common_utils::pii;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::types;
use hyperswitch_interfaces::errors::ConnectorError;
use masking::Secret;
use serde::{Deserialize, Serialize};

use super::{AdyenPlatformRouterData, Error};
use crate::{
    connectors::adyen::transformers as adyen,
    types::PayoutsResponseRouterData,
    utils::{self, AddressDetailsData, PayoutsData as _, RouterData as _},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AdyenPlatformConnectorMetadataObject {
    source_balance_account: Option<Secret<String>>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for AdyenPlatformConnectorMetadataObject {
    type Error = Error;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(ConnectorError::InvalidConnectorConfig { config: "metadata" })?;
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
    priority: Option<AdyenPayoutPriority>,
    reference: String,
    reference_for_beneficiary: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenPayoutMethod {
    Bank,
    Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdyenPayoutMethodDetails {
    BankAccount(AdyenBankAccountDetails),
    Card(AdyenCardDetails),
    #[serde(rename = "card")]
    CardToken(AdyenCardTokenDetails),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenBankAccountDetails {
    account_holder: AdyenAccountHolder,
    account_identification: AdyenBankAccountIdentification,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAccountHolder {
    address: AdyenAddress,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    full_name: Option<Secret<String>>,
    #[serde(rename = "reference")]
    customer_id: Option<String>,
    #[serde(rename = "type")]
    entity_type: Option<EntityType>,
}

#[serde_with::skip_serializing_none]
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenAddress {
    line1: Secret<String>,
    line2: Secret<String>,
    postal_code: Option<Secret<String>>,
    state_or_province: Option<Secret<String>>,
    city: String,
    country: enums::CountryAlpha2,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCardDetails {
    card_holder: AdyenAccountHolder,
    card_identification: AdyenCardIdentification,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCardIdentification {
    #[serde(rename = "number")]
    card_number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    issue_number: Option<String>,
    start_month: Option<String>,
    start_year: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCardTokenDetails {
    card_holder: AdyenAccountHolder,
    card_identification: AdyenCardTokenIdentification,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenCardTokenIdentification {
    stored_payment_method_id: Secret<String>,
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
    category_data: Option<AdyenCategoryData>,
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
    CardTransfer,
    InternalTransfer,
    Payment,
    Refund,
}

impl TryFrom<&hyperswitch_domain_models::address::AddressDetails> for AdyenAddress {
    type Error = Error;

    fn try_from(
        address: &hyperswitch_domain_models::address::AddressDetails,
    ) -> Result<Self, Self::Error> {
        let line1 = address
            .get_line1()
            .change_context(ConnectorError::MissingRequiredField {
                field_name: "billing.address.line1",
            })?
            .clone();
        let line2 = address
            .get_line2()
            .change_context(ConnectorError::MissingRequiredField {
                field_name: "billing.address.line2",
            })?
            .clone();
        Ok(Self {
            line1,
            line2,
            postal_code: address.get_optional_zip(),
            state_or_province: address.get_optional_state(),
            city: address.get_city()?.to_owned(),
            country: address.get_country()?.to_owned(),
        })
    }
}

impl<F> TryFrom<(&types::PayoutsRouterData<F>, enums::PayoutType)> for AdyenAccountHolder {
    type Error = Error;

    fn try_from(
        (router_data, payout_type): (&types::PayoutsRouterData<F>, enums::PayoutType),
    ) -> Result<Self, Self::Error> {
        let billing_address = router_data.get_billing_address()?;
        let (first_name, last_name, full_name) = match payout_type {
            enums::PayoutType::Card => (
                Some(router_data.get_billing_first_name()?),
                Some(router_data.get_billing_last_name()?),
                None,
            ),
            enums::PayoutType::Bank => (None, None, Some(router_data.get_billing_full_name()?)),
            _ => Err(ConnectorError::NotSupported {
                message: "Payout method not supported".to_string(),
                connector: "Adyen",
            })?,
        };
        Ok(Self {
            address: billing_address.try_into()?,
            first_name,
            last_name,
            full_name,
            customer_id: Some(router_data.get_customer_id()?.get_string_repr().to_owned()),
            entity_type: Some(EntityType::from(router_data.request.entity_type)),
        })
    }
}

impl<F> TryFrom<&AdyenPlatformRouterData<&types::PayoutsRouterData<F>>> for AdyenTransferRequest {
    type Error = Error;
    fn try_from(
        item: &AdyenPlatformRouterData<&types::PayoutsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let request = &item.router_data.request;
        let (counterparty, priority) = match item.router_data.get_payout_method_data()? {
            payouts::PayoutMethodData::Wallet(_) => Err(ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Adyenplatform"),
            ))?,
            payouts::PayoutMethodData::Card(c) => {
                let card_holder: AdyenAccountHolder =
                    (item.router_data, enums::PayoutType::Card).try_into()?;
                let card_identification = AdyenCardIdentification {
                    card_number: c.card_number,
                    expiry_month: c.expiry_month,
                    expiry_year: c.expiry_year,
                    issue_number: None,
                    start_month: None,
                    start_year: None,
                };
                let counterparty = AdyenPayoutMethodDetails::Card(AdyenCardDetails {
                    card_holder,
                    card_identification,
                });
                (counterparty, None)
            }
            payouts::PayoutMethodData::Bank(bd) => {
                let account_holder: AdyenAccountHolder =
                    (item.router_data, enums::PayoutType::Bank).try_into()?;
                let bank_details = match bd {
                    payouts::Bank::Sepa(b) => AdyenBankAccountIdentification {
                        bank_type: "iban".to_string(),
                        account_details: AdyenBankAccountIdentificationDetails::Sepa(SepaDetails {
                            iban: b.iban,
                        }),
                    },
                    payouts::Bank::Ach(..) => Err(ConnectorError::NotSupported {
                        message: "Bank transfer via ACH is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                    payouts::Bank::Bacs(..) => Err(ConnectorError::NotSupported {
                        message: "Bank transfer via Bacs is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                    payouts::Bank::Pix(..) => Err(ConnectorError::NotSupported {
                        message: "Bank transfer via Pix is not supported".to_string(),
                        connector: "Adyenplatform",
                    })?,
                };
                let counterparty = AdyenPayoutMethodDetails::BankAccount(AdyenBankAccountDetails {
                    account_holder,
                    account_identification: bank_details,
                });
                let priority = request
                    .priority
                    .ok_or(ConnectorError::MissingRequiredField {
                        field_name: "priority",
                    })?;
                (counterparty, Some(AdyenPayoutPriority::from(priority)))
            }
        };
        let adyen_connector_metadata_object =
            AdyenPlatformConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;
        let balance_account_id = adyen_connector_metadata_object
            .source_balance_account
            .ok_or(ConnectorError::InvalidConnectorConfig {
                config: "metadata.source_balance_account",
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
            priority,
            reference: item.router_data.connector_request_reference_id.clone(),
            reference_for_beneficiary: item.router_data.connector_request_reference_id.clone(),
            description: item.router_data.description.clone(),
        })
    }
}

impl<F> TryFrom<PayoutsResponseRouterData<F, AdyenTransferResponse>>
    for types::PayoutsRouterData<F>
{
    type Error = Error;
    fn try_from(
        item: PayoutsResponseRouterData<F, AdyenTransferResponse>,
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
            enums::PayoutType::Card => Ok(Self::Card),
            enums::PayoutType::Wallet => Err(report!(ConnectorError::NotSupported {
                message: "Card or wallet payouts".to_string(),
                connector: "Adyenplatform",
            })),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformIncomingWebhook {
    pub data: AdyenplatformIncomingWebhookData,
    #[serde(rename = "type")]
    pub webhook_type: AdyenplatformWebhookEventType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformIncomingWebhookData {
    pub status: AdyenplatformWebhookStatus,
    pub reference: String,
    pub tracking: Option<AdyenplatformInstantStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenplatformInstantStatus {
    status: Option<InstantPriorityStatus>,
    estimated_arrival_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstantPriorityStatus {
    Pending,
    Credited,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AdyenplatformWebhookEventType {
    #[serde(rename = "balancePlatform.transfer.created")]
    PayoutCreated,
    #[serde(rename = "balancePlatform.transfer.updated")]
    PayoutUpdated,
}

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
pub fn get_adyen_webhook_event(
    event_type: AdyenplatformWebhookEventType,
    status: AdyenplatformWebhookStatus,
    instant_status: Option<AdyenplatformInstantStatus>,
) -> webhooks::IncomingWebhookEvent {
    match (event_type, status, instant_status) {
        (AdyenplatformWebhookEventType::PayoutCreated, _, _) => {
            webhooks::IncomingWebhookEvent::PayoutCreated
        }
        (AdyenplatformWebhookEventType::PayoutUpdated, _, Some(instant_status)) => {
            match (instant_status.status, instant_status.estimated_arrival_time) {
                (Some(InstantPriorityStatus::Credited), _) | (None, Some(_)) => {
                    webhooks::IncomingWebhookEvent::PayoutSuccess
                }
                _ => webhooks::IncomingWebhookEvent::PayoutProcessing,
            }
        }
        (AdyenplatformWebhookEventType::PayoutUpdated, status, _) => match status {
            AdyenplatformWebhookStatus::Authorised
            | AdyenplatformWebhookStatus::Booked
            | AdyenplatformWebhookStatus::Received => webhooks::IncomingWebhookEvent::PayoutCreated,
            AdyenplatformWebhookStatus::Pending => webhooks::IncomingWebhookEvent::PayoutProcessing,
            AdyenplatformWebhookStatus::Failed => webhooks::IncomingWebhookEvent::PayoutFailure,
            AdyenplatformWebhookStatus::Returned => webhooks::IncomingWebhookEvent::PayoutReversed,
        },
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
    pub invalid_fields: Option<Vec<AdyenInvalidField>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdyenInvalidField {
    pub name: Option<String>,
    pub value: Option<String>,
    pub message: Option<String>,
}
