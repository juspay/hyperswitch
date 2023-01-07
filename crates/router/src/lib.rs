#![forbid(unsafe_code)]
#![recursion_limit = "256"]

#[cfg(feature = "stripe")]
pub mod compatibility;
pub mod configs;
pub mod connection;
pub mod connector;
pub(crate) mod consts;
pub mod core;
pub mod cors;
pub mod db;
pub mod env;
pub(crate) mod macros;
pub mod routes;
pub mod scheduler;

mod middleware;
pub mod services;
pub mod types;
pub mod utils;

pub mod olap_lib;
pub mod oltp_lib;



pub use self::env::logger;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Header Constants
pub mod headers {
    pub const X_API_KEY: &str = "X-API-KEY";
    pub const CONTENT_TYPE: &str = "Content-Type";
    pub const X_ROUTER: &str = "X-router";
    pub const AUTHORIZATION: &str = "Authorization";
    pub const ACCEPT: &str = "Accept";
    pub const X_API_VERSION: &str = "X-ApiVersion";
}

pub mod pii {
    //! Personal Identifiable Information protection.

    pub(crate) use common_utils::pii::{CardNumber, Email};
    #[doc(inline)]
    pub use masking::*;
}
