use api_models::enums as api_enums;
use common_utils::{crypto::Encryptable, ext_traits::ValueExt, pii};
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use masking::{ExposeInterface, PeekInterface};

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

impl ForeignFrom<api_models::refunds::RefundType> for storage_enums::RefundType {
    fn foreign_from(item: api_models::refunds::RefundType) -> Self {
        match item {
            api_models::refunds::RefundType::Instant => Self::InstantRefund,
            api_models::refunds::RefundType::Scheduled => Self::RegularRefund,
        }
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

            storage_enums::AttemptStatus::PartialCharged => Self::PartiallyCaptured,
            storage_enums::AttemptStatus::Started
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

impl ForeignTryFrom<storage_enums::AttemptStatus> for storage_enums::CaptureStatus {
    type Error = error_stack::Report<errors::ApiErrorResponse>;

    fn foreign_try_from(
        attempt_status: storage_enums::AttemptStatus,
    ) -> errors::RouterResult<Self> {
        match attempt_status {
            storage_enums::AttemptStatus::Charged
            | storage_enums::AttemptStatus::PartialCharged => Ok(Self::Charged),
            storage_enums::AttemptStatus::Pending
            | storage_enums::AttemptStatus::CaptureInitiated => Ok(Self::Pending),
            storage_enums::AttemptStatus::Failure
            | storage_enums::AttemptStatus::CaptureFailed => Ok(Self::Failed),

            storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::AuthenticationPending
            | storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::Authorized
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::Voided
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::AutoRefunded
            | storage_enums::AttemptStatus::Unresolved
            | storage_enums::AttemptStatus::PaymentMethodAwaited
            | storage_enums::AttemptStatus::ConfirmationAwaited
            | storage_enums::AttemptStatus::DeviceDataCollectionPending => {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "AttemptStatus must be one of these for multiple partial captures [Charged, PartialCharged, Pending, CaptureInitiated, Failure, CaptureFailed]".into(),
                }.into())
            }
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
            currency: from.currency,
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
            currency: from.currency,
            start_date: from.start_date,
            end_date: from.end_date,
            metadata: from.metadata,
        }
    }
}

impl ForeignFrom<api_enums::IntentStatus> for Option<storage_enums::EventType> {
    fn foreign_from(value: api_enums::IntentStatus) -> Self {
        match value {
            api_enums::IntentStatus::Succeeded => Some(storage_enums::EventType::PaymentSucceeded),
            api_enums::IntentStatus::Failed => Some(storage_enums::EventType::PaymentFailed),
            api_enums::IntentStatus::Processing => {
                Some(storage_enums::EventType::PaymentProcessing)
            }
            api_enums::IntentStatus::RequiresMerchantAction
            | api_enums::IntentStatus::RequiresCustomerAction => {
                Some(storage_enums::EventType::ActionRequired)
            }
            api_enums::IntentStatus::Cancelled
            | api_enums::IntentStatus::RequiresPaymentMethod
            | api_enums::IntentStatus::RequiresConfirmation
            | api_enums::IntentStatus::RequiresCapture
            | api_enums::IntentStatus::PartiallyCaptured => None,
        }
    }
}

impl ForeignFrom<api_enums::PaymentMethodType> for api_enums::PaymentMethod {
    fn foreign_from(payment_method_type: api_enums::PaymentMethodType) -> Self {
        match payment_method_type {
            api_enums::PaymentMethodType::ApplePay
            | api_enums::PaymentMethodType::GooglePay
            | api_enums::PaymentMethodType::Paypal
            | api_enums::PaymentMethodType::AliPay
            | api_enums::PaymentMethodType::AliPayHk
            | api_enums::PaymentMethodType::Dana
            | api_enums::PaymentMethodType::MbWay
            | api_enums::PaymentMethodType::MobilePay
            | api_enums::PaymentMethodType::SamsungPay
            | api_enums::PaymentMethodType::Twint
            | api_enums::PaymentMethodType::Vipps
            | api_enums::PaymentMethodType::TouchNGo
            | api_enums::PaymentMethodType::Swish
            | api_enums::PaymentMethodType::WeChatPay
            | api_enums::PaymentMethodType::GoPay
            | api_enums::PaymentMethodType::Gcash
            | api_enums::PaymentMethodType::Momo
            | api_enums::PaymentMethodType::Cashapp
            | api_enums::PaymentMethodType::KakaoPay => Self::Wallet,
            api_enums::PaymentMethodType::Affirm
            | api_enums::PaymentMethodType::Alma
            | api_enums::PaymentMethodType::AfterpayClearpay
            | api_enums::PaymentMethodType::Klarna
            | api_enums::PaymentMethodType::PayBright
            | api_enums::PaymentMethodType::Atome
            | api_enums::PaymentMethodType::Walley => Self::PayLater,
            api_enums::PaymentMethodType::Giropay
            | api_enums::PaymentMethodType::Ideal
            | api_enums::PaymentMethodType::Sofort
            | api_enums::PaymentMethodType::Eps
            | api_enums::PaymentMethodType::BancontactCard
            | api_enums::PaymentMethodType::Blik
            | api_enums::PaymentMethodType::OnlineBankingThailand
            | api_enums::PaymentMethodType::OnlineBankingCzechRepublic
            | api_enums::PaymentMethodType::OnlineBankingFinland
            | api_enums::PaymentMethodType::OnlineBankingFpx
            | api_enums::PaymentMethodType::OnlineBankingPoland
            | api_enums::PaymentMethodType::OnlineBankingSlovakia
            | api_enums::PaymentMethodType::OpenBankingUk
            | api_enums::PaymentMethodType::Przelewy24
            | api_enums::PaymentMethodType::Trustly
            | api_enums::PaymentMethodType::Bizum
            | api_enums::PaymentMethodType::Interac => Self::BankRedirect,
            api_enums::PaymentMethodType::UpiCollect => Self::Upi,
            api_enums::PaymentMethodType::CryptoCurrency => Self::Crypto,
            api_enums::PaymentMethodType::Ach
            | api_enums::PaymentMethodType::Sepa
            | api_enums::PaymentMethodType::Bacs
            | api_enums::PaymentMethodType::Becs => Self::BankDebit,
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
                Self::Card
            }
            api_enums::PaymentMethodType::Evoucher
            | api_enums::PaymentMethodType::ClassicReward => Self::Reward,
            api_enums::PaymentMethodType::Boleto
            | api_enums::PaymentMethodType::Efecty
            | api_enums::PaymentMethodType::PagoEfectivo
            | api_enums::PaymentMethodType::RedCompra
            | api_enums::PaymentMethodType::Alfamart
            | api_enums::PaymentMethodType::Indomaret
            | api_enums::PaymentMethodType::Oxxo
            | api_enums::PaymentMethodType::SevenEleven
            | api_enums::PaymentMethodType::Lawson
            | api_enums::PaymentMethodType::MiniStop
            | api_enums::PaymentMethodType::FamilyMart
            | api_enums::PaymentMethodType::Seicomart
            | api_enums::PaymentMethodType::PayEasy
            | api_enums::PaymentMethodType::RedPagos => Self::Voucher,
            api_enums::PaymentMethodType::Pse
            | api_enums::PaymentMethodType::Multibanco
            | api_enums::PaymentMethodType::PermataBankTransfer
            | api_enums::PaymentMethodType::BcaBankTransfer
            | api_enums::PaymentMethodType::BniVa
            | api_enums::PaymentMethodType::BriVa
            | api_enums::PaymentMethodType::CimbVa
            | api_enums::PaymentMethodType::DanamonVa
            | api_enums::PaymentMethodType::MandiriVa
            | api_enums::PaymentMethodType::Pix => Self::BankTransfer,
            api_enums::PaymentMethodType::Givex | api_enums::PaymentMethodType::PaySafeCard => {
                Self::GiftCard
            }
            api_enums::PaymentMethodType::Benefit
            | api_enums::PaymentMethodType::Knet
            | api_enums::PaymentMethodType::MomoAtm => Self::CardRedirect,
        }
    }
}

impl ForeignTryFrom<api_models::payments::PaymentMethodData> for api_enums::PaymentMethod {
    type Error = errors::ApiErrorResponse;
    fn foreign_try_from(
        payment_method_data: api_models::payments::PaymentMethodData,
    ) -> Result<Self, Self::Error> {
        match payment_method_data {
            api_models::payments::PaymentMethodData::Card(..) => Ok(Self::Card),
            api_models::payments::PaymentMethodData::Wallet(..) => Ok(Self::Wallet),
            api_models::payments::PaymentMethodData::PayLater(..) => Ok(Self::PayLater),
            api_models::payments::PaymentMethodData::BankRedirect(..) => Ok(Self::BankRedirect),
            api_models::payments::PaymentMethodData::BankDebit(..) => Ok(Self::BankDebit),
            api_models::payments::PaymentMethodData::BankTransfer(..) => Ok(Self::BankTransfer),
            api_models::payments::PaymentMethodData::Crypto(..) => Ok(Self::Crypto),
            api_models::payments::PaymentMethodData::Reward => Ok(Self::Reward),
            api_models::payments::PaymentMethodData::Upi(..) => Ok(Self::Upi),
            api_models::payments::PaymentMethodData::Voucher(..) => Ok(Self::Voucher),
            api_models::payments::PaymentMethodData::GiftCard(..) => Ok(Self::GiftCard),
            api_models::payments::PaymentMethodData::CardRedirect(..) => Ok(Self::CardRedirect),
            api_models::payments::PaymentMethodData::MandatePayment => {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: ("Mandate payments cannot have payment_method_data field".to_string()),
                })
            }
        }
    }
}

impl ForeignFrom<storage_enums::RefundStatus> for Option<storage_enums::EventType> {
    fn foreign_from(value: storage_enums::RefundStatus) -> Self {
        match value {
            storage_enums::RefundStatus::Success => Some(storage_enums::EventType::RefundSucceeded),
            storage_enums::RefundStatus::Failure => Some(storage_enums::EventType::RefundFailed),
            api_enums::RefundStatus::ManualReview
            | api_enums::RefundStatus::Pending
            | api_enums::RefundStatus::TransactionFailure => None,
        }
    }
}

impl ForeignFrom<storage_enums::DisputeStatus> for storage_enums::EventType {
    fn foreign_from(value: storage_enums::DisputeStatus) -> Self {
        match value {
            storage_enums::DisputeStatus::DisputeOpened => Self::DisputeOpened,
            storage_enums::DisputeStatus::DisputeExpired => Self::DisputeExpired,
            storage_enums::DisputeStatus::DisputeAccepted => Self::DisputeAccepted,
            storage_enums::DisputeStatus::DisputeCancelled => Self::DisputeCancelled,
            storage_enums::DisputeStatus::DisputeChallenged => Self::DisputeChallenged,
            storage_enums::DisputeStatus::DisputeWon => Self::DisputeWon,
            storage_enums::DisputeStatus::DisputeLost => Self::DisputeLost,
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

impl
    ForeignFrom<(
        diesel_models::api_keys::ApiKey,
        crate::core::api_keys::PlaintextApiKey,
    )> for api_models::api_keys::CreateApiKeyResponse
{
    fn foreign_from(
        item: (
            diesel_models::api_keys::ApiKey,
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

impl ForeignFrom<diesel_models::api_keys::ApiKey> for api_models::api_keys::RetrieveApiKeyResponse {
    fn foreign_from(api_key: diesel_models::api_keys::ApiKey) -> Self {
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
    for diesel_models::api_keys::ApiKeyUpdate
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
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
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
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
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

impl ForeignFrom<diesel_models::cards_info::CardInfo> for api_models::cards_info::CardInfoResponse {
    fn foreign_from(item: diesel_models::cards_info::CardInfo) -> Self {
        Self {
            card_iin: item.card_iin,
            card_type: item.card_type,
            card_sub_type: item.card_subtype,
            card_network: item.card_network.map(|x| x.to_string()),
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
                let configs_for_frm : Vec<api_models::admin::FrmConfigs> = frm_value
                    .iter()
                    .map(|config| { config
                        .peek()
                        .clone()
                        .parse_value("FrmConfigs")
                        .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                            field_name: "frm_configs".to_string(),
                            expected_format: "[{ \"gateway\": \"stripe\", \"payment_methods\": [{ \"payment_method\": \"card\",\"payment_method_types\": [{\"payment_method_type\": \"credit\",\"card_networks\": [\"Visa\"],\"flow\": \"pre\",\"action\": \"cancel_txn\"}]}]}]".to_string(),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(configs_for_frm)
            }
            None => None,
        };
        Ok(Self {
            connector_type: item.connector_type,
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
            connector_webhook_details: item
                .connector_webhook_details
                .map(|webhook_details| {
                    serde_json::Value::parse_value(
                        webhook_details.expose(),
                        "MerchantConnectorWebhookDetails",
                    )
                    .attach_printable("Unable to deserialize connector_webhook_details")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                })
                .transpose()?,
            profile_id: item.profile_id,
        })
    }
}

impl ForeignFrom<storage::PaymentAttempt> for api_models::payments::PaymentAttemptResponse {
    fn foreign_from(payment_attempt: storage::PaymentAttempt) -> Self {
        Self {
            attempt_id: payment_attempt.attempt_id,
            status: payment_attempt.status,
            amount: payment_attempt.amount,
            currency: payment_attempt.currency,
            connector: payment_attempt.connector,
            error_message: payment_attempt.error_reason,
            payment_method: payment_attempt.payment_method,
            connector_transaction_id: payment_attempt.connector_transaction_id,
            capture_method: payment_attempt.capture_method,
            authentication_type: payment_attempt.authentication_type,
            cancellation_reason: payment_attempt.cancellation_reason,
            mandate_id: payment_attempt.mandate_id,
            error_code: payment_attempt.error_code,
            payment_token: payment_attempt.payment_token,
            connector_metadata: payment_attempt.connector_metadata,
            payment_experience: payment_attempt.payment_experience,
            payment_method_type: payment_attempt.payment_method_type,
            reference_id: payment_attempt.connector_response_reference_id,
        }
    }
}

impl ForeignFrom<storage::Capture> for api_models::payments::CaptureResponse {
    fn foreign_from(capture: storage::Capture) -> Self {
        Self {
            capture_id: capture.capture_id,
            status: capture.status,
            amount: capture.amount,
            currency: capture.currency,
            connector: capture.connector,
            authorized_attempt_id: capture.authorized_attempt_id,
            connector_capture_id: capture.connector_capture_id,
            capture_sequence: capture.capture_sequence,
            error_message: capture.error_message,
            error_code: capture.error_code,
            error_reason: capture.error_reason,
            reference_id: capture.connector_response_reference_id,
        }
    }
}

impl ForeignFrom<api_models::payouts::Bank> for api_enums::PaymentMethodType {
    fn foreign_from(value: api_models::payouts::Bank) -> Self {
        match value {
            api_models::payouts::Bank::Ach(_) => Self::Ach,
            api_models::payouts::Bank::Bacs(_) => Self::Bacs,
            api_models::payouts::Bank::Sepa(_) => Self::Sepa,
        }
    }
}

impl ForeignFrom<api_models::payouts::PayoutMethodData> for api_enums::PaymentMethod {
    fn foreign_from(value: api_models::payouts::PayoutMethodData) -> Self {
        match value {
            api_models::payouts::PayoutMethodData::Bank(_) => Self::BankTransfer,
            api_models::payouts::PayoutMethodData::Card(_) => Self::Card,
        }
    }
}

impl ForeignFrom<api_models::enums::PayoutType> for api_enums::PaymentMethod {
    fn foreign_from(value: api_models::enums::PayoutType) -> Self {
        match value {
            api_models::enums::PayoutType::Bank => Self::BankTransfer,
            api_models::enums::PayoutType::Card => Self::Card,
        }
    }
}

impl
    ForeignFrom<(
        Option<&storage::PaymentAttempt>,
        Option<&domain::Address>,
        Option<&domain::Address>,
        Option<&domain::Customer>,
    )> for api_models::payments::PaymentsRequest
{
    fn foreign_from(
        value: (
            Option<&storage::PaymentAttempt>,
            Option<&domain::Address>,
            Option<&domain::Address>,
            Option<&domain::Customer>,
        ),
    ) -> Self {
        let (payment_attempt, shipping, billing, customer) = value;
        Self {
            currency: payment_attempt.map(|pa| pa.currency.unwrap_or_default()),
            shipping: shipping.map(api_types::Address::from),
            billing: billing.map(api_types::Address::from),
            amount: payment_attempt.map(|pa| api_types::Amount::from(pa.amount)),
            email: customer
                .and_then(|cust| cust.email.as_ref().map(|em| pii::Email::from(em.clone()))),
            phone: customer.and_then(|cust| cust.phone.as_ref().map(|p| p.clone().into_inner())),
            name: customer.and_then(|cust| cust.name.as_ref().map(|n| n.clone().into_inner())),
            ..Self::default()
        }
    }
}
