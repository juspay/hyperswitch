use common_utils::{errors, id_type};
use diesel_models::card_issuer;

#[async_trait::async_trait]
pub trait CardIssuersInterface {
    type Error;
    async fn insert_card_issuer(
        &self,
        new: card_issuer::NewCardIssuer,
    ) -> errors::CustomResult<card_issuer::CardIssuer, Self::Error>;

    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: card_issuer::UpdateCardIssuer,
    ) -> errors::CustomResult<card_issuer::CardIssuer, Self::Error>;

    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> errors::CustomResult<Vec<card_issuer::CardIssuer>, Self::Error>;

    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> errors::CustomResult<Vec<card_issuer::CardIssuer>, Self::Error>;
}
