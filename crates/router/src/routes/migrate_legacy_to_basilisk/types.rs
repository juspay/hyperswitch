/// Request body for the `migrate_legacy_to_basilisk` route.
#[derive(Debug, serde::Serialize, Eq, PartialEq, serde::Deserialize)]
pub struct MigrateLegacyToBasiliskRequest {
    pub merchant_id: String,
    pub customer_id: String,
}
