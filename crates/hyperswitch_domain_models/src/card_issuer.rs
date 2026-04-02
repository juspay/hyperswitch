use common_utils::{errors, id_type};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CardIssuer {
    pub id: id_type::CardIssuerId,
    pub issuer_name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateCardIssuer {
    pub issuer_name: String,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NewCardIssuer {
    pub id: id_type::CardIssuerId,
    pub issuer_name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
pub trait CardIssuersInterface {
    type Error;
    async fn insert_card_issuer(
        &self,
        new: NewCardIssuer,
    ) -> errors::CustomResult<CardIssuer, Self::Error>;

    async fn update_card_issuer(
        &self,
        id: id_type::CardIssuerId,
        update: UpdateCardIssuer,
    ) -> errors::CustomResult<CardIssuer, Self::Error>;

    async fn list_card_issuers(
        &self,
        query: Option<String>,
        limit: Option<u8>,
    ) -> errors::CustomResult<Vec<CardIssuer>, Self::Error>;

    async fn get_card_issuers_by_ids(
        &self,
        ids: Vec<id_type::CardIssuerId>,
    ) -> errors::CustomResult<Vec<CardIssuer>, Self::Error>;
}
