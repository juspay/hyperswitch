use common_utils::id_type;
use diesel::{associations::HasTable, BoolExpressionMethods, ExpressionMethods};

use super::generics;
#[cfg(feature = "v1")]
use crate::schema::customers::dsl;
#[cfg(feature = "v2")]
use crate::schema_v2::customers::dsl;
use crate::{
    customers::{Customer, CustomerNew, CustomerUpdateInternal},
    errors, PgPooledConn, StorageResult,
};

impl CustomerNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<Customer> {
        generics::generic_insert(conn, self).await
    }
}

pub struct CustomerListConstraints {
    pub limit: i64,
    pub offset: Option<i64>,
    pub customer_id: Option<id_type::CustomerId>,
    pub time_range: Option<common_utils::types::TimeRange>,
}

impl Customer {
    #[cfg(feature = "v2")]
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

    #[cfg(feature = "v2")]
    pub async fn find_by_global_id(
        conn: &PgPooledConn,
        id: &id_type::GlobalCustomerId,
    ) -> StorageResult<Self> {
        generics::generic_find_by_id::<<Self as HasTable>::Table, _, _>(conn, id.to_owned()).await
    }

    #[cfg(feature = "v1")]
    pub async fn get_customer_count_by_merchant_id_and_constraints(
        conn: &PgPooledConn,
        merchant_id: &id_type::MerchantId,
        customer_list_constraints: CustomerListConstraints,
    ) -> StorageResult<usize> {
        if let Some(customer_id) = customer_list_constraints.customer_id {
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::customer_id.eq(customer_id));
            generics::generic_count::<<Self as HasTable>::Table, _>(conn, predicate).await
        } else if let Some(time_range) = customer_list_constraints.time_range {
            let start_time = time_range.start_time;
            let end_time = time_range
                .end_time
                .unwrap_or_else(common_utils::date_time::now);
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::created_at.between(start_time, end_time));

            generics::generic_count::<<Self as HasTable>::Table, _>(conn, predicate).await
        } else {
            generics::generic_count::<<Self as HasTable>::Table, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
            )
            .await
        }
    }

    #[cfg(feature = "v2")]
    pub async fn get_customer_count_by_merchant_id_and_constraints(
        conn: &PgPooledConn,
        merchant_id: &id_type::MerchantId,
        customer_list_constraints: CustomerListConstraints,
    ) -> StorageResult<usize> {
        if let Some(customer_id) = customer_list_constraints.customer_id {
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::merchant_reference_id.eq(customer_id));
            generics::generic_count::<<Self as HasTable>::Table, _>(conn, predicate).await
        } else if let Some(time_range) = customer_list_constraints.time_range {
            let start_time = time_range.start_time;
            let end_time = time_range
                .end_time
                .unwrap_or_else(common_utils::date_time::now);
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::created_at.between(start_time, end_time));

            generics::generic_count::<<Self as HasTable>::Table, _>(conn, predicate).await
        } else {
            generics::generic_count::<<Self as HasTable>::Table, _>(
                conn,
                dsl::merchant_id.eq(merchant_id.to_owned()),
            )
            .await
        }
    }

    #[cfg(feature = "v1")]
    pub async fn list_customers_by_merchant_id_and_constraints(
        conn: &PgPooledConn,
        merchant_id: &id_type::MerchantId,
        constraints: CustomerListConstraints,
    ) -> StorageResult<Vec<Self>> {
        if let Some(customer_id) = constraints.customer_id {
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::customer_id.eq(customer_id));
            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        } else if let Some(time_range) = constraints.time_range {
            let start_time = time_range.start_time;
            let end_time = time_range
                .end_time
                .unwrap_or_else(common_utils::date_time::now);
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::created_at.between(start_time, end_time));

            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        } else {
            let predicate = dsl::merchant_id.eq(merchant_id.clone());
            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        }
    }

    #[cfg(feature = "v2")]
    pub async fn list_customers_by_merchant_id_and_constraints(
        conn: &PgPooledConn,
        merchant_id: &id_type::MerchantId,
        constraints: CustomerListConstraints,
    ) -> StorageResult<Vec<Self>> {
        if let Some(customer_id) = constraints.customer_id {
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::merchant_reference_id.eq(customer_id));
            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        } else if let Some(time_range) = constraints.time_range {
            let start_time = time_range.start_time;
            let end_time = time_range
                .end_time
                .unwrap_or_else(common_utils::date_time::now);
            let predicate = dsl::merchant_id
                .eq(merchant_id.clone())
                .and(dsl::created_at.between(start_time, end_time));

            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        } else {
            let predicate = dsl::merchant_id.eq(merchant_id.clone());
            generics::generic_filter::<<Self as HasTable>::Table, _, _, Self>(
                conn,
                predicate,
                Some(constraints.limit),
                constraints.offset,
                Some(dsl::created_at),
            )
            .await
        }
    }

    #[cfg(feature = "v2")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v1")]
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

    #[cfg(feature = "v2")]
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

    #[cfg(feature = "v1")]
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
