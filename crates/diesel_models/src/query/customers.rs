use common_utils::id_type;
#[cfg(all(feature = "v2", feature = "customer_v2"))]
use diesel::BoolExpressionMethods;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use diesel::BoolExpressionMethods;
use diesel::{associations::HasTable, ExpressionMethods};

use super::generics;
// #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::errors;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use crate::schema::customers::dsl;
#[cfg(all(feature = "v2", feature = "customer_v2"))]
use crate::schema_v2::customers::dsl;
use crate::{
    customers::{Customer, CustomerNew, CustomerUpdateInternal},
    PgPooledConn, StorageResult,
};

impl CustomerNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Customer> {
        generics::generic_insert(conn, self).await
    }
}

pub struct CustomerListConstraints {
    pub limit: i64,
    pub offset: Option<i64>,
}

// #[cfg(all(feature = "v2", feature = "customer_v2"))]
impl Customer {
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    pub async fn update_by_id(
        conn: &PgPooledConn,
        id: id_type::GlobalCustomerId,
        customer: CustomerUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            id.clone(),
            customer,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id).await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    pub async fn find_by_global_id(
        conn: &PgPooledConn,
        id: &id_type::GlobalCustomerId,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id.to_owned()).await
    }

    pub async fn list_by_merchant_id(
        conn: &PgPooledConn,
        merchant_id: &id_type::MerchantId,
        constraints: CustomerListConstraints,
    ) -> StorageResult<Vec<Self>> {
        generics::generic_filter::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::merchant_id.eq(merchant_id.to_owned()),
            Some(constraints.limit),
            constraints.offset,
            Some(dsl::created_at),
        )
        .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    pub async fn find_optional_by_merchant_id_merchant_reference_id(
        conn: &PgPooledConn,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_one_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_reference_id.eq(customer_id.to_owned())),
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    pub async fn find_optional_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Option<Self>> {
        generics::generic_find_by_id_optional::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    pub async fn update_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: id_type::CustomerId,
        merchant_id: id_type::MerchantId,
        customer: CustomerUpdateInternal,
    ) -> StorageResult<Self> {
        match generics::generic_update_by_id::<<Self as HasTable>::Table, _, _, _>(
            conn,
            (customer_id.clone(), merchant_id.clone()),
            customer,
        )
        .await
        {
            Err(error) => match error.current_context() {
                errors::DatabaseError::NoFieldsToUpdate => {
                    generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
                        conn,
                        (customer_id, merchant_id),
                    )
                    .await
                }
                _ => Err(error),
            },
            result => result,
        }
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    pub async fn delete_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<bool> {
        generics::generic_delete::<<Self as HasTable>::Table, _>(
            conn,
            dsl::customer_id
                .eq(customer_id.to_owned())
                .and(dsl::merchant_id.eq(merchant_id.to_owned())),
        )
        .await
    }

    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    pub async fn find_by_merchant_reference_id_merchant_id(
        conn: &PgPooledConn,
        merchant_reference_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::merchant_id
                .eq(merchant_id.to_owned())
                .and(dsl::merchant_reference_id.eq(merchant_reference_id.to_owned())),
        )
        .await
    }

    #[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
    pub async fn find_by_customer_id_merchant_id(
        conn: &PgPooledConn,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(
            conn,
            (customer_id.to_owned(), merchant_id.to_owned()),
        )
        .await
    }
}
