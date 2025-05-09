use common_utils::errors;
use diesel_models::cards_info;

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    type Error;
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> errors::CustomResult<Option<cards_info::CardInfo>, Self::Error>;
    async fn add_card_info(
        &self,
        data: cards_info::CardInfo,
    ) -> errors::CustomResult<cards_info::CardInfo, Self::Error>;
    async fn update_card_info(
        &self,
        card_iin: String,
        data: cards_info::UpdateCardInfo,
    ) -> errors::CustomResult<cards_info::CardInfo, Self::Error>;
}
