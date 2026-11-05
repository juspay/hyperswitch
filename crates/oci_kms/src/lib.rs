#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod client;
mod config;
mod config_file;
mod credentials;
mod environment;
mod error;
mod signing;
mod transport;
mod workload_identity;

pub use client::{DataKey, OciKmsClient};
pub use config::OciKmsConfig;
pub use environment::{Environment, SystemEnvironment};
pub use error::OciKmsError;
