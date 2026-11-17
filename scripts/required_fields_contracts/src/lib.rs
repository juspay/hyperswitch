//! Opt-in test companion. Scripted transport evidence never certifies a server.
pub mod api;
pub mod contract;
pub mod suite;
pub mod transport;

use std::fmt;

#[derive(Debug)]
pub struct ContractError;
impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("required-fields API contract refused or failed")
    }
}
impl std::error::Error for ContractError {}
pub type Result<T> = std::result::Result<T, error_stack::Report<ContractError>>;
pub fn refuse(message: impl Into<String>) -> error_stack::Report<ContractError> {
    error_stack::Report::new(ContractError).attach_printable(message.into())
}
