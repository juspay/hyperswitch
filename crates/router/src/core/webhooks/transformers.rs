use std::convert::TryFrom;

use crate::{core::errors, types::storage::enums};

impl TryFrom<enums::IntentStatus> for enums::EventType {
    type Error = errors::ValidationError;

    fn try_from(value: enums::IntentStatus) -> Result<Self, Self::Error> {
        match value {
            enums::IntentStatus::Succeeded => Ok(Self::PaymentSucceeded),
            _ => Err(errors::ValidationError::IncorrectValueProvided {
                field_name: "intent_status",
            }),
        }
    }
}
