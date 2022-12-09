use crate::{
    core::errors,
    types::{api::enums as api_enums, storage::enums as storage_enums},
};

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

impl From<api_enums::AttemptStatus> for api_enums::IntentStatus {
    fn from(s: api_enums::AttemptStatus) -> Self {
        match s {
            api_enums::AttemptStatus::Charged | api_enums::AttemptStatus::AutoRefunded => {
                api_enums::IntentStatus::Succeeded
            }

            api_enums::AttemptStatus::ConfirmationAwaited => {
                api_enums::IntentStatus::RequiresConfirmation
            }
            api_enums::AttemptStatus::PaymentMethodAwaited => {
                api_enums::IntentStatus::RequiresPaymentMethod
            }

            api_enums::AttemptStatus::Authorized => api_enums::IntentStatus::RequiresCapture,
            api_enums::AttemptStatus::PendingVbv => api_enums::IntentStatus::RequiresCustomerAction,

            api_enums::AttemptStatus::PartialCharged
            | api_enums::AttemptStatus::Started
            | api_enums::AttemptStatus::VbvSuccessful
            | api_enums::AttemptStatus::Authorizing
            | api_enums::AttemptStatus::CodInitiated
            | api_enums::AttemptStatus::VoidInitiated
            | api_enums::AttemptStatus::CaptureInitiated
            | api_enums::AttemptStatus::Pending => api_enums::IntentStatus::Processing,

            api_enums::AttemptStatus::AuthenticationFailed
            | api_enums::AttemptStatus::AuthorizationFailed
            | api_enums::AttemptStatus::VoidFailed
            | api_enums::AttemptStatus::JuspayDeclined
            | api_enums::AttemptStatus::CaptureFailed
            | api_enums::AttemptStatus::Failure => api_enums::IntentStatus::Failed,
            api_enums::AttemptStatus::Voided => api_enums::IntentStatus::Cancelled,
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
