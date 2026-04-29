use crate::{
    consts,
    errors::{CustomResult, ValidationError},
    generate_id_with_len,
};

crate::id_type!(
    CardIssuerId,
    "A type for card_issuer_id that can be used for unique identifier for a card issuer",
    diesel::sql_types::Text,
    { consts::CARD_ISSUER_ID_LENGTH },
    { consts::CARD_ISSUER_ID_LENGTH }
);
crate::impl_id_type_methods!(CardIssuerId, "card_issuer_id");

// This is to display the `CardIssuerId` as CardIssuerId(abcd)
crate::impl_debug_id_type!(CardIssuerId);
crate::impl_try_from_cow_str_id_type!(CardIssuerId, "card_issuer_id");

crate::impl_serializable_secret_id_type!(CardIssuerId);
crate::impl_queryable_id_type!(CardIssuerId);
crate::impl_to_sql_from_sql_id_type!(
    CardIssuerId,
    diesel::sql_types::Text,
    { consts::CARD_ISSUER_ID_LENGTH },
    { consts::CARD_ISSUER_ID_LENGTH }
);

impl CardIssuerId {
    /// Get card issuer id from String
    pub fn try_from_string(card_issuer_id: String) -> CustomResult<Self, ValidationError> {
        Self::try_from(std::borrow::Cow::from(card_issuer_id))
    }

    /// Generate a new unique card issuer ID of length [`consts::CARD_ISSUER_ID_LENGTH`]
    pub fn generate() -> CustomResult<Self, ValidationError> {
        let id = generate_id_with_len(consts::CARD_ISSUER_ID_LENGTH.into());
        Self::try_from_string(id)
    }
}
