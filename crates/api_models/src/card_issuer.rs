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

#[derive(Debug, serde::Serialize)]
pub struct CardIssuerDeleteRequest {
    pub id: CardIssuerId,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct CardIssuerDeleteResponse {
    #[schema(value_type = String)]
    pub id: CardIssuerId,
    #[schema(example = true)]
    pub deleted: bool,
}

impl ApiEventMetric for CardIssuerUpdateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerDeleteRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::CardIssuers)
    }
}
impl ApiEventMetric for CardIssuerDeleteResponse {
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
