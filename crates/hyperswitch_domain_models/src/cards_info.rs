use common_utils::errors::CustomResult;
use diesel_models::cards_info::{CardInfo, UpdateCardInfo};

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    type Error;
    async fn get_card_info(&self, _card_iin: &str) -> CustomResult<Option<CardInfo>, Self::Error>;
    async fn add_card_info(&self, data: CardInfo) -> CustomResult<CardInfo, Self::Error>;
    async fn update_card_info(
        &self,
        card_iin: String,
        data: UpdateCardInfo,
    ) -> CustomResult<CardInfo, Self::Error>;
}
