//! Curated allowlist of critical `api_flow`s surfaced in the organization user activity log.
//!
//! The `api_flow` column in the `api_events` ClickHouse table stores the `Display`
//! representation of `router_env::Flow` (e.g. `Flow::MerchantConnectorsCreate` is stored as
//! the string `"MerchantConnectorsCreate"`). We match against these strings directly
//! (rather than the `Flow` enum itself) since that is the exact representation persisted in
//! ClickHouse and read back out of it.
//!
//! Flow name and display label are kept as a single `(flow, label)` list, rather than two
//! separately maintained lists, so adding a new critical action can't drift out of sync
//! between the filter allowlist and the display label.

/// (raw `api_flow` string, human-readable label) pairs for flows considered "critical
/// actions" for the purposes of the organization admin activity log. Only these flows are
/// surfaced, and only when a `user_id` is present on the event.
const CRITICAL_ACTIONS: &[(&str, &str)] = &[
    ("MerchantConnectorsCreate", "Connector Created"),
    ("MerchantConnectorsUpdate", "Connector Updated"),
    ("MerchantConnectorsDelete", "Connector Deleted"),
    ("ApiKeyCreate", "API Key Created"),
    ("ApiKeyUpdate", "API Key Updated"),
    ("ApiKeyRevoke", "API Key Revoked"),
    ("RoutingCreateConfig", "Routing Config Created"),
    ("RoutingLinkConfig", "Routing Config Activated"),
    ("RoutingUnlinkConfig", "Routing Config Deactivated"),
    ("RoutingUpdateConfig", "Routing Config Updated"),
    ("RoutingUpdateDefaultConfig", "Default Routing Config Updated"),
    ("ProfileCreate", "Business Profile Created"),
    ("ProfileUpdate", "Business Profile Updated"),
    ("ProfileDelete", "Business Profile Deleted"),
    ("MerchantsAccountUpdate", "Merchant Account Updated"),
];

/// The raw `api_flow` strings from [`CRITICAL_ACTIONS`], for use in the ClickHouse
/// `api_flow IN (...)` filter.
pub fn critical_action_flows() -> Vec<&'static str> {
    CRITICAL_ACTIONS.iter().map(|(flow, _)| *flow).collect()
}

/// Maps a raw `api_flow` string (as stored in ClickHouse) to a short, human-readable label
/// suitable for display in the activity log. Falls back to the raw flow name when the flow
/// is not part of the curated allowlist.
pub fn action_label(api_flow: &str) -> String {
    CRITICAL_ACTIONS
        .iter()
        .find_map(|(flow, label)| (*flow == api_flow).then_some(*label))
        .unwrap_or(api_flow)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_critical_action_flow_has_a_label() {
        for (flow, _) in CRITICAL_ACTIONS {
            assert_ne!(
                action_label(flow),
                *flow,
                "flow {flow} is missing a real display label (falling back to the raw flow name)"
            );
        }
    }

    #[test]
    fn unknown_flow_falls_back_to_the_raw_flow_name() {
        assert_eq!(action_label("SomeUnrelatedFlow"), "SomeUnrelatedFlow");
    }

    #[test]
    fn critical_action_flows_matches_the_curated_list() {
        assert_eq!(critical_action_flows().len(), CRITICAL_ACTIONS.len());
        assert!(critical_action_flows().contains(&"MerchantConnectorsUpdate"));
    }
}
