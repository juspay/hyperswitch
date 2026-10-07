//! Cloud provider clients shared across Hyperswitch services.
//!
//! Each backend sits behind its own Cargo feature, so a service only compiles the providers it
//! uses:
//!
//! | Feature   | Module                | Backend                                                  |
//! |-----------|-----------------------|----------------------------------------------------------|
//! | `s3`      | [`storage::s3`]       | AWS S3 and S3-compatible stores (OCI Object Storage, ...) |
//! | `oci_kms` | [`kms::oci`]          | OCI Vault KMS                                            |
//!
//! Errors are plain [`std::error::Error`] types so callers can wrap them in whatever error
//! handling they use.

#![warn(missing_docs, missing_debug_implementations)]

pub mod kms;
pub mod storage;
