//! Constants that are used in the domain models.

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V1;

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V2;

/// Length of the unique reference ID generated for connector mandate requests
pub const CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH: usize = 18;
