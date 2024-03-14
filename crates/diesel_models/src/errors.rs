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
    #[error("An error occurred when generating typed SQL query")]
    QueryGenerationFailed,
    // InsertFailed,
    #[error("An unknown error occurred")]
    Others,
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(error: diesel::result::Error) -> Self {
        match error {
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
