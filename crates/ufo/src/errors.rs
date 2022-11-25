//!
//! errors and error specific types for universal use

/// Custom Result
/// A custom datatype that wraps the error varient <E> into a report, allowing
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
        /// Error variant $name
        /// Custom error variant for performing $arg specific
        #[derive(Debug)]
        pub struct $name;

        impl_error_display!($name, $arg);

        impl std::error::Error for $name {}
    };
}

impl_error_type!(ParsingError, "Parsing error");
