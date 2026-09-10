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

/// Which integration a merchant builds its checkout with, and therefore which
/// `X-Integration-Type` header values its payment requests may carry.
///
/// `client_and_server` is the default: an account that never set this keeps accepting either
/// header value, exactly as it did before the setting existed.
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
pub enum MerchantIntegrationType {
    /// Only client integrations: the header must be `client` or absent.
    Client,
    /// Only server integrations: the header must be `server`.
    Server,
    /// Either integration: any header value is accepted.
    #[default]
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
