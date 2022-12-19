//! Errors and error specific types for universal use

/// Custom Result
/// A custom datatype that wraps the error variant <E> into a report, allowing
/// error_stack::Report<E> specific extendability
///
/// Effectively, equivalent to `Result<T, error_stack::Report<E>>`
///
pub type CustomResult<T, E> = error_stack::Result<T, E>;

macro_rules! impl_error_display {
    ($st: ident, $arg: tt) => {
        impl std::fmt::Display for $st {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.write_str(&format!(
                    "{{ error_type: {:?}, error_description: {} }}",
                    self, $arg
                ))
            }
        }
    };
}

macro_rules! impl_error_type {
    ($name: ident, $arg: tt) => {
        #[doc = ""]
        #[doc = stringify!(Error variant $name)]
        #[doc = stringify!(Custom error variant for $arg)]
        #[doc = ""]
        #[derive(Debug)]
        pub struct $name;

        impl_error_display!($name, $arg);

        impl std::error::Error for $name {}
    };
}

impl_error_type!(ParsingError, "Parsing error");

/// Validation errors.
#[allow(missing_docs)] // Only to prevent warnings about struct fields not being documented
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// The provided input is missing a required field.
    #[error("Missing required field: {field_name}")]
    MissingRequiredField { field_name: String },

    /// An incorrect value was provided for the field specified by `field_name`.
    #[error("Incorrect value provided for field: {field_name}")]
    IncorrectValueProvided { field_name: &'static str },

    /// An invalid input was provided.
    #[error("{message}")]
    InvalidValue { message: String },
}

/// Cryptograpic algorithm errors
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// The cryptographic algorithm was unable to encode the message
    #[error("Failed to encode given message")]
    EncodingFailed,
    /// The cryptographic algorithm was unable to decode the message
    #[error("Failed to decode given message")]
    DecodingFailed,
    /// The cryptographic algorithm was unable to sign the message
    #[error("Failed to sign message")]
    MessageSigningFailed,
    /// The cryptographic algorithm was unable to verify the given signature
    #[error("Failed to verify signature")]
    SignatureVerificationFailed,
}
