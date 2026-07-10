use std::fmt::Debug;

use common_utils::events::ApiEventMetric;
use utoipa::ToSchema;

use crate::enums;

/// Accepts either a JSON array of co-badged card networks (from the create/update JSON APIs) or a
/// comma-separated list of network codes within a single CSV cell (from the bulk migration upload),
/// e.g. `STAR,PULSE`.
fn deserialize_co_badged_card_networks<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<enums::CoBadgedCardNetwork>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct CoBadgedCardNetworksVisitor;

    impl<'de> serde::de::Visitor<'de> for CoBadgedCardNetworksVisitor {
        type Value = Option<Vec<enums::CoBadgedCardNetwork>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(
                "a JSON array of card networks, or a comma-separated list of card networks",
            )
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if value.trim().is_empty() {
                return Ok(None);
            }
            let networks = value
                .split(',')
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .map(|token| {
                    serde_json::from_str::<enums::CoBadgedCardNetwork>(&format!("\"{token}\""))
                        .map_err(|_| {
                            E::custom(format!(
                                "invalid card network in co_badged_card_networks: {token}"
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(networks))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut networks = Vec::new();
            while let Some(network) = seq.next_element::<enums::CoBadgedCardNetwork>()? {
                networks.push(network);
            }
            Ok(Some(networks))
        }
    }

    deserializer.deserialize_any(CoBadgedCardNetworksVisitor)
}

#[derive(serde::Deserialize, ToSchema)]
pub struct CardsInfoRequestParams {
    #[schema(example = "pay_OSERgeV9qAy7tlK7aKpc_secret_TuDUoh11Msxh12sXn3Yp")]
    pub client_secret: Option<String>,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CardsInfoRequest {
    pub client_secret: Option<String>,
    pub card_iin: String,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoResponse {
    #[schema(example = "374431")]
    pub card_iin: String,
    #[schema(example = "AMEX")]
    pub card_issuer: Option<String>,
    #[schema(example = "AMEX")]
    pub card_network: Option<String>,
    #[schema(example = "CREDIT")]
    pub card_type: Option<String>,
    #[schema(example = "CLASSIC")]
    pub card_sub_type: Option<String>,
    #[schema(example = "INDIA")]
    pub card_issuing_country: Option<String>,
    #[schema(example = "CREDIT")]
    pub funding_source: Option<String>,
    #[schema(example = "PAN")]
    pub card_iin_type: Option<String>,
    #[schema(example = false)]
    pub virtual_card: Option<bool>,
    #[schema(example = false)]
    pub gambling_blocked: Option<bool>,
    #[schema(example = json!(["VISA", "RUPAY"]))]
    pub co_badged_card_networks: Option<Vec<String>>,
}

#[derive(serde::Serialize, Debug, ToSchema)]
pub struct CardInfoMigrateResponseRecord {
    pub card_iin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub funding_source: Option<String>,
    pub card_iin_type: Option<String>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    pub co_badged_card_networks: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoCreateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub funding_source: Option<enums::FundingSource>,
    pub card_iin_type: Option<enums::PanOrToken>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_co_badged_card_networks")]
    #[schema(value_type = Option<Vec<CoBadgedCardNetwork>>, example = json!(["RUPAY", "STAR"]))]
    pub co_badged_card_networks: Option<Vec<enums::CoBadgedCardNetwork>>,
}

impl ApiEventMetric for CardInfoCreateRequest {}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardInfoUpdateRequest {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated_provider: Option<String>,
    pub funding_source: Option<enums::FundingSource>,
    pub card_iin_type: Option<enums::PanOrToken>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_co_badged_card_networks")]
    #[schema(value_type = Option<Vec<CoBadgedCardNetwork>>, example = json!(["RUPAY", "STAR"]))]
    pub co_badged_card_networks: Option<Vec<enums::CoBadgedCardNetwork>>,
    pub line_number: Option<i64>,
}

impl ApiEventMetric for CardInfoUpdateRequest {}

#[derive(Debug, Default, serde::Serialize)]
pub enum CardInfoMigrationStatus {
    Success,
    #[default]
    Failed,
}
#[derive(Debug, Default, serde::Serialize)]
pub struct CardInfoMigrationResponse {
    pub line_number: Option<i64>,
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<String>,
    pub card_type: Option<String>,
    pub card_sub_type: Option<String>,
    pub card_issuing_country: Option<String>,
    pub funding_source: Option<String>,
    pub card_iin_type: Option<String>,
    pub virtual_card: Option<bool>,
    pub gambling_blocked: Option<bool>,
    pub co_badged_card_networks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_error: Option<String>,
    pub migration_status: CardInfoMigrationStatus,
}
impl ApiEventMetric for CardInfoMigrationResponse {}

type CardInfoMigrationResponseType = (
    Result<CardInfoMigrateResponseRecord, String>,
    CardInfoUpdateRequest,
);

impl From<CardInfoMigrationResponseType> for CardInfoMigrationResponse {
    fn from((response, record): CardInfoMigrationResponseType) -> Self {
        match response {
            Ok(res) => Self {
                card_iin: record.card_iin,
                line_number: record.line_number,
                card_issuer: res.card_issuer,
                card_network: res.card_network,
                card_type: res.card_type,
                card_sub_type: res.card_sub_type,
                card_issuing_country: res.card_issuing_country,
                funding_source: res.funding_source,
                card_iin_type: res.card_iin_type,
                virtual_card: res.virtual_card,
                gambling_blocked: res.gambling_blocked,
                co_badged_card_networks: res.co_badged_card_networks,
                migration_status: CardInfoMigrationStatus::Success,
                migration_error: None,
            },
            Err(e) => Self {
                card_iin: record.card_iin,
                migration_status: CardInfoMigrationStatus::Failed,
                migration_error: Some(e),
                line_number: record.line_number,
                ..Self::default()
            },
        }
    }
}
