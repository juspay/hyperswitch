use common_enums::enums;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::co_badged_cards_info;

#[derive(
    Clone,
    Debug,
    Queryable,
    Identifiable,
    Selectable,
    serde::Deserialize,
    serde::Serialize,
    Insertable,
)]
#[diesel(table_name = co_badged_cards_info, primary_key(card_bin_min, card_bin_max, issuing_bank_name), check_for_backend(diesel::pg::Pg))]
pub struct CoBadgedCardInfo {
    pub id: common_utils::id_type::CoBadgedCardsInfoID,
    pub card_bin_min: i64,
    pub card_bin_max: i64,
    pub issuing_bank_name: Option<String>,
    pub card_network: enums::CardNetwork,
    pub country: enums::CountryAlpha2,
    pub card_type: enums::CardType,
    pub regulated: bool,
    pub regulated_name: Option<String>,
    pub prepaid: bool,
    pub reloadable: bool,
    pub pan_or_token: enums::PanOrToken,
    pub card_bin_length: i16,
    pub card_brand_is_additional: bool,
    pub domestic_only: bool,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub last_updated_provider: Option<String>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, router_derive::DebugAsDisplay, serde::Deserialize, AsChangeset,
)]
#[diesel(table_name = co_badged_cards_info)]
pub struct UpdateCoBadgedCardInfo {
    pub card_bin_min: Option<i64>,
    pub card_bin_max: Option<i64>,
    pub card_network: Option<enums::CardNetwork>,
    pub country: Option<enums::CountryAlpha2>,
    pub regulated: Option<bool>,
    pub regulated_name: Option<String>,
    pub prepaid: Option<bool>,
    pub reloadable: Option<bool>,
    pub pan_or_token: Option<enums::PanOrToken>,
    pub card_bin_length: Option<i16>,
    pub card_brand_is_additional: bool,
    pub domestic_only: Option<bool>,
    pub modified_at: PrimitiveDateTime,
    pub last_updated_provider: Option<String>,
}
