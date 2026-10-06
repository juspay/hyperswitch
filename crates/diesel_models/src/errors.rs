// Deja replay reconstructs a recorded DB error as the SAME typed context the
// recording threw ("recording threw ⇒ replay throws"); the serde derives give
// the fieldless variants a lossless wire form (the bare variant-name string).
#[cfg_attr(feature = "deja", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("An error occurred when obtaining database connection")]
    DatabaseConnectionError,
    #[error("The requested resource was not found in the database")]
    NotFound,
    #[error("A unique constraint violation occurred")]
    UniqueViolation,
    #[error("No fields were provided to be updated")]
    NoFieldsToUpdate,
    #[error("An error occurred when generating SQL query")]
    QueryGenerationFailed,
    #[error("An unknown error occurred")]
    Others,
}

impl common_utils::errors::ErrorSwitchFrom<diesel::result::Error> for DatabaseError {
    fn switch_from(error: &diesel::result::Error) -> Self {
        match *error {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            ) => Self::UniqueViolation,
            diesel::result::Error::NotFound => Self::NotFound,
            diesel::result::Error::QueryBuilderError(_) => Self::QueryGenerationFailed,
            _ => Self::Others,
        }
    }
}
