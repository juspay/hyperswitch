use common_utils::{events::ApiEventMetric, id_type::CardIssuerId, new_type::CardIssuerName};
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardIssuerRequest {
    /// The name of the card issuer to add
    #[schema(example = "STATE BANK OF INDIA", value_type = String)]
    pub issuer_name: CardIssuerName,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CardIssuerResponse {
    #[schema(value_type = String)]
    pub id: CardIssuerId,
    #[schema(value_type = String)]
    pub issuer_name: CardIssuerName,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardIssuerListQuery {
    /// Optional search term to filter issuers by name (case-insensitive prefix match)
    #[schema(example = "hdfc")]
    pub query: Option<String>,
    /// Maximum number of results to return (default: 30, max: 255)
    #[schema(example = 30, default = 30, maximum = 255, value_type = u8)]
    #[serde(default = "default_card_issuer_list_limit")]
    pub limit: u8,
}

fn default_card_issuer_list_limit() -> u8 {
    common_utils::consts::DEFAULT_CARD_ISSUER_LIST_LIMIT
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CardIssuerListResponse {
    pub issuers: Vec<CardIssuerResponse>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardIssuerUpdateRequest {
    /// The new name for the card issuer
    #[schema(example = "STATE BANK OF INDIA UPDATED", value_type = String)]
    pub issuer_name: CardIssuerName,
}

impl ApiEventMetric for CardIssuerUpdateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerListResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerListQuery {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
