use api_models::enums as api_enums;
use common_utils::{crypto::Encryptable, ext_traits::ValueExt};
use error_stack::ResultExt;
use masking::PeekInterface;
use storage_models::enums as storage_enums;

use super::domain;
use crate::{
    core::errors,
    types::{api as api_types, storage},
};

pub trait ForeignInto<T> {
    fn foreign_into(self) -> T;
}

pub trait ForeignTryInto<T> {
    type Error;

    fn foreign_try_into(self) -> Result<T, Self::Error>;
}

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

impl<F, T> ForeignInto<T> for F
where
    T: ForeignFrom<F>,
{
    fn foreign_into(self) -> T {
        T::foreign_from(self)
    }
}

impl<F, T> ForeignTryInto<T> for F
where
    T: ForeignTryFrom<F>,
{
    type Error = <T as ForeignTryFrom<F>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        T::foreign_try_from(self)
    }
}

impl ForeignFrom<api_enums::ConnectorType> for storage_enums::ConnectorType {
    fn foreign_from(conn: api_enums::ConnectorType) -> Self {
        frunk::labelled_convert_from(conn)
    }
}

impl ForeignFrom<storage_enums::ConnectorType> for api_enums::ConnectorType {
    fn foreign_from(conn: storage_enums::ConnectorType) -> Self {
        frunk::labelled_convert_from(conn)
    }
}

impl ForeignFrom<api_models::refunds::RefundType> for storage_enums::RefundType {
    fn foreign_from(item: api_models::refunds::RefundType) -> Self {
        match item {
            api_models::refunds::RefundType::Instant => Self::InstantRefund,
            api_models::refunds::RefundType::Scheduled => Self::RegularRefund,
        }
    }
}

impl ForeignFrom<storage_enums::MandateStatus> for api_enums::MandateStatus {
    fn foreign_from(status: storage_enums::MandateStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::MandateStatus> for storage_enums::MandateStatus {
    fn foreign_from(status: api_enums::MandateStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::PaymentMethod> for storage_enums::PaymentMethod {
    fn foreign_from(pm_type: api_enums::PaymentMethod) -> Self {
        frunk::labelled_convert_from(pm_type)
    }
}

impl ForeignFrom<storage_enums::PaymentMethod> for api_enums::PaymentMethod {
    fn foreign_from(pm_type: storage_enums::PaymentMethod) -> Self {
        frunk::labelled_convert_from(pm_type)
    }
}

impl ForeignFrom<storage_enums::PaymentMethodIssuerCode> for api_enums::PaymentMethodIssuerCode {
    fn foreign_from(issuer_code: storage_enums::PaymentMethodIssuerCode) -> Self {
        frunk::labelled_convert_from(issuer_code)
    }
}

impl ForeignFrom<api_enums::PaymentExperience> for storage_enums::PaymentExperience {
    fn foreign_from(experience: api_enums::PaymentExperience) -> Self {
        frunk::labelled_convert_from(experience)
    }
}

impl ForeignFrom<storage_enums::PaymentExperience> for api_enums::PaymentExperience {
    fn foreign_from(experience: storage_enums::PaymentExperience) -> Self {
        frunk::labelled_convert_from(experience)
    }
}

impl ForeignFrom<storage_enums::IntentStatus> for api_enums::IntentStatus {
    fn foreign_from(status: storage_enums::IntentStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::IntentStatus> for storage_enums::IntentStatus {
    fn foreign_from(status: api_enums::IntentStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<storage_enums::AttemptStatus> for storage_enums::IntentStatus {
    fn foreign_from(s: storage_enums::AttemptStatus) -> Self {
        match s {
            storage_enums::AttemptStatus::Charged | storage_enums::AttemptStatus::AutoRefunded => {
                Self::Succeeded
            }

            storage_enums::AttemptStatus::ConfirmationAwaited => Self::RequiresConfirmation,
            storage_enums::AttemptStatus::PaymentMethodAwaited => Self::RequiresPaymentMethod,

            storage_enums::AttemptStatus::Authorized => Self::RequiresCapture,
            storage_enums::AttemptStatus::AuthenticationPending
            | storage_enums::AttemptStatus::DeviceDataCollectionPending => {
                Self::RequiresCustomerAction
            }
            storage_enums::AttemptStatus::Unresolved => Self::RequiresMerchantAction,

            storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::Pending => Self::Processing,

            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::CaptureFailed
            | storage_enums::AttemptStatus::Failure => Self::Failed,
            storage_enums::AttemptStatus::Voided => Self::Cancelled,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateType> for storage_enums::MandateDataType {
    fn foreign_from(from: api_models::payments::MandateType) -> Self {
        match from {
            api_models::payments::MandateType::SingleUse(inner) => {
                Self::SingleUse(inner.foreign_into())
            }
            api_models::payments::MandateType::MultiUse(inner) => {
                Self::MultiUse(inner.map(ForeignInto::foreign_into))
            }
        }
    }
}
impl ForeignFrom<storage_enums::MandateDataType> for api_models::payments::MandateType {
    fn foreign_from(from: storage_enums::MandateDataType) -> Self {
        match from {
            storage_enums::MandateDataType::SingleUse(inner) => {
                Self::SingleUse(inner.foreign_into())
            }
            storage_enums::MandateDataType::MultiUse(inner) => {
                Self::MultiUse(inner.map(ForeignInto::foreign_into))
            }
        }
    }
}

impl ForeignFrom<storage_enums::MandateAmountData> for api_models::payments::MandateAmountData {
    fn foreign_from(from: storage_enums::MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency.foreign_into(),
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignFrom<api_models::payments::MandateAmountData> for storage_enums::MandateAmountData {
    fn foreign_from(from: api_models::payments::MandateAmountData) -> Self {
        Self {
            amount: from.amount,
            currency: from.currency.foreign_into(),
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignTryFrom<api_enums::IntentStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: api_enums::IntentStatus) -> Result<Self, Self::Error> {
        match value {
            api_enums::IntentStatus::Succeeded => Ok(Self::PaymentSucceeded),
            api_enums::IntentStatus::Failed => Ok(Self::PaymentFailed),
            api_enums::IntentStatus::Processing => Ok(Self::PaymentProcessing),
            api_enums::IntentStatus::RequiresMerchantAction
            | api_enums::IntentStatus::RequiresCustomerAction => Ok(Self::ActionRequired),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "intent_status",
            }),
        }
    }
}

impl ForeignTryFrom<storage_enums::RefundStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: storage_enums::RefundStatus) -> Result<Self, Self::Error> {
        match value {
            storage_enums::RefundStatus::Success => Ok(Self::RefundSucceeded),
            storage_enums::RefundStatus::Failure => Ok(Self::RefundFailed),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "refund_status",
            }),
        }
    }
}

impl ForeignTryFrom<storage_enums::DisputeStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn foreign_try_from(value: storage_enums::DisputeStatus) -> Result<Self, Self::Error> {
        match value {
            storage_enums::DisputeStatus::DisputeOpened => Ok(Self::DisputeOpened),
            storage_enums::DisputeStatus::DisputeExpired => Ok(Self::DisputeExpired),
            storage_enums::DisputeStatus::DisputeAccepted => Ok(Self::DisputeAccepted),
            storage_enums::DisputeStatus::DisputeCancelled => Ok(Self::DisputeCancelled),
            storage_enums::DisputeStatus::DisputeChallenged => Ok(Self::DisputeChallenged),
            storage_enums::DisputeStatus::DisputeWon => Ok(Self::DisputeWon),
            storage_enums::DisputeStatus::DisputeLost => Ok(Self::DisputeLost),
        }
    }
}

impl ForeignTryFrom<api_models::webhooks::IncomingWebhookEvent> for storage_enums::RefundStatus {
    type Error = errors::ValidationError;

    fn foreign_try_from(
        value: api_models::webhooks::IncomingWebhookEvent,
    ) -> Result<Self, Self::Error> {
        match value {
            api_models::webhooks::IncomingWebhookEvent::RefundSuccess => Ok(Self::Success),
            api_models::webhooks::IncomingWebhookEvent::RefundFailure => Ok(Self::Failure),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "incoming_webhook_event_type",
            }),
        }
    }
}

impl ForeignFrom<storage_enums::EventType> for api_enums::EventType {
    fn foreign_from(event_type: storage_enums::EventType) -> Self {
        frunk::labelled_convert_from(event_type)
    }
}

impl ForeignFrom<api_enums::FutureUsage> for storage_enums::FutureUsage {
    fn foreign_from(future_usage: api_enums::FutureUsage) -> Self {
        frunk::labelled_convert_from(future_usage)
    }
}

impl ForeignFrom<storage_enums::FutureUsage> for api_enums::FutureUsage {
    fn foreign_from(future_usage: storage_enums::FutureUsage) -> Self {
        frunk::labelled_convert_from(future_usage)
    }
}

impl ForeignFrom<storage_enums::RefundStatus> for api_enums::RefundStatus {
    fn foreign_from(status: storage_enums::RefundStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::CaptureMethod> for storage_enums::CaptureMethod {
    fn foreign_from(capture_method: api_enums::CaptureMethod) -> Self {
        frunk::labelled_convert_from(capture_method)
    }
}

impl ForeignFrom<storage_enums::CaptureMethod> for api_enums::CaptureMethod {
    fn foreign_from(capture_method: storage_enums::CaptureMethod) -> Self {
        frunk::labelled_convert_from(capture_method)
    }
}

impl ForeignFrom<api_enums::AuthenticationType> for storage_enums::AuthenticationType {
    fn foreign_from(auth_type: api_enums::AuthenticationType) -> Self {
        frunk::labelled_convert_from(auth_type)
    }
}

impl ForeignFrom<storage_enums::AuthenticationType> for api_enums::AuthenticationType {
    fn foreign_from(auth_type: storage_enums::AuthenticationType) -> Self {
        frunk::labelled_convert_from(auth_type)
    }
}

impl ForeignFrom<api_enums::Currency> for storage_enums::Currency {
    fn foreign_from(currency: api_enums::Currency) -> Self {
        frunk::labelled_convert_from(currency)
    }
}

impl ForeignFrom<storage_enums::Currency> for api_enums::Currency {
    fn foreign_from(currency: storage_enums::Currency) -> Self {
        frunk::labelled_convert_from(currency)
    }
}

impl ForeignFrom<storage::Config> for api_types::Config {
    fn foreign_from(config: storage::Config) -> Self {
        let config = config;
        Self {
            key: config.key,
            value: config.config,
        }
    }
}

impl<'a> ForeignFrom<&'a api_types::ConfigUpdate> for storage::ConfigUpdate {
    fn foreign_from(config: &api_types::ConfigUpdate) -> Self {
        let config_update = config;
        Self::Update {
            config: Some(config_update.value.clone()),
        }
    }
}

impl<'a> From<&'a domain::Address> for api_types::Address {
    fn from(address: &domain::Address) -> Self {
        let address = address;
        Self {
            address: Some(api_types::AddressDetails {
                city: address.city.clone(),
                country: address.country,
                line1: address.line1.clone().map(Encryptable::into_inner),
                line2: address.line2.clone().map(Encryptable::into_inner),
                line3: address.line3.clone().map(Encryptable::into_inner),
                state: address.state.clone().map(Encryptable::into_inner),
                zip: address.zip.clone().map(Encryptable::into_inner),
                first_name: address.first_name.clone().map(Encryptable::into_inner),
                last_name: address.last_name.clone().map(Encryptable::into_inner),
            }),
            phone: Some(api_types::PhoneDetails {
                number: address.phone_number.clone().map(Encryptable::into_inner),
                country_code: address.country_code.clone(),
            }),
        }
    }
}

impl ForeignFrom<api_models::enums::PaymentMethodType>
    for storage_models::enums::PaymentMethodType
{
    fn foreign_from(payment_method_type: api_models::enums::PaymentMethodType) -> Self {
        frunk::labelled_convert_from(payment_method_type)
    }
}

impl ForeignFrom<storage_models::enums::PaymentMethodType>
    for api_models::enums::PaymentMethodType
{
    fn foreign_from(payment_method_type: storage_models::enums::PaymentMethodType) -> Self {
        frunk::labelled_convert_from(payment_method_type)
    }
}

impl
    ForeignFrom<(
        storage_models::api_keys::ApiKey,
        crate::core::api_keys::PlaintextApiKey,
    )> for api_models::api_keys::CreateApiKeyResponse
{
    fn foreign_from(
        item: (
            storage_models::api_keys::ApiKey,
            crate::core::api_keys::PlaintextApiKey,
        ),
    ) -> Self {
        use masking::StrongSecret;

        let (api_key, plaintext_api_key) = item;
        Self {
            key_id: api_key.key_id,
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            api_key: StrongSecret::from(plaintext_api_key.peek().to_owned()),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
    }
}

impl ForeignFrom<storage_models::api_keys::ApiKey>
    for api_models::api_keys::RetrieveApiKeyResponse
{
    fn foreign_from(api_key: storage_models::api_keys::ApiKey) -> Self {
        Self {
            key_id: api_key.key_id,
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            prefix: api_key.prefix.into(),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
    }
}

impl ForeignFrom<api_models::api_keys::UpdateApiKeyRequest>
    for storage_models::api_keys::ApiKeyUpdate
{
    fn foreign_from(api_key: api_models::api_keys::UpdateApiKeyRequest) -> Self {
        Self::Update {
            name: api_key.name,
            description: api_key.description,
            expires_at: api_key.expiration.map(Into::into),
            last_used: None,
        }
    }
}

impl ForeignFrom<storage_enums::AttemptStatus> for api_enums::AttemptStatus {
    fn foreign_from(status: storage_enums::AttemptStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::DisputeStage> for storage_enums::DisputeStage {
    fn foreign_from(status: api_enums::DisputeStage) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<api_enums::DisputeStatus> for storage_enums::DisputeStatus {
    fn foreign_from(status: api_enums::DisputeStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<storage_enums::DisputeStage> for api_enums::DisputeStage {
    fn foreign_from(status: storage_enums::DisputeStage) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignFrom<storage_enums::DisputeStatus> for api_enums::DisputeStatus {
    fn foreign_from(status: storage_enums::DisputeStatus) -> Self {
        frunk::labelled_convert_from(status)
    }
}

impl ForeignTryFrom<api_models::webhooks::IncomingWebhookEvent> for storage_enums::DisputeStatus {
    type Error = errors::ValidationError;

    fn foreign_try_from(
        value: api_models::webhooks::IncomingWebhookEvent,
    ) -> Result<Self, Self::Error> {
        match value {
            api_models::webhooks::IncomingWebhookEvent::DisputeOpened => Ok(Self::DisputeOpened),
            api_models::webhooks::IncomingWebhookEvent::DisputeExpired => Ok(Self::DisputeExpired),
            api_models::webhooks::IncomingWebhookEvent::DisputeAccepted => {
                Ok(Self::DisputeAccepted)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeCancelled => {
                Ok(Self::DisputeCancelled)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeChallenged => {
                Ok(Self::DisputeChallenged)
            }
            api_models::webhooks::IncomingWebhookEvent::DisputeWon => Ok(Self::DisputeWon),
            api_models::webhooks::IncomingWebhookEvent::DisputeLost => Ok(Self::DisputeLost),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "incoming_webhook_event",
            }),
        }
    }
}

impl ForeignFrom<storage::Dispute> for api_models::disputes::DisputeResponse {
    fn foreign_from(dispute: storage::Dispute) -> Self {
        Self {
            dispute_id: dispute.dispute_id,
            payment_id: dispute.payment_id,
            attempt_id: dispute.attempt_id,
            amount: dispute.amount,
            currency: dispute.currency,
            dispute_stage: dispute.dispute_stage.foreign_into(),
            dispute_status: dispute.dispute_status.foreign_into(),
            connector: dispute.connector,
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: dispute.created_at,
        }
    }
}

impl ForeignFrom<storage::Dispute> for api_models::disputes::DisputeResponsePaymentsRetrieve {
    fn foreign_from(dispute: storage::Dispute) -> Self {
        Self {
            dispute_id: dispute.dispute_id,
            dispute_stage: dispute.dispute_stage.foreign_into(),
            dispute_status: dispute.dispute_status.foreign_into(),
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: dispute.created_at,
        }
    }
}

impl ForeignFrom<storage::FileMetadata> for api_models::files::FileMetadataResponse {
    fn foreign_from(file_metadata: storage::FileMetadata) -> Self {
        Self {
            file_id: file_metadata.file_id,
            file_name: file_metadata.file_name,
            file_size: file_metadata.file_size,
            file_type: file_metadata.file_type,
            available: file_metadata.available,
        }
    }
}

impl ForeignFrom<storage_models::cards_info::CardInfo>
    for api_models::cards_info::CardInfoResponse
{
    fn foreign_from(item: storage_models::cards_info::CardInfo) -> Self {
        Self {
            card_iin: item.card_iin,
            card_type: item.card_type,
            card_sub_type: item.card_subtype,
            card_network: item.card_network,
            card_issuer: item.card_issuer,
            card_issuing_country: item.card_issuing_country,
        }
    }
}

impl TryFrom<domain::MerchantConnectorAccount> for api_models::admin::MerchantConnectorResponse {
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: domain::MerchantConnectorAccount) -> Result<Self, Self::Error> {
        let payment_methods_enabled = match item.payment_methods_enabled {
            Some(val) => serde_json::Value::Array(val)
                .parse_value("PaymentMethods")
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            None => None,
        };
        let frm_configs = match item.frm_configs {
            Some(frm_value) => {
                let configs_for_frm : api_models::admin::FrmConfigs = frm_value
                    .peek()
                    .clone()
                    .parse_value("FrmConfigs")
                    .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                        field_name: "frm_configs".to_string(),
                        expected_format: "\"frm_configs\" : { \"frm_enabled_pms\" : [\"card\"], \"frm_enabled_pm_types\" : [\"credit\"], \"frm_enabled_gateways\" : [\"stripe\"], \"frm_action\": \"cancel_txn\", \"frm_preferred_flow_type\" : \"pre\" }".to_string(),
                    })?;
                Some(configs_for_frm)
            }
            None => None,
        };
        Ok(Self {
            connector_type: item.connector_type.foreign_into(),
            connector_name: item.connector_name,
            connector_label: item.connector_label,
            merchant_connector_id: item.merchant_connector_id,
            connector_account_details: item.connector_account_details.into_inner(),
            test_mode: item.test_mode,
            disabled: item.disabled,
            payment_methods_enabled,
            metadata: item.metadata,
            business_country: item.business_country,
            business_label: item.business_label,
            business_sub_label: item.business_sub_label,
            frm_configs,
        })
    }
}
