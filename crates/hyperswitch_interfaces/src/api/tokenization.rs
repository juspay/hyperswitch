//! //! Tokenization interface

/// Defines the behavior for tokenizable entities
pub trait Tokenizable {
    /// Sets the session token for the entity
    fn set_session_token(&mut self, token: Option<String>);
}
