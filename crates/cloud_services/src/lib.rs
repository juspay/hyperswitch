//! Cloud provider clients shared across Hyperswitch services.
//!
//! Each backend sits behind its own Cargo feature, so a service only compiles the providers it
//! uses:
//!
//! | Feature              | Module           | Backend                                           |
//! |----------------------|------------------|---------------------------------------------------|
//! | `s3`                 | [`storage::s3`]  | AWS S3                                            |
//! | `oci_object_storage` | [`storage::oci`] | OCI Object Storage, through its S3-compatible API |
//! | `aws_kms`            | [`kms::aws`]     | AWS KMS                                           |
//! | `gcp_kms`            | [`kms::gcp`]     | GCP Cloud KMS                                     |
//! | `oci_kms`            | [`kms::oci`]     | OCI Vault KMS                                     |
//!
//! Errors are plain [`std::error::Error`] types so callers can wrap them in whatever error
//! handling they use.

#![warn(missing_docs, missing_debug_implementations)]

pub mod kms;
pub mod storage;
