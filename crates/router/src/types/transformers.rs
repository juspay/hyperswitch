use std::convert::TryInto;

use api_models::enums as api_enums;
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
        match algo.0 {
            api_enums::RoutingAlgorithm::RoundRobin => storage_enums::RoutingAlgorithm::RoundRobin,
            api_enums::RoutingAlgorithm::MaxConversion => {
                storage_enums::RoutingAlgorithm::MaxConversion
            }
            api_enums::RoutingAlgorithm::MinCost => storage_enums::RoutingAlgorithm::MinCost,
            api_enums::RoutingAlgorithm::Custom => storage_enums::RoutingAlgorithm::Custom,
        }
        .into()
    }
}

impl From<F<storage_enums::RoutingAlgorithm>> for F<api_enums::RoutingAlgorithm> {
    fn from(algo: F<storage_enums::RoutingAlgorithm>) -> Self {
        match algo.0 {
            storage_enums::RoutingAlgorithm::RoundRobin => api_enums::RoutingAlgorithm::RoundRobin,
            storage_enums::RoutingAlgorithm::MaxConversion => {
                api_enums::RoutingAlgorithm::MaxConversion
            }
            storage_enums::RoutingAlgorithm::MinCost => api_enums::RoutingAlgorithm::MinCost,
            storage_enums::RoutingAlgorithm::Custom => api_enums::RoutingAlgorithm::Custom,
        }
        .into()
    }
}

impl From<F<api_enums::ConnectorType>> for F<storage_enums::ConnectorType> {
    fn from(conn: F<api_enums::ConnectorType>) -> Self {
        match conn.0 {
            api_enums::ConnectorType::PaymentProcessor => {
                storage_enums::ConnectorType::PaymentProcessor
            }
            api_enums::ConnectorType::PaymentVas => storage_enums::ConnectorType::PaymentVas,
            api_enums::ConnectorType::FinOperations => storage_enums::ConnectorType::FinOperations,
            api_enums::ConnectorType::FizOperations => storage_enums::ConnectorType::FizOperations,
            api_enums::ConnectorType::Networks => storage_enums::ConnectorType::Networks,
            api_enums::ConnectorType::BankingEntities => {
                storage_enums::ConnectorType::BankingEntities
            }
            api_enums::ConnectorType::NonBankingFinance => {
                storage_enums::ConnectorType::NonBankingFinance
            }
        }
        .into()
    }
}

impl From<F<storage_enums::ConnectorType>> for F<api_enums::ConnectorType> {
    fn from(conn: F<storage_enums::ConnectorType>) -> Self {
        match conn.0 {
            storage_enums::ConnectorType::PaymentProcessor => {
                api_enums::ConnectorType::PaymentProcessor
            }
            storage_enums::ConnectorType::PaymentVas => api_enums::ConnectorType::PaymentVas,
            storage_enums::ConnectorType::FinOperations => api_enums::ConnectorType::FinOperations,
            storage_enums::ConnectorType::FizOperations => api_enums::ConnectorType::FizOperations,
            storage_enums::ConnectorType::Networks => api_enums::ConnectorType::Networks,
            storage_enums::ConnectorType::BankingEntities => {
                api_enums::ConnectorType::BankingEntities
            }
            storage_enums::ConnectorType::NonBankingFinance => {
                api_enums::ConnectorType::NonBankingFinance
            }
        }
        .into()
    }
}

impl From<F<storage_enums::MandateStatus>> for F<api_enums::MandateStatus> {
    fn from(status: F<storage_enums::MandateStatus>) -> Self {
        match status.0 {
            storage_enums::MandateStatus::Active => api_enums::MandateStatus::Active,
            storage_enums::MandateStatus::Inactive => api_enums::MandateStatus::Inactive,
            storage_enums::MandateStatus::Pending => api_enums::MandateStatus::Pending,
            storage_enums::MandateStatus::Revoked => api_enums::MandateStatus::Revoked,
        }
        .into()
    }
}

impl From<F<api_enums::PaymentMethodType>> for F<storage_enums::PaymentMethodType> {
    fn from(pm_type: F<api_enums::PaymentMethodType>) -> Self {
        match pm_type.0 {
            api_enums::PaymentMethodType::Card => storage_enums::PaymentMethodType::Card,
            api_enums::PaymentMethodType::PaymentContainer => {
                storage_enums::PaymentMethodType::PaymentContainer
            }
            api_enums::PaymentMethodType::BankTransfer => {
                storage_enums::PaymentMethodType::BankTransfer
            }
            api_enums::PaymentMethodType::BankDebit => storage_enums::PaymentMethodType::BankDebit,
            api_enums::PaymentMethodType::PayLater => storage_enums::PaymentMethodType::PayLater,
            api_enums::PaymentMethodType::Netbanking => {
                storage_enums::PaymentMethodType::Netbanking
            }
            api_enums::PaymentMethodType::Upi => storage_enums::PaymentMethodType::Upi,
            api_enums::PaymentMethodType::OpenBanking => {
                storage_enums::PaymentMethodType::OpenBanking
            }
            api_enums::PaymentMethodType::ConsumerFinance => {
                storage_enums::PaymentMethodType::ConsumerFinance
            }
            api_enums::PaymentMethodType::Wallet => storage_enums::PaymentMethodType::Wallet,
            api_enums::PaymentMethodType::Klarna => storage_enums::PaymentMethodType::Klarna,
            api_enums::PaymentMethodType::Paypal => storage_enums::PaymentMethodType::Paypal,
        }
        .into()
    }
}

impl From<F<storage_enums::PaymentMethodType>> for F<api_enums::PaymentMethodType> {
    fn from(pm_type: F<storage_enums::PaymentMethodType>) -> Self {
        match pm_type.0 {
            storage_enums::PaymentMethodType::Card => api_enums::PaymentMethodType::Card,
            storage_enums::PaymentMethodType::PaymentContainer => {
                api_enums::PaymentMethodType::PaymentContainer
            }
            storage_enums::PaymentMethodType::BankTransfer => {
                api_enums::PaymentMethodType::BankTransfer
            }
            storage_enums::PaymentMethodType::BankDebit => api_enums::PaymentMethodType::BankDebit,
            storage_enums::PaymentMethodType::PayLater => api_enums::PaymentMethodType::PayLater,
            storage_enums::PaymentMethodType::Netbanking => {
                api_enums::PaymentMethodType::Netbanking
            }
            storage_enums::PaymentMethodType::Upi => api_enums::PaymentMethodType::Upi,
            storage_enums::PaymentMethodType::OpenBanking => {
                api_enums::PaymentMethodType::OpenBanking
            }
            storage_enums::PaymentMethodType::ConsumerFinance => {
                api_enums::PaymentMethodType::ConsumerFinance
            }
            storage_enums::PaymentMethodType::Wallet => api_enums::PaymentMethodType::Wallet,
            storage_enums::PaymentMethodType::Klarna => api_enums::PaymentMethodType::Klarna,
            storage_enums::PaymentMethodType::Paypal => api_enums::PaymentMethodType::Paypal,
        }
        .into()
    }
}

impl From<F<api_enums::PaymentMethodSubType>> for F<storage_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<api_enums::PaymentMethodSubType>) -> Self {
        match pm_subtype.0 {
            api_enums::PaymentMethodSubType::Credit => storage_enums::PaymentMethodSubType::Credit,
            api_enums::PaymentMethodSubType::Debit => storage_enums::PaymentMethodSubType::Debit,
            api_enums::PaymentMethodSubType::UpiIntent => {
                storage_enums::PaymentMethodSubType::UpiIntent
            }
            api_enums::PaymentMethodSubType::UpiCollect => {
                storage_enums::PaymentMethodSubType::UpiCollect
            }
            api_enums::PaymentMethodSubType::CreditCardInstallments => {
                storage_enums::PaymentMethodSubType::CreditCardInstallments
            }
            api_enums::PaymentMethodSubType::PayLaterInstallments => {
                storage_enums::PaymentMethodSubType::PayLaterInstallments
            }
        }
        .into()
    }
}

impl From<F<storage_enums::PaymentMethodSubType>> for F<api_enums::PaymentMethodSubType> {
    fn from(pm_subtype: F<storage_enums::PaymentMethodSubType>) -> Self {
        match pm_subtype.0 {
            storage_enums::PaymentMethodSubType::Credit => api_enums::PaymentMethodSubType::Credit,
            storage_enums::PaymentMethodSubType::Debit => api_enums::PaymentMethodSubType::Debit,
            storage_enums::PaymentMethodSubType::UpiIntent => {
                api_enums::PaymentMethodSubType::UpiIntent
            }
            storage_enums::PaymentMethodSubType::UpiCollect => {
                api_enums::PaymentMethodSubType::UpiCollect
            }
            storage_enums::PaymentMethodSubType::CreditCardInstallments => {
                api_enums::PaymentMethodSubType::CreditCardInstallments
            }
            storage_enums::PaymentMethodSubType::PayLaterInstallments => {
                api_enums::PaymentMethodSubType::PayLaterInstallments
            }
        }
        .into()
    }
}

impl From<F<storage_enums::PaymentMethodIssuerCode>> for F<api_enums::PaymentMethodIssuerCode> {
    fn from(issuer_code: F<storage_enums::PaymentMethodIssuerCode>) -> Self {
        match issuer_code.0 {
            storage_enums::PaymentMethodIssuerCode::JpHdfc => {
                api_enums::PaymentMethodIssuerCode::JpHdfc
            }
            storage_enums::PaymentMethodIssuerCode::JpIcici => {
                api_enums::PaymentMethodIssuerCode::JpIcici
            }
            storage_enums::PaymentMethodIssuerCode::JpGooglepay => {
                api_enums::PaymentMethodIssuerCode::JpGooglepay
            }
            storage_enums::PaymentMethodIssuerCode::JpApplepay => {
                api_enums::PaymentMethodIssuerCode::JpApplepay
            }
            storage_enums::PaymentMethodIssuerCode::JpPhonepay => {
                api_enums::PaymentMethodIssuerCode::JpPhonepay
            }
            storage_enums::PaymentMethodIssuerCode::JpWechat => {
                api_enums::PaymentMethodIssuerCode::JpWechat
            }
            storage_enums::PaymentMethodIssuerCode::JpSofort => {
                api_enums::PaymentMethodIssuerCode::JpSofort
            }
            storage_enums::PaymentMethodIssuerCode::JpGiropay => {
                api_enums::PaymentMethodIssuerCode::JpGiropay
            }
            storage_enums::PaymentMethodIssuerCode::JpSepa => {
                api_enums::PaymentMethodIssuerCode::JpSepa
            }
            storage_enums::PaymentMethodIssuerCode::JpBacs => {
                api_enums::PaymentMethodIssuerCode::JpBacs
            }
        }
        .into()
    }
}

impl From<F<storage_enums::IntentStatus>> for F<api_enums::IntentStatus> {
    fn from(status: F<storage_enums::IntentStatus>) -> Self {
        match status.0 {
            storage_enums::IntentStatus::Succeeded => api_enums::IntentStatus::Succeeded,
            storage_enums::IntentStatus::Failed => api_enums::IntentStatus::Failed,
            storage_enums::IntentStatus::Cancelled => api_enums::IntentStatus::Cancelled,
            storage_enums::IntentStatus::Processing => api_enums::IntentStatus::Processing,
            storage_enums::IntentStatus::RequiresCustomerAction => {
                api_enums::IntentStatus::RequiresCustomerAction
            }
            storage_enums::IntentStatus::RequiresPaymentMethod => {
                api_enums::IntentStatus::RequiresPaymentMethod
            }
            storage_enums::IntentStatus::RequiresConfirmation => {
                api_enums::IntentStatus::RequiresConfirmation
            }
            storage_enums::IntentStatus::RequiresCapture => {
                api_enums::IntentStatus::RequiresCapture
            }
        }
        .into()
    }
}

impl From<F<api_enums::IntentStatus>> for F<storage_enums::IntentStatus> {
    fn from(status: F<api_enums::IntentStatus>) -> Self {
        match status.0 {
            api_enums::IntentStatus::Succeeded => storage_enums::IntentStatus::Succeeded,
            api_enums::IntentStatus::Failed => storage_enums::IntentStatus::Failed,
            api_enums::IntentStatus::Cancelled => storage_enums::IntentStatus::Cancelled,
            api_enums::IntentStatus::Processing => storage_enums::IntentStatus::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => {
                storage_enums::IntentStatus::RequiresCustomerAction
            }
            api_enums::IntentStatus::RequiresPaymentMethod => {
                storage_enums::IntentStatus::RequiresPaymentMethod
            }
            api_enums::IntentStatus::RequiresConfirmation => {
                storage_enums::IntentStatus::RequiresConfirmation
            }
            api_enums::IntentStatus::RequiresCapture => {
                storage_enums::IntentStatus::RequiresCapture
            }
        }
        .into()
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
        match event_type.0 {
            storage_enums::EventType::PaymentSucceeded => api_enums::EventType::PaymentSucceeded,
        }
        .into()
    }
}

impl From<F<api_enums::FutureUsage>> for F<storage_enums::FutureUsage> {
    fn from(future_usage: F<api_enums::FutureUsage>) -> Self {
        match future_usage.0 {
            api_enums::FutureUsage::OnSession => storage_enums::FutureUsage::OnSession,
            api_enums::FutureUsage::OffSession => storage_enums::FutureUsage::OffSession,
        }
        .into()
    }
}

impl From<F<storage_enums::FutureUsage>> for F<api_enums::FutureUsage> {
    fn from(future_usage: F<storage_enums::FutureUsage>) -> Self {
        match future_usage.0 {
            storage_enums::FutureUsage::OnSession => api_enums::FutureUsage::OnSession,
            storage_enums::FutureUsage::OffSession => api_enums::FutureUsage::OffSession,
        }
        .into()
    }
}

impl From<F<storage_enums::RefundStatus>> for F<api_enums::RefundStatus> {
    fn from(status: F<storage_enums::RefundStatus>) -> Self {
        match status.0 {
            storage_enums::RefundStatus::Failure => api_enums::RefundStatus::Failure,
            storage_enums::RefundStatus::ManualReview => api_enums::RefundStatus::ManualReview,
            storage_enums::RefundStatus::Pending => api_enums::RefundStatus::Pending,
            storage_enums::RefundStatus::Success => api_enums::RefundStatus::Success,
            storage_enums::RefundStatus::TransactionFailure => {
                api_enums::RefundStatus::TransactionFailure
            }
        }
        .into()
    }
}

impl From<F<api_enums::CaptureMethod>> for F<storage_enums::CaptureMethod> {
    fn from(capture_method: F<api_enums::CaptureMethod>) -> Self {
        match capture_method.0 {
            api_enums::CaptureMethod::Automatic => storage_enums::CaptureMethod::Automatic,
            api_enums::CaptureMethod::Manual => storage_enums::CaptureMethod::Manual,
            api_enums::CaptureMethod::ManualMultiple => {
                storage_enums::CaptureMethod::ManualMultiple
            }
            api_enums::CaptureMethod::Scheduled => storage_enums::CaptureMethod::Scheduled,
        }
        .into()
    }
}

impl From<F<storage_enums::CaptureMethod>> for F<api_enums::CaptureMethod> {
    fn from(capture_method: F<storage_enums::CaptureMethod>) -> Self {
        match capture_method.0 {
            storage_enums::CaptureMethod::Automatic => api_enums::CaptureMethod::Automatic,
            storage_enums::CaptureMethod::Manual => api_enums::CaptureMethod::Manual,
            storage_enums::CaptureMethod::ManualMultiple => {
                api_enums::CaptureMethod::ManualMultiple
            }
            storage_enums::CaptureMethod::Scheduled => api_enums::CaptureMethod::Scheduled,
        }
        .into()
    }
}

impl From<F<api_enums::AuthenticationType>> for F<storage_enums::AuthenticationType> {
    fn from(auth_type: F<api_enums::AuthenticationType>) -> Self {
        match auth_type.0 {
            api_enums::AuthenticationType::ThreeDs => storage_enums::AuthenticationType::ThreeDs,
            api_enums::AuthenticationType::NoThreeDs => {
                storage_enums::AuthenticationType::NoThreeDs
            }
        }
        .into()
    }
}

impl From<F<storage_enums::AuthenticationType>> for F<api_enums::AuthenticationType> {
    fn from(auth_type: F<storage_enums::AuthenticationType>) -> Self {
        match auth_type.0 {
            storage_enums::AuthenticationType::ThreeDs => api_enums::AuthenticationType::ThreeDs,
            storage_enums::AuthenticationType::NoThreeDs => {
                api_enums::AuthenticationType::NoThreeDs
            }
        }
        .into()
    }
}

impl From<F<api_enums::Currency>> for F<storage_enums::Currency> {
    fn from(currency: F<api_enums::Currency>) -> Self {
        match currency.0 {
            api_enums::Currency::AED => storage_enums::Currency::AED,
            api_enums::Currency::ALL => storage_enums::Currency::ALL,
            api_enums::Currency::AMD => storage_enums::Currency::AMD,
            api_enums::Currency::ARS => storage_enums::Currency::ARS,
            api_enums::Currency::AUD => storage_enums::Currency::AUD,
            api_enums::Currency::AWG => storage_enums::Currency::AWG,
            api_enums::Currency::AZN => storage_enums::Currency::AZN,
            api_enums::Currency::BBD => storage_enums::Currency::BBD,
            api_enums::Currency::BDT => storage_enums::Currency::BDT,
            api_enums::Currency::BHD => storage_enums::Currency::BHD,
            api_enums::Currency::BMD => storage_enums::Currency::BMD,
            api_enums::Currency::BND => storage_enums::Currency::BND,
            api_enums::Currency::BOB => storage_enums::Currency::BOB,
            api_enums::Currency::BRL => storage_enums::Currency::BRL,
            api_enums::Currency::BSD => storage_enums::Currency::BSD,
            api_enums::Currency::BWP => storage_enums::Currency::BWP,
            api_enums::Currency::BZD => storage_enums::Currency::BZD,
            api_enums::Currency::CAD => storage_enums::Currency::CAD,
            api_enums::Currency::CHF => storage_enums::Currency::CHF,
            api_enums::Currency::CNY => storage_enums::Currency::CNY,
            api_enums::Currency::COP => storage_enums::Currency::COP,
            api_enums::Currency::CRC => storage_enums::Currency::CRC,
            api_enums::Currency::CUP => storage_enums::Currency::CUP,
            api_enums::Currency::CZK => storage_enums::Currency::CZK,
            api_enums::Currency::DKK => storage_enums::Currency::DKK,
            api_enums::Currency::DOP => storage_enums::Currency::DOP,
            api_enums::Currency::DZD => storage_enums::Currency::DZD,
            api_enums::Currency::EGP => storage_enums::Currency::EGP,
            api_enums::Currency::ETB => storage_enums::Currency::ETB,
            api_enums::Currency::EUR => storage_enums::Currency::EUR,
            api_enums::Currency::FJD => storage_enums::Currency::FJD,
            api_enums::Currency::GBP => storage_enums::Currency::GBP,
            api_enums::Currency::GHS => storage_enums::Currency::GHS,
            api_enums::Currency::GIP => storage_enums::Currency::GIP,
            api_enums::Currency::GMD => storage_enums::Currency::GMD,
            api_enums::Currency::GTQ => storage_enums::Currency::GTQ,
            api_enums::Currency::GYD => storage_enums::Currency::GYD,
            api_enums::Currency::HKD => storage_enums::Currency::HKD,
            api_enums::Currency::HNL => storage_enums::Currency::HNL,
            api_enums::Currency::HRK => storage_enums::Currency::HRK,
            api_enums::Currency::HTG => storage_enums::Currency::HTG,
            api_enums::Currency::HUF => storage_enums::Currency::HUF,
            api_enums::Currency::IDR => storage_enums::Currency::IDR,
            api_enums::Currency::ILS => storage_enums::Currency::ILS,
            api_enums::Currency::INR => storage_enums::Currency::INR,
            api_enums::Currency::JMD => storage_enums::Currency::JMD,
            api_enums::Currency::JOD => storage_enums::Currency::JOD,
            api_enums::Currency::JPY => storage_enums::Currency::JPY,
            api_enums::Currency::KES => storage_enums::Currency::KES,
            api_enums::Currency::KGS => storage_enums::Currency::KGS,
            api_enums::Currency::KHR => storage_enums::Currency::KHR,
            api_enums::Currency::KRW => storage_enums::Currency::KRW,
            api_enums::Currency::KWD => storage_enums::Currency::KWD,
            api_enums::Currency::KYD => storage_enums::Currency::KYD,
            api_enums::Currency::KZT => storage_enums::Currency::KZT,
            api_enums::Currency::LAK => storage_enums::Currency::LAK,
            api_enums::Currency::LBP => storage_enums::Currency::LBP,
            api_enums::Currency::LKR => storage_enums::Currency::LKR,
            api_enums::Currency::LRD => storage_enums::Currency::LRD,
            api_enums::Currency::LSL => storage_enums::Currency::LSL,
            api_enums::Currency::MAD => storage_enums::Currency::MAD,
            api_enums::Currency::MDL => storage_enums::Currency::MDL,
            api_enums::Currency::MKD => storage_enums::Currency::MKD,
            api_enums::Currency::MMK => storage_enums::Currency::MMK,
            api_enums::Currency::MNT => storage_enums::Currency::MNT,
            api_enums::Currency::MOP => storage_enums::Currency::MOP,
            api_enums::Currency::MUR => storage_enums::Currency::MUR,
            api_enums::Currency::MVR => storage_enums::Currency::MVR,
            api_enums::Currency::MWK => storage_enums::Currency::MWK,
            api_enums::Currency::MXN => storage_enums::Currency::MXN,
            api_enums::Currency::MYR => storage_enums::Currency::MYR,
            api_enums::Currency::NAD => storage_enums::Currency::NAD,
            api_enums::Currency::NGN => storage_enums::Currency::NGN,
            api_enums::Currency::NIO => storage_enums::Currency::NIO,
            api_enums::Currency::NOK => storage_enums::Currency::NOK,
            api_enums::Currency::NPR => storage_enums::Currency::NPR,
            api_enums::Currency::NZD => storage_enums::Currency::NZD,
            api_enums::Currency::OMR => storage_enums::Currency::OMR,
            api_enums::Currency::PEN => storage_enums::Currency::PEN,
            api_enums::Currency::PGK => storage_enums::Currency::PGK,
            api_enums::Currency::PHP => storage_enums::Currency::PHP,
            api_enums::Currency::PKR => storage_enums::Currency::PKR,
            api_enums::Currency::PLN => storage_enums::Currency::PLN,
            api_enums::Currency::QAR => storage_enums::Currency::QAR,
            api_enums::Currency::RUB => storage_enums::Currency::RUB,
            api_enums::Currency::SAR => storage_enums::Currency::SAR,
            api_enums::Currency::SCR => storage_enums::Currency::SCR,
            api_enums::Currency::SEK => storage_enums::Currency::SEK,
            api_enums::Currency::SGD => storage_enums::Currency::SGD,
            api_enums::Currency::SLL => storage_enums::Currency::SLL,
            api_enums::Currency::SOS => storage_enums::Currency::SOS,
            api_enums::Currency::SSP => storage_enums::Currency::SSP,
            api_enums::Currency::SVC => storage_enums::Currency::SVC,
            api_enums::Currency::SZL => storage_enums::Currency::SZL,
            api_enums::Currency::THB => storage_enums::Currency::THB,
            api_enums::Currency::TTD => storage_enums::Currency::TTD,
            api_enums::Currency::TWD => storage_enums::Currency::TWD,
            api_enums::Currency::TZS => storage_enums::Currency::TZS,
            api_enums::Currency::USD => storage_enums::Currency::USD,
            api_enums::Currency::UYU => storage_enums::Currency::UYU,
            api_enums::Currency::UZS => storage_enums::Currency::UZS,
            api_enums::Currency::YER => storage_enums::Currency::YER,
            api_enums::Currency::ZAR => storage_enums::Currency::ZAR,
        }
        .into()
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
