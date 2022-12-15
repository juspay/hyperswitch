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

impl From<F<api_enums::RoutingAlgorithm>> for F<storage_enums::RoutingAlgorithm> {
    fn from(algo: F<api_enums::RoutingAlgorithm>) -> Self {
        Foreign(frunk::labelled_convert_from(algo.0))
    }
}

impl From<F<storage_enums::RoutingAlgorithm>> for F<api_enums::RoutingAlgorithm> {
    fn from(algo: F<storage_enums::RoutingAlgorithm>) -> Self {
        Foreign(frunk::labelled_convert_from(algo.0))
    }
}

impl From<F<api_enums::ConnectorType>> for F<storage_enums::ConnectorType> {
    fn from(conn: F<api_enums::ConnectorType>) -> Self {
        Foreign(frunk::labelled_convert_from(conn.0))
    }
}

impl From<F<storage_enums::ConnectorType>> for F<api_enums::ConnectorType> {
    fn from(conn: F<storage_enums::ConnectorType>) -> Self {
        Foreign(frunk::labelled_convert_from(conn.0))
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
        Foreign(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::PaymentMethodType>> for F<storage_enums::PaymentMethodType> {
    fn from(pm_type: F<api_enums::PaymentMethodType>) -> Self {
        Foreign(frunk::labelled_convert_from(pm_type.0))
    }
}

impl From<F<storage_enums::PaymentMethodType>> for F<api_enums::PaymentMethodType> {
    fn from(pm_type: F<storage_enums::PaymentMethodType>) -> Self {
        Foreign(frunk::labelled_convert_from(pm_type.0))
    }
}

impl From<F<api_enums::PaymentMethodSubType>> for F<storage_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<api_enums::PaymentMethodSubType>) -> Self {
        Foreign(frunk::labelled_convert_from(pm_subtype.0))
    }
}

impl From<F<storage_enums::PaymentMethodSubType>> for F<api_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<storage_enums::PaymentMethodSubType>) -> Self {
        Foreign(frunk::labelled_convert_from(pm_subtype.0))
    }
}

impl From<F<storage_enums::PaymentMethodIssuerCode>> for F<api_enums::PaymentMethodIssuerCode> {
    fn from(issuer_code: F<storage_enums::PaymentMethodIssuerCode>) -> Self {
        Foreign(frunk::labelled_convert_from(issuer_code.0))
    }
}

impl From<F<storage_enums::IntentStatus>> for F<api_enums::IntentStatus> {
    fn from(status: F<storage_enums::IntentStatus>) -> Self {
        Foreign(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::IntentStatus>> for F<storage_enums::IntentStatus> {
    fn from(status: F<api_enums::IntentStatus>) -> Self {
        Foreign(frunk::labelled_convert_from(status.0))
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
            storage_enums::AttemptStatus::PendingVbv => {
                storage_enums::IntentStatus::RequiresCustomerAction
            }

            storage_enums::AttemptStatus::PartialCharged
            | storage_enums::AttemptStatus::Started
            | storage_enums::AttemptStatus::VbvSuccessful
            | storage_enums::AttemptStatus::Authorizing
            | storage_enums::AttemptStatus::CodInitiated
            | storage_enums::AttemptStatus::VoidInitiated
            | storage_enums::AttemptStatus::CaptureInitiated
            | storage_enums::AttemptStatus::Pending => storage_enums::IntentStatus::Processing,

            storage_enums::AttemptStatus::AuthenticationFailed
            | storage_enums::AttemptStatus::AuthorizationFailed
            | storage_enums::AttemptStatus::VoidFailed
            | storage_enums::AttemptStatus::JuspayDeclined
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
        Foreign(frunk::labelled_convert_from(event_type.0))
    }
}

impl From<F<api_enums::FutureUsage>> for F<storage_enums::FutureUsage> {
    fn from(future_usage: F<api_enums::FutureUsage>) -> Self {
        Foreign(frunk::labelled_convert_from(future_usage.0))
    }
}

impl From<F<storage_enums::FutureUsage>> for F<api_enums::FutureUsage> {
    fn from(future_usage: F<storage_enums::FutureUsage>) -> Self {
        Foreign(frunk::labelled_convert_from(future_usage.0))
    }
}

impl From<F<storage_enums::RefundStatus>> for F<api_enums::RefundStatus> {
    fn from(status: F<storage_enums::RefundStatus>) -> Self {
        Foreign(frunk::labelled_convert_from(status.0))
    }
}

impl From<F<api_enums::CaptureMethod>> for F<storage_enums::CaptureMethod> {
    fn from(capture_method: F<api_enums::CaptureMethod>) -> Self {
        Foreign(frunk::labelled_convert_from(capture_method.0))
    }
}

impl From<F<storage_enums::CaptureMethod>> for F<api_enums::CaptureMethod> {
    fn from(capture_method: F<storage_enums::CaptureMethod>) -> Self {
        Foreign(frunk::labelled_convert_from(capture_method.0))
    }
}

impl From<F<api_enums::AuthenticationType>> for F<storage_enums::AuthenticationType> {
    fn from(auth_type: F<api_enums::AuthenticationType>) -> Self {
        Foreign(frunk::labelled_convert_from(auth_type.0))
    }
}

impl From<F<storage_enums::AuthenticationType>> for F<api_enums::AuthenticationType> {
    fn from(auth_type: F<storage_enums::AuthenticationType>) -> Self {
        Foreign(frunk::labelled_convert_from(auth_type.0))
    }
}

impl From<F<api_enums::Currency>> for F<storage_enums::Currency> {
    fn from(currency: F<api_enums::Currency>) -> Self {
        Foreign(frunk::labelled_convert_from(currency.0))
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
            metadata: None,
            payment_methods_enabled,
        }
        .into())
    }
}
