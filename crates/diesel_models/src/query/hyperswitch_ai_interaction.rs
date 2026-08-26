use diesel::{associations::HasTable, ExpressionMethods};

use crate::{
    hyperswitch_ai_interaction::{HyperswitchAiInteraction, HyperswitchAiInteractionNew},
    query::generics,
    schema::hyperswitch_ai_interaction::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl HyperswitchAiInteractionNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<HyperswitchAiInteraction> {
        generics::generic_insert(conn, self).await
    }
}

impl HyperswitchAiInteraction {
    pub async fn filter_by_optional_merchant_id(
        conn: &DatabaseConnectionWithContext<'_>,
        merchant_id: Option<&common_utils::id_type::MerchantId>,
        limit: i64,
        offset: i64,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.cloned()),
            Some(limit),
            Some(offset),
            Some(dsl::created_at.desc()),
        )
        .await
    }
}
