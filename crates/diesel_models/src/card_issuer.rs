use common_utils::id_type;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::card_issuers;

#[derive(
    Clone, Debug, Queryable, Identifiable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = card_issuers, check_for_backend(diesel::pg::Pg))]
pub struct CardIssuer {
    pub id: id_type::CardIssuerId,
    pub issuer_name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = card_issuers)]
pub struct UpdateCardIssuer {
    pub issuer_name: String,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = card_issuers)]
pub struct NewCardIssuer {
    pub id: id_type::CardIssuerId,
    pub issuer_name: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
