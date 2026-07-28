//! Curated allowlist of critical `api_flow`s surfaced in the organization user activity log.
//!
//! The `api_flow` column in the `api_events` ClickHouse table stores the `Display`
//! representation of `router_env::Flow` (e.g. `Flow::MerchantConnectorsCreate` is stored as
//! the string `"MerchantConnectorsCreate"`). We match against these strings directly
//! (rather than the `Flow` enum itself) since that is the exact representation persisted in
//! ClickHouse and read back out of it.

/// Flows considered "critical actions" for the purposes of the organization admin activity log.
/// Only these flows are surfaced, and only when a `user_id` is present on the event.
pub const CRITICAL_ACTION_FLOWS: &[&str] = &[
    "MerchantConnectorsCreate",
    "MerchantConnectorsUpdate",
    "MerchantConnectorsDelete",
    "ApiKeyCreate",
    "ApiKeyUpdate",
    "ApiKeyRevoke",
    "RoutingCreateConfig",
    "RoutingLinkConfig",
    "RoutingUnlinkConfig",
    "RoutingUpdateConfig",
    "RoutingUpdateDefaultConfig",
    "ProfileCreate",
    "ProfileUpdate",
    "ProfileDelete",
    "MerchantsAccountUpdate",
];

/// Maps a raw `api_flow` string (as stored in ClickHouse) to a short, human-readable label
/// suitable for display in the activity log. Falls back to the raw flow name when the flow
/// is not part of the curated allowlist.
pub fn action_label(api_flow: &str) -> String {
    match api_flow {
        "MerchantConnectorsCreate" => "Connector Created",
        "MerchantConnectorsUpdate" => "Connector Updated",
        "MerchantConnectorsDelete" => "Connector Deleted",
        "ApiKeyCreate" => "API Key Created",
        "ApiKeyUpdate" => "API Key Updated",
        "ApiKeyRevoke" => "API Key Revoked",
        "RoutingCreateConfig" => "Routing Config Created",
        "RoutingLinkConfig" => "Routing Config Activated",
        "RoutingUnlinkConfig" => "Routing Config Deactivated",
        "RoutingUpdateConfig" => "Routing Config Updated",
        "RoutingUpdateDefaultConfig" => "Default Routing Config Updated",
        "ProfileCreate" => "Business Profile Created",
        "ProfileUpdate" => "Business Profile Updated",
        "ProfileDelete" => "Business Profile Deleted",
        "MerchantsAccountUpdate" => "Merchant Account Updated",
        other => other,
    }
    .to_string()
}
