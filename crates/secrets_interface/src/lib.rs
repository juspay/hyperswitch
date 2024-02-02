//! Secrets management interface

#![warn(missing_docs, missing_debug_implementations)]

/// Module for managing encryption and decryption of application secrets
pub mod secrets_management;

/// Module containing trait for config decryption
pub mod decryption;

/// Module to manage encrypted and decrypted states for a given type.
pub mod type_state;
