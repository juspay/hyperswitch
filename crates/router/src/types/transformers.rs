use std::convert::TryInto;

use api_models::enums as api_enums;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use storage_models::enums as storage_enums;

use crate::{
    core::errors,
    types::{api as api_types, storage},
};

pub struct Foreign<T>(pub T);

type F<T> = Foreign<T>;

impl<T> From<T> for Foreign<T> {
    fn from(val: T) -> Self {
        Self(val)
    }
}

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
    Foreign<F>: Into<Foreign<T>>,
{
    fn foreign_into(self) -> T {
        let f_from = Foreign(self);
        let f_to: Foreign<T> = f_from.into();
        f_to.0
    }
}

impl<F, T> ForeignTryInto<T> for F
where
    Foreign<F>: TryInto<Foreign<T>>,
{
    type Error = <Foreign<F> as TryInto<Foreign<T>>>::Error;

    fn foreign_try_into(self) -> Result<T, Self::Error> {
        let f_from = Foreign(self);
        let f_to: Result<Foreign<T>, Self::Error> = f_from.try_into();
        f_to.map(|f| f.0)
    }
}

impl<F, T> ForeignFrom<F> for T
where
    Foreign<T>: From<Foreign<F>>,
{
    fn foreign_from(from: F) -> Self {
        let f_from = Foreign(from);
        let f_to: Foreign<Self> = f_from.into();
        f_to.0
    }
}

impl<F, T> ForeignTryFrom<F> for T
where
    Foreign<T>: TryFrom<Foreign<F>>,
{
    type Error = <Foreign<T> as TryFrom<Foreign<F>>>::Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error> {
        let f_from = Foreign(from);
        let f_to: Result<Foreign<Self>, Self::Error> = f_from.try_into();
        f_to.map(|f| f.0)
    }
}

impl From<F<api_enums::ConnectorType>> for F<storage_enums::ConnectorType> {
    fn from(conn: F<api_enums::ConnectorType>) -> Self {
        Self(frunk::labelled_convert_from(conn.0))
    }
}

impl From<F<storage_enums::ConnectorType>> for F<api_enums::ConnectorType> {
    fn from(conn: F<storage_enums::ConnectorType>) -> Self {
        Self(frunk::labelled_convert_from(conn.0))
    }
}

impl From<F<api_models::refunds::RefundType>> for F<storage_enums::RefundType> {
    fn from(item: F<api_models::refunds::RefundType>) -> Self {
        match item.0 {
            api_models::refunds::RefundType::Instant => storage_enums::RefundType::InstantRefund,
            api_models::refunds::RefundType::Scheduled => storage_enums::RefundType::RegularRefund,
        }
        .into()
    }
}

impl From<F<storage_enums::MandateStatus>> for F<api_enums::MandateStatus> {
    fn from(status: F<storage_enums::MandateStatus>) -> Self {
        Self(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::PaymentMethodType>> for F<storage_enums::PaymentMethodType> {
    fn from(pm_type: F<api_enums::PaymentMethodType>) -> Self {
        Self(frunk::labelled_convert_from(pm_type.0))
    }
}

impl From<F<storage_enums::PaymentMethodType>> for F<api_enums::PaymentMethodType> {
    fn from(pm_type: F<storage_enums::PaymentMethodType>) -> Self {
        Self(frunk::labelled_convert_from(pm_type.0))
    }
}

impl From<F<api_enums::PaymentMethodSubType>> for F<storage_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<api_enums::PaymentMethodSubType>) -> Self {
        Self(frunk::labelled_convert_from(pm_subtype.0))
    }
}

impl From<F<storage_enums::PaymentMethodSubType>> for F<api_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<storage_enums::PaymentMethodSubType>) -> Self {
        Self(frunk::labelled_convert_from(pm_subtype.0))
    }
}

impl From<F<storage_enums::PaymentMethodIssuerCode>> for F<api_enums::PaymentMethodIssuerCode> {
    fn from(issuer_code: F<storage_enums::PaymentMethodIssuerCode>) -> Self {
        Self(frunk::labelled_convert_from(issuer_code.0))
    }
}

impl From<F<api_enums::PaymentIssuer>> for F<storage_enums::PaymentIssuer> {
    fn from(issuer: F<api_enums::PaymentIssuer>) -> Self {
        Self(frunk::labelled_convert_from(issuer.0))
    }
}

impl From<F<api_enums::PaymentExperience>> for F<storage_enums::PaymentExperience> {
    fn from(experience: F<api_enums::PaymentExperience>) -> Self {
        Self(frunk::labelled_convert_from(experience.0))
    }
}

impl From<F<storage_enums::IntentStatus>> for F<api_enums::IntentStatus> {
    fn from(status: F<storage_enums::IntentStatus>) -> Self {
        Self(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::IntentStatus>> for F<storage_enums::IntentStatus> {
    fn from(status: F<api_enums::IntentStatus>) -> Self {
        Self(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<storage_enums::AttemptStatus>> for F<storage_enums::IntentStatus> {
    fn from(s: F<storage_enums::AttemptStatus>) -> Self {
        match s.0 {
            storage_enums::AttemptStatus::Charged | storage_enums::AttemptStatus::AutoRefunded => {
                storage_enums::IntentStatus::Succeeded
            }

            storage_enums::AttemptStatus::ConfirmationAwaited => {
                storage_enums::IntentStatus::RequiresConfirmation
            }
            storage_enums::AttemptStatus::PaymentMethodAwaited => {
                storage_enums::IntentStatus::RequiresPaymentMethod
            }

            storage_enums::AttemptStatus::Authorized => {
                storage_enums::IntentStatus::RequiresCapture
            }
            storage_enums::AttemptStatus::AuthenticationPending => {
                storage_enums::IntentStatus::RequiresCustomerAction
            }

            storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::AuthenticationSuccessful
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::Pending => storage_enums::IntentStatus::Processing,

            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::RouterDeclined
            | storage_enums::AttemptStatus::CaptureFailed
            | storage_enums::AttemptStatus::Failure => storage_enums::IntentStatus::Failed,
            storage_enums::AttemptStatus::Voided => storage_enums::IntentStatus::Cancelled,
        }
        .into()
    }
}

impl TryFrom<F<api_enums::IntentStatus>> for F<storage_enums::EventType> {
    type Error = errors::ValidationError;

    fn try_from(value: F<api_enums::IntentStatus>) -> Result<Self, Self::Error> {
        match value.0 {
            api_enums::IntentStatus::Succeeded => Ok(storage_enums::EventType::PaymentSucceeded),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "intent_status",
            }),
        }
        .map(Into::into)
    }
}

impl From<F<storage_enums::EventType>> for F<api_enums::EventType> {
    fn from(event_type: F<storage_enums::EventType>) -> Self {
        Self(frunk::labelled_convert_from(event_type.0))
    }
}

impl From<F<api_enums::FutureUsage>> for F<storage_enums::FutureUsage> {
    fn from(future_usage: F<api_enums::FutureUsage>) -> Self {
        Self(frunk::labelled_convert_from(future_usage.0))
    }
}

impl From<F<storage_enums::FutureUsage>> for F<api_enums::FutureUsage> {
    fn from(future_usage: F<storage_enums::FutureUsage>) -> Self {
        Self(frunk::labelled_convert_from(future_usage.0))
    }
}

impl From<F<storage_enums::RefundStatus>> for F<api_enums::RefundStatus> {
    fn from(status: F<storage_enums::RefundStatus>) -> Self {
        Self(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::CaptureMethod>> for F<storage_enums::CaptureMethod> {
    fn from(capture_method: F<api_enums::CaptureMethod>) -> Self {
        Self(frunk::labelled_convert_from(capture_method.0))
    }
}

impl From<F<storage_enums::CaptureMethod>> for F<api_enums::CaptureMethod> {
    fn from(capture_method: F<storage_enums::CaptureMethod>) -> Self {
        Self(frunk::labelled_convert_from(capture_method.0))
    }
}

impl From<F<api_enums::AuthenticationType>> for F<storage_enums::AuthenticationType> {
    fn from(auth_type: F<api_enums::AuthenticationType>) -> Self {
        Self(frunk::labelled_convert_from(auth_type.0))
    }
}

impl From<F<storage_enums::AuthenticationType>> for F<api_enums::AuthenticationType> {
    fn from(auth_type: F<storage_enums::AuthenticationType>) -> Self {
        Self(frunk::labelled_convert_from(auth_type.0))
    }
}

impl From<F<api_enums::Currency>> for F<storage_enums::Currency> {
    fn from(currency: F<api_enums::Currency>) -> Self {
        Self(frunk::labelled_convert_from(currency.0))
    }
}
impl From<F<storage_enums::Currency>> for F<api_enums::Currency> {
    fn from(currency: F<storage_enums::Currency>) -> Self {
        Self(frunk::labelled_convert_from(currency.0))
    }
}

impl<'a> From<F<&'a api_types::Address>> for F<storage::AddressUpdate> {
    fn from(address: F<&api_types::Address>) -> Self {
        let address = address.0;
        storage::AddressUpdate::Update {
            city: address.address.as_ref().and_then(|a| a.city.clone()),
            country: address.address.as_ref().and_then(|a| a.country.clone()),
            line1: address.address.as_ref().and_then(|a| a.line1.clone()),
            line2: address.address.as_ref().and_then(|a| a.line2.clone()),
            line3: address.address.as_ref().and_then(|a| a.line3.clone()),
            state: address.address.as_ref().and_then(|a| a.state.clone()),
            zip: address.address.as_ref().and_then(|a| a.zip.clone()),
            first_name: address.address.as_ref().and_then(|a| a.first_name.clone()),
            last_name: address.address.as_ref().and_then(|a| a.last_name.clone()),
            phone_number: address.phone.as_ref().and_then(|a| a.number.clone()),
            country_code: address.phone.as_ref().and_then(|a| a.country_code.clone()),
        }
        .into()
    }
}

impl From<F<storage::Config>> for F<api_types::Config> {
    fn from(config: F<storage::Config>) -> Self {
        let config = config.0;
        api_types::Config {
            key: config.key,
            value: config.config,
        }
        .into()
    }
}

impl<'a> From<F<&'a api_types::ConfigUpdate>> for F<storage::ConfigUpdate> {
    fn from(config: F<&api_types::ConfigUpdate>) -> Self {
        let config_update = config.0;
        storage::ConfigUpdate::Update {
            config: Some(config_update.value.clone()),
        }
        .into()
    }
}

impl<'a> From<F<&'a storage::Address>> for F<api_types::Address> {
    fn from(address: F<&storage::Address>) -> Self {
        let address = address.0;
        api_types::Address {
            address: Some(api_types::AddressDetails {
                city: address.city.clone(),
                country: address.country.clone(),
                line1: address.line1.clone(),
                line2: address.line2.clone(),
                line3: address.line3.clone(),
                state: address.state.clone(),
                zip: address.zip.clone(),
                first_name: address.first_name.clone(),
                last_name: address.last_name.clone(),
            }),
            phone: Some(api_types::PhoneDetails {
                number: address.phone_number.clone(),
                country_code: address.country_code.clone(),
            }),
        }
        .into()
    }
}

impl TryFrom<F<storage::MerchantConnectorAccount>>
    for F<api_models::admin::PaymentConnectorCreate>
{
    type Error = error_stack::Report<errors::ApiErrorResponse>;
    fn try_from(item: F<storage::MerchantConnectorAccount>) -> Result<Self, Self::Error> {
        let merchant_ca = item.0;

        let payment_methods_enabled = match merchant_ca.payment_methods_enabled {
            Some(val) => serde_json::Value::Array(val)
                .parse_value("PaymentMethods")
                .change_context(errors::ApiErrorResponse::InternalServerError)?,
            None => None,
        };

        Ok(api_models::admin::PaymentConnectorCreate {
            connector_type: merchant_ca.connector_type.foreign_into(),
            connector_name: merchant_ca.connector_name,
            merchant_connector_id: Some(merchant_ca.merchant_connector_id),
            connector_account_details: Some(masking::Secret::new(
                merchant_ca.connector_account_details,
            )),
            test_mode: merchant_ca.test_mode,
            disabled: merchant_ca.disabled,
            metadata: merchant_ca.metadata,
            payment_methods_enabled,
        }
        .into())
    }
}

impl From<F<api_models::payments::AddressDetails>> for F<storage_models::address::AddressNew> {
    fn from(item: F<api_models::payments::AddressDetails>) -> Self {
        let address = item.0;
        storage_models::address::AddressNew {
            city: address.city,
            country: address.country,
            line1: address.line1,
            line2: address.line2,
            line3: address.line3,
            state: address.state,
            zip: address.zip,
            first_name: address.first_name,
            last_name: address.last_name,
            ..Default::default()
        }
        .into()
    }
}

impl
    From<
        F<(
            storage_models::api_keys::ApiKey,
            crate::core::api_keys::PlaintextApiKey,
        )>,
    > for F<api_models::api_keys::CreateApiKeyResponse>
{
    fn from(
        item: F<(
            storage_models::api_keys::ApiKey,
            crate::core::api_keys::PlaintextApiKey,
        )>,
    ) -> Self {
        use masking::StrongSecret;

        let (api_key, plaintext_api_key) = item.0;
        api_models::api_keys::CreateApiKeyResponse {
            key_id: api_key.key_id.clone(),
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            api_key: StrongSecret::from(format!(
                "{}-{}",
                api_key.key_id,
                plaintext_api_key.peek().to_owned()
            )),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
        .into()
    }
}

impl From<F<storage_models::api_keys::ApiKey>> for F<api_models::api_keys::RetrieveApiKeyResponse> {
    fn from(item: F<storage_models::api_keys::ApiKey>) -> Self {
        let api_key = item.0;
        api_models::api_keys::RetrieveApiKeyResponse {
            key_id: api_key.key_id.clone(),
            merchant_id: api_key.merchant_id,
            name: api_key.name,
            description: api_key.description,
            prefix: format!("{}-{}", api_key.key_id, api_key.prefix).into(),
            created: api_key.created_at,
            expiration: api_key.expires_at.into(),
        }
        .into()
    }
}

impl From<F<api_models::api_keys::UpdateApiKeyRequest>>
    for F<storage_models::api_keys::ApiKeyUpdate>
{
    fn from(item: F<api_models::api_keys::UpdateApiKeyRequest>) -> Self {
        let api_key = item.0;
        storage_models::api_keys::ApiKeyUpdate::Update {
            name: api_key.name,
            description: api_key.description,
            expires_at: api_key.expiration.map(Into::into),
            last_used: None,
        }
        .into()
    }
}
