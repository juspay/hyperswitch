use api_models::enums as api_enums;

use crate::{core::errors, types::storage::enums as storage_enums};

impl From<api_enums::RoutingAlgorithm> for storage_enums::RoutingAlgorithm {
    fn from(algo: api_enums::RoutingAlgorithm) -> Self {
        match algo {
            api_enums::RoutingAlgorithm::RoundRobin => Self::RoundRobin,
            api_enums::RoutingAlgorithm::MaxConversion => Self::MaxConversion,
            api_enums::RoutingAlgorithm::MinCost => Self::MinCost,
            api_enums::RoutingAlgorithm::Custom => Self::Custom,
        }
    }
}

impl From<storage_enums::RoutingAlgorithm> for api_enums::RoutingAlgorithm {
    fn from(algo: storage_enums::RoutingAlgorithm) -> Self {
        match algo {
            storage_enums::RoutingAlgorithm::RoundRobin => Self::RoundRobin,
            storage_enums::RoutingAlgorithm::MaxConversion => Self::MaxConversion,
            storage_enums::RoutingAlgorithm::MinCost => Self::MinCost,
            storage_enums::RoutingAlgorithm::Custom => Self::Custom,
        }
    }
}

impl From<api_enums::ConnectorType> for storage_enums::ConnectorType {
    fn from(conn: api_enums::ConnectorType) -> Self {
        match conn {
            api_enums::ConnectorType::PaymentProcessor => Self::PaymentProcessor,
            api_enums::ConnectorType::PaymentVas => Self::PaymentVas,
            api_enums::ConnectorType::FinOperations => Self::FinOperations,
            api_enums::ConnectorType::FizOperations => Self::FizOperations,
            api_enums::ConnectorType::Networks => Self::Networks,
            api_enums::ConnectorType::BankingEntities => Self::BankingEntities,
            api_enums::ConnectorType::NonBankingFinance => Self::NonBankingFinance,
        }
    }
}

impl From<storage_enums::ConnectorType> for api_enums::ConnectorType {
    fn from(conn: storage_enums::ConnectorType) -> Self {
        match conn {
            storage_enums::ConnectorType::PaymentProcessor => Self::PaymentProcessor,
            storage_enums::ConnectorType::PaymentVas => Self::PaymentVas,
            storage_enums::ConnectorType::FinOperations => Self::FinOperations,
            storage_enums::ConnectorType::FizOperations => Self::FizOperations,
            storage_enums::ConnectorType::Networks => Self::Networks,
            storage_enums::ConnectorType::BankingEntities => Self::BankingEntities,
            storage_enums::ConnectorType::NonBankingFinance => Self::NonBankingFinance,
        }
    }
}

impl From<storage_enums::MandateStatus> for api_enums::MandateStatus {
    fn from(status: storage_enums::MandateStatus) -> Self {
        match status {
            storage_enums::MandateStatus::Active => Self::Active,
            storage_enums::MandateStatus::Inactive => Self::Inactive,
            storage_enums::MandateStatus::Pending => Self::Pending,
            storage_enums::MandateStatus::Revoked => Self::Revoked,
        }
    }
}

impl From<api_enums::PaymentMethodType> for storage_enums::PaymentMethodType {
    fn from(pm_type: api_enums::PaymentMethodType) -> Self {
        match pm_type {
            api_enums::PaymentMethodType::Card => Self::Card,
            api_enums::PaymentMethodType::PaymentContainer => Self::PaymentContainer,
            api_enums::PaymentMethodType::BankTransfer => Self::BankTransfer,
            api_enums::PaymentMethodType::BankDebit => Self::BankDebit,
            api_enums::PaymentMethodType::PayLater => Self::PayLater,
            api_enums::PaymentMethodType::Netbanking => Self::Netbanking,
            api_enums::PaymentMethodType::Upi => Self::Upi,
            api_enums::PaymentMethodType::OpenBanking => Self::OpenBanking,
            api_enums::PaymentMethodType::ConsumerFinance => Self::ConsumerFinance,
            api_enums::PaymentMethodType::Wallet => Self::Wallet,
            api_enums::PaymentMethodType::Klarna => Self::Klarna,
            api_enums::PaymentMethodType::Paypal => Self::Paypal,
        }
    }
}

impl From<storage_enums::PaymentMethodType> for api_enums::PaymentMethodType {
    fn from(pm_type: storage_enums::PaymentMethodType) -> Self {
        match pm_type {
            storage_enums::PaymentMethodType::Card => Self::Card,
            storage_enums::PaymentMethodType::PaymentContainer => Self::PaymentContainer,
            storage_enums::PaymentMethodType::BankTransfer => Self::BankTransfer,
            storage_enums::PaymentMethodType::BankDebit => Self::BankDebit,
            storage_enums::PaymentMethodType::PayLater => Self::PayLater,
            storage_enums::PaymentMethodType::Netbanking => Self::Netbanking,
            storage_enums::PaymentMethodType::Upi => Self::Upi,
            storage_enums::PaymentMethodType::OpenBanking => Self::OpenBanking,
            storage_enums::PaymentMethodType::ConsumerFinance => Self::ConsumerFinance,
            storage_enums::PaymentMethodType::Wallet => Self::Wallet,
            storage_enums::PaymentMethodType::Klarna => Self::Klarna,
            storage_enums::PaymentMethodType::Paypal => Self::Paypal,
        }
    }
}

impl From<api_enums::PaymentMethodSubType> for storage_enums::PaymentMethodSubType {
    fn from(pm_subtype: api_enums::PaymentMethodSubType) -> Self {
        match pm_subtype {
            api_enums::PaymentMethodSubType::Credit => Self::Credit,
            api_enums::PaymentMethodSubType::Debit => Self::Debit,
            api_enums::PaymentMethodSubType::UpiIntent => Self::UpiIntent,
            api_enums::PaymentMethodSubType::UpiCollect => Self::UpiCollect,
            api_enums::PaymentMethodSubType::CreditCardInstallments => Self::CreditCardInstallments,
            api_enums::PaymentMethodSubType::PayLaterInstallments => Self::PayLaterInstallments,
        }
    }
}

impl From<storage_enums::PaymentMethodSubType> for api_enums::PaymentMethodSubType {
    fn from(pm_subtype: storage_enums::PaymentMethodSubType) -> Self {
        match pm_subtype {
            storage_enums::PaymentMethodSubType::Credit => Self::Credit,
            storage_enums::PaymentMethodSubType::Debit => Self::Debit,
            storage_enums::PaymentMethodSubType::UpiIntent => Self::UpiIntent,
            storage_enums::PaymentMethodSubType::UpiCollect => Self::UpiCollect,
            storage_enums::PaymentMethodSubType::CreditCardInstallments => {
                Self::CreditCardInstallments
            }
            storage_enums::PaymentMethodSubType::PayLaterInstallments => Self::PayLaterInstallments,
        }
    }
}

impl From<storage_enums::PaymentMethodIssuerCode> for api_enums::PaymentMethodIssuerCode {
    fn from(issuer_code: storage_enums::PaymentMethodIssuerCode) -> Self {
        match issuer_code {
            storage_enums::PaymentMethodIssuerCode::JpHdfc => Self::JpHdfc,
            storage_enums::PaymentMethodIssuerCode::JpIcici => Self::JpIcici,
            storage_enums::PaymentMethodIssuerCode::JpGooglepay => Self::JpGooglepay,
            storage_enums::PaymentMethodIssuerCode::JpApplepay => Self::JpApplepay,
            storage_enums::PaymentMethodIssuerCode::JpPhonepay => Self::JpPhonepay,
            storage_enums::PaymentMethodIssuerCode::JpWechat => Self::JpWechat,
            storage_enums::PaymentMethodIssuerCode::JpSofort => Self::JpSofort,
            storage_enums::PaymentMethodIssuerCode::JpGiropay => Self::JpGiropay,
            storage_enums::PaymentMethodIssuerCode::JpSepa => Self::JpSepa,
            storage_enums::PaymentMethodIssuerCode::JpBacs => Self::JpBacs,
        }
    }
}

impl From<storage_enums::IntentStatus> for api_enums::IntentStatus {
    fn from(status: storage_enums::IntentStatus) -> Self {
        match status {
            storage_enums::IntentStatus::Succeeded => Self::Succeeded,
            storage_enums::IntentStatus::Failed => Self::Failed,
            storage_enums::IntentStatus::Cancelled => Self::Cancelled,
            storage_enums::IntentStatus::Processing => Self::Processing,
            storage_enums::IntentStatus::RequiresCustomerAction => Self::RequiresCustomerAction,
            storage_enums::IntentStatus::RequiresPaymentMethod => Self::RequiresPaymentMethod,
            storage_enums::IntentStatus::RequiresConfirmation => Self::RequiresConfirmation,
            storage_enums::IntentStatus::RequiresCapture => Self::RequiresCapture,
        }
    }
}

impl From<api_enums::IntentStatus> for storage_enums::IntentStatus {
    fn from(status: api_enums::IntentStatus) -> Self {
        match status {
            api_enums::IntentStatus::Succeeded => Self::Succeeded,
            api_enums::IntentStatus::Failed => Self::Failed,
            api_enums::IntentStatus::Cancelled => Self::Cancelled,
            api_enums::IntentStatus::Processing => Self::Processing,
            api_enums::IntentStatus::RequiresCustomerAction => Self::RequiresCustomerAction,
            api_enums::IntentStatus::RequiresPaymentMethod => Self::RequiresPaymentMethod,
            api_enums::IntentStatus::RequiresConfirmation => Self::RequiresConfirmation,
            api_enums::IntentStatus::RequiresCapture => Self::RequiresCapture,
        }
    }
}

impl From<storage_enums::AttemptStatus> for storage_enums::IntentStatus {
    fn from(s: storage_enums::AttemptStatus) -> Self {
        match s {
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
    }
}

impl TryFrom<api_enums::IntentStatus> for storage_enums::EventType {
    type Error = errors::ValidationError;

    fn try_from(value: api_enums::IntentStatus) -> Result<Self, Self::Error> {
        match value {
            api_enums::IntentStatus::Succeeded => Ok(Self::PaymentSucceeded),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "intent_status",
            }),
        }
    }
}

impl From<storage_enums::EventType> for api_enums::EventType {
    fn from(event_type: storage_enums::EventType) -> Self {
        match event_type {
            storage_enums::EventType::PaymentSucceeded => api_enums::EventType::PaymentSucceeded,
        }
    }
}

impl From<api_enums::FutureUsage> for storage_enums::FutureUsage {
    fn from(future_usage: api_enums::FutureUsage) -> Self {
        match future_usage {
            api_enums::FutureUsage::OnSession => Self::OnSession,
            api_enums::FutureUsage::OffSession => Self::OffSession,
        }
    }
}

impl From<storage_enums::FutureUsage> for api_enums::FutureUsage {
    fn from(future_usage: storage_enums::FutureUsage) -> Self {
        match future_usage {
            storage_enums::FutureUsage::OnSession => Self::OnSession,
            storage_enums::FutureUsage::OffSession => Self::OffSession,
        }
    }
}

impl From<storage_enums::RefundStatus> for api_enums::RefundStatus {
    fn from(status: storage_enums::RefundStatus) -> Self {
        match status {
            storage_enums::RefundStatus::Failure => Self::Failure,
            storage_enums::RefundStatus::ManualReview => Self::ManualReview,
            storage_enums::RefundStatus::Pending => Self::Pending,
            storage_enums::RefundStatus::Success => Self::Success,
            storage_enums::RefundStatus::TransactionFailure => Self::TransactionFailure,
        }
    }
}

impl From<api_enums::CaptureMethod> for storage_enums::CaptureMethod {
    fn from(capture_method: api_enums::CaptureMethod) -> Self {
        match capture_method {
            api_enums::CaptureMethod::Automatic => Self::Automatic,
            api_enums::CaptureMethod::Manual => Self::Manual,
            api_enums::CaptureMethod::ManualMultiple => Self::ManualMultiple,
            api_enums::CaptureMethod::Scheduled => Self::Scheduled,
        }
    }
}

impl From<storage_enums::CaptureMethod> for api_enums::CaptureMethod {
    fn from(capture_method: storage_enums::CaptureMethod) -> Self {
        match capture_method {
            storage_enums::CaptureMethod::Automatic => Self::Automatic,
            storage_enums::CaptureMethod::Manual => Self::Manual,
            storage_enums::CaptureMethod::ManualMultiple => Self::ManualMultiple,
            storage_enums::CaptureMethod::Scheduled => Self::Scheduled,
        }
    }
}

impl From<api_enums::AuthenticationType> for storage_enums::AuthenticationType {
    fn from(auth_type: api_enums::AuthenticationType) -> Self {
        match auth_type {
            api_enums::AuthenticationType::ThreeDs => Self::ThreeDs,
            api_enums::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}

impl From<storage_enums::AuthenticationType> for api_enums::AuthenticationType {
    fn from(auth_type: storage_enums::AuthenticationType) -> Self {
        match auth_type {
            storage_enums::AuthenticationType::ThreeDs => Self::ThreeDs,
            storage_enums::AuthenticationType::NoThreeDs => Self::NoThreeDs,
        }
    }
}

impl From<api_enums::Currency> for storage_enums::Currency {
    fn from(currency: api_enums::Currency) -> Self {
        match currency {
            api_enums::Currency::AED => Self::AED,
            api_enums::Currency::ALL => Self::ALL,
            api_enums::Currency::AMD => Self::AMD,
            api_enums::Currency::ARS => Self::ARS,
            api_enums::Currency::AUD => Self::AUD,
            api_enums::Currency::AWG => Self::AWG,
            api_enums::Currency::AZN => Self::AZN,
            api_enums::Currency::BBD => Self::BBD,
            api_enums::Currency::BDT => Self::BDT,
            api_enums::Currency::BHD => Self::BHD,
            api_enums::Currency::BMD => Self::BMD,
            api_enums::Currency::BND => Self::BND,
            api_enums::Currency::BOB => Self::BOB,
            api_enums::Currency::BRL => Self::BRL,
            api_enums::Currency::BSD => Self::BSD,
            api_enums::Currency::BWP => Self::BWP,
            api_enums::Currency::BZD => Self::BZD,
            api_enums::Currency::CAD => Self::CAD,
            api_enums::Currency::CHF => Self::CHF,
            api_enums::Currency::CNY => Self::CNY,
            api_enums::Currency::COP => Self::COP,
            api_enums::Currency::CRC => Self::CRC,
            api_enums::Currency::CUP => Self::CUP,
            api_enums::Currency::CZK => Self::CZK,
            api_enums::Currency::DKK => Self::DKK,
            api_enums::Currency::DOP => Self::DOP,
            api_enums::Currency::DZD => Self::DZD,
            api_enums::Currency::EGP => Self::EGP,
            api_enums::Currency::ETB => Self::ETB,
            api_enums::Currency::EUR => Self::EUR,
            api_enums::Currency::FJD => Self::FJD,
            api_enums::Currency::GBP => Self::GBP,
            api_enums::Currency::GHS => Self::GHS,
            api_enums::Currency::GIP => Self::GIP,
            api_enums::Currency::GMD => Self::GMD,
            api_enums::Currency::GTQ => Self::GTQ,
            api_enums::Currency::GYD => Self::GYD,
            api_enums::Currency::HKD => Self::HKD,
            api_enums::Currency::HNL => Self::HNL,
            api_enums::Currency::HRK => Self::HRK,
            api_enums::Currency::HTG => Self::HTG,
            api_enums::Currency::HUF => Self::HUF,
            api_enums::Currency::IDR => Self::IDR,
            api_enums::Currency::ILS => Self::ILS,
            api_enums::Currency::INR => Self::INR,
            api_enums::Currency::JMD => Self::JMD,
            api_enums::Currency::JOD => Self::JOD,
            api_enums::Currency::JPY => Self::JPY,
            api_enums::Currency::KES => Self::KES,
            api_enums::Currency::KGS => Self::KGS,
            api_enums::Currency::KHR => Self::KHR,
            api_enums::Currency::KRW => Self::KRW,
            api_enums::Currency::KWD => Self::KWD,
            api_enums::Currency::KYD => Self::KYD,
            api_enums::Currency::KZT => Self::KZT,
            api_enums::Currency::LAK => Self::LAK,
            api_enums::Currency::LBP => Self::LBP,
            api_enums::Currency::LKR => Self::LKR,
            api_enums::Currency::LRD => Self::LRD,
            api_enums::Currency::LSL => Self::LSL,
            api_enums::Currency::MAD => Self::MAD,
            api_enums::Currency::MDL => Self::MDL,
            api_enums::Currency::MKD => Self::MKD,
            api_enums::Currency::MMK => Self::MMK,
            api_enums::Currency::MNT => Self::MNT,
            api_enums::Currency::MOP => Self::MOP,
            api_enums::Currency::MUR => Self::MUR,
            api_enums::Currency::MVR => Self::MVR,
            api_enums::Currency::MWK => Self::MWK,
            api_enums::Currency::MXN => Self::MXN,
            api_enums::Currency::MYR => Self::MYR,
            api_enums::Currency::NAD => Self::NAD,
            api_enums::Currency::NGN => Self::NGN,
            api_enums::Currency::NIO => Self::NIO,
            api_enums::Currency::NOK => Self::NOK,
            api_enums::Currency::NPR => Self::NPR,
            api_enums::Currency::NZD => Self::NZD,
            api_enums::Currency::OMR => Self::OMR,
            api_enums::Currency::PEN => Self::PEN,
            api_enums::Currency::PGK => Self::PGK,
            api_enums::Currency::PHP => Self::PHP,
            api_enums::Currency::PKR => Self::PKR,
            api_enums::Currency::PLN => Self::PLN,
            api_enums::Currency::QAR => Self::QAR,
            api_enums::Currency::RUB => Self::RUB,
            api_enums::Currency::SAR => Self::SAR,
            api_enums::Currency::SCR => Self::SCR,
            api_enums::Currency::SEK => Self::SEK,
            api_enums::Currency::SGD => Self::SGD,
            api_enums::Currency::SLL => Self::SLL,
            api_enums::Currency::SOS => Self::SOS,
            api_enums::Currency::SSP => Self::SSP,
            api_enums::Currency::SVC => Self::SVC,
            api_enums::Currency::SZL => Self::SZL,
            api_enums::Currency::THB => Self::THB,
            api_enums::Currency::TTD => Self::TTD,
            api_enums::Currency::TWD => Self::TWD,
            api_enums::Currency::TZS => Self::TZS,
            api_enums::Currency::USD => Self::USD,
            api_enums::Currency::UYU => Self::UYU,
            api_enums::Currency::UZS => Self::UZS,
            api_enums::Currency::YER => Self::YER,
            api_enums::Currency::ZAR => Self::ZAR,
        }
    }
}
