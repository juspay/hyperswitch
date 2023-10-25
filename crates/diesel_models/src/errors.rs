#[derive(Debug, thiserror::Error)]
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
    #[error("DB transaction failure")]
    TransactionFailed(String),
    // InsertFailed,
    #[error("An unknown error occurred")]
    Others,
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        Self::TransactionFailed(err.to_string())
    }
}
