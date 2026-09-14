pub mod config;
pub mod lifecycle;
pub mod mappers;
pub mod notifications;

use diesel_models::errors::DatabaseError;

use crate::errors::ObservabilityError;

pub(super) fn escalate(
    error: error_stack::Report<DatabaseError>,
    recognise: impl FnOnce(DatabaseError) -> Option<ObservabilityError>,
) -> error_stack::Report<ObservabilityError> {
    let database_error = *error.current_context();
    let context = recognise(database_error).unwrap_or(match database_error {
        DatabaseError::DatabaseConnectionError => ObservabilityError::StorageUnavailable,
        _ => ObservabilityError::InternalServerError,
    });

    error.change_context(context)
}

pub(super) fn unrecognised(_: DatabaseError) -> Option<ObservabilityError> {
    None
}
