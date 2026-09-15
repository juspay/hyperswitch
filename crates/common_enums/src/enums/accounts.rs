use utoipa::ToSchema;
#[derive(
    Copy,
    Default,
    Clone,
    Debug,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
    Hash,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MerchantProductType {
    #[default]
    Orchestration,
    Vault,
    Recon,
    Recovery,
    CostObservability,
    DynamicRouting,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MerchantAccountType {
    #[default]
    Standard,
    Platform,
    Connected,
}

/// Which `X-Integration-Type` header values a merchant may send, from Superposition
/// (`system.payment_integration_type`). Defaults to `client`: the server shape is opt-in.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MerchantIntegrationType {
    /// The default: the header must be `client` or absent.
    #[default]
    Client,
    /// The header must be `server`.
    Server,
    /// Any header value is accepted.
    ClientAndServer,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum OrganizationType {
    #[default]
    Standard,
    Platform,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    strum::Display,
    strum::EnumString,
    ToSchema,
)]
#[router_derive::diesel_enum(storage_type = "text")]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MerchantAccountRequestType {
    #[default]
    Standard,
    Connected,
}

impl From<MerchantAccountRequestType> for MerchantAccountType {
    fn from(value: MerchantAccountRequestType) -> Self {
        match value {
            MerchantAccountRequestType::Standard => Self::Standard,
            MerchantAccountRequestType::Connected => Self::Connected,
        }
    }
}
