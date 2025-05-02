use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
use crate::{
    errors,
    schema::three_ds_decision_rule::dsl,
    three_ds_decision_rule::{
        ThreeDSDecisionRule, ThreeDSDecisionRuleNew, ThreeDSDecisionRuleUpdateInternal,
    },
    PgPooledConn, StorageResult,
};

impl ThreeDSDecisionRuleNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<ThreeDSDecisionRule> {
        generics::generic_insert(conn, self).await
    }
}

impl ThreeDSDecisionRule {
    pub async fn update(
        self,
        conn: &PgPooledConn,
        update: ThreeDSDecisionRuleUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(conn, dsl::id.eq(self.id.to_owned()), update)
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn find_by_id(
        conn: &PgPooledConn,
        id: &common_utils::id_type::ThreeDSDecisionRuleId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::id.eq(id.to_owned()),
        )
        .await
    }
}
