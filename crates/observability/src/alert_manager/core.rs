pub mod config;
pub mod instances;
pub mod lifecycle;
pub mod mappers;
pub mod notifications;

use diesel_models::errors::DatabaseError;
use time::PrimitiveDateTime;

use crate::errors::ObservabilityError;

pub(super) fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}

pub(super) fn escalate(
    error: error_stack::Report<DatabaseError>,
    recognise: impl FnOnce(DatabaseError) -> Option<ObservabilityError>,
) -> error_stack::Report<ObservabilityError> {
    let context =
        recognise(*error.current_context()).unwrap_or(ObservabilityError::StorageUnavailable);

    error.change_context(context)
}

pub(super) fn unrecognised(_: DatabaseError) -> Option<ObservabilityError> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unrecognised_storage_error_is_storage_unavailable() {
        let report = error_stack::report!(DatabaseError::Others);

        assert!(matches!(
            escalate(report, unrecognised).current_context(),
            ObservabilityError::StorageUnavailable
        ));
    }

    #[test]
    fn a_recognised_error_keeps_its_own_context() {
        let report = error_stack::report!(DatabaseError::UniqueViolation);

        assert!(matches!(
            escalate(report, |context| matches!(
                context,
                DatabaseError::UniqueViolation
            )
            .then_some(ObservabilityError::InternalServerError))
            .current_context(),
            ObservabilityError::InternalServerError
        ));
    }
}
