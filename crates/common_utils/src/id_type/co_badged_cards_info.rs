crate::id_type!(
    CoBadgedCardsInfoID,
    "A type for co_badged_cards_info_id that can be used for co_badged_cards_info ids"
);
crate::impl_id_type_methods!(CoBadgedCardsInfoID, "co_badged_cards_info_id");
crate::impl_generate_id_id_type!(CoBadgedCardsInfoID, "co_badged_cards_info");
crate::impl_serializable_secret_id_type!(CoBadgedCardsInfoID);
crate::impl_queryable_id_type!(CoBadgedCardsInfoID);
crate::impl_to_sql_from_sql_id_type!(CoBadgedCardsInfoID);

crate::impl_debug_id_type!(CoBadgedCardsInfoID);
