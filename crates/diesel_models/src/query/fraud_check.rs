use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use crate::{
    errors, fraud_check::*, query::generics, schema::fraud_check::dsl,
    DatabaseConnectionWithContext, StorageResult,
};

impl FraudCheckNew {
    pub async fn insert(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
    ) -> StorageResult<FraudCheck> {
        generics::generic_insert(conn, self).await
    }
}

impl FraudCheck {
    pub async fn update_with_frm_id(
        self,
        conn: &DatabaseConnectionWithContext<'_>,
        fraud_check: FraudCheckUpdate,
    ) -> StorageResult<Self> {
        match generics::generic_update_with_unique_predicate_get_result::<
            <Self as HasTable>::Table,
            _,
            _,
            _,
        >(
            conn,
            dsl::frm_id
                .eq(self.frm_id.to_owned())
                .and(dsl::merchant_id.eq(self.merchant_id.to_owned())),
            FraudCheckUpdateInternal::from(fraud_check),
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => Ok(self),
                _ => Err(error),
            },
            result => result,
        }
    }

    pub async fn get_with_frm_id(
        conn: &DatabaseConnectionWithContext<'_>,
        frm_id: String,
        merchant_id: common_utils::id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::frm_id.eq(frm_id).and(dsl::merchant_id.eq(merchant_id)),
        )
        .await
    }
}
