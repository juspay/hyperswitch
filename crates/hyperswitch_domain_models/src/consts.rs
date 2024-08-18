//! Constants that are used in the domain models.

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "customer_v2"),
    not(feature = "merchant_account_v2"),
    not(feature = "merchant_connector_account_v2")
))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V1;

#[cfg(all(
    feature = "v2",
    any(
        feature = "customer_v2",
        feature = "merchant_account_v2",
        feature = "merchant_connector_account_v2"
    )
))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V2;
