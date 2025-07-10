use common_utils::errors;
// use diesel_models::cards_info;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CardInfo {
    pub card_iin: String,
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub date_created: PrimitiveDateTime,
    pub last_updated: Option<PrimitiveDateTime>,
    pub last_updated_provider: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, router_derive::DebugAsDisplay, serde::Deserialize)]
pub struct UpdateCardInfo {
    pub card_issuer: Option<String>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_type: Option<String>,
    pub card_subtype: Option<String>,
    pub card_issuing_country: Option<String>,
    pub bank_code_id: Option<String>,
    pub bank_code: Option<String>,
    pub country_code: Option<String>,
    pub last_updated: Option<PrimitiveDateTime>,
    pub last_updated_provider: Option<String>,
}

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    type Error;
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> errors::CustomResult<Option<CardInfo>, Self::Error>;
    async fn add_card_info(&self, data: CardInfo) -> errors::CustomResult<CardInfo, Self::Error>;
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> errors::CustomResult<CardInfo, Self::Error>;
}
