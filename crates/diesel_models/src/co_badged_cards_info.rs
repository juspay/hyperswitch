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
    /// The unique identifier for the co-badged card info
    pub id: common_utils::id_type::CoBadgedCardsInfoID,
    /// Represents the minimum value of the primary card brand's BIN range in which a 
    /// specific BIN value falls. It is a 19-digit number, padded with zeros.
    pub card_bin_min: i64,
    /// Represents the maximum value of the primary card brand's BIN range in which a 
    /// specific BIN value falls. It is a 19-digit number, padded with zeros.
    pub card_bin_max: i64,
    /// The issuing bank name
    pub issuing_bank_name: Option<String>,
    /// The card network
    pub card_network: enums::CardNetwork,
    /// The issuing bank country
    pub country: enums::CountryAlpha2,
    /// The card type eg. credit, debit
    pub card_type: enums::CardType,
    /// Field regulated refers to government-imposed limits on interchange fees for card transactions
    pub regulated: bool,
    /// The name of the regulated entity
    pub regulated_name: Option<String>,
    /// Prepaid cards are a type of payment card that can be loaded with funds in advance and used for transactions
    pub prepaid: bool,
    /// Identifies if the card is reloadable with additional funds. This helps distinguish between one-time-use and reloadable prepaid cards.
    pub reloadable: bool,
    /// Indicates whether the bin range is associated with a PAN or a tokenized card.
    pub pan_or_token: enums::PanOrToken,
    /// The length of the card bin
    pub card_bin_length: i16,
    /// The `card_brand_is_additional` field is used to indicate whether a BIN range is associated with a primary or secondary card network
    pub card_brand_is_additional: bool,
    /// The `domestic_only` field is a Visa-only indicator that shows whether a BIN or Account Range is restricted to domestic use only
    pub domestic_only: bool,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    /// The name of the provider that last updated the card information.
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
