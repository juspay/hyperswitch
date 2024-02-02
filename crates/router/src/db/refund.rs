#[cfg(feature = "olap")]
use std::collections::HashSet;

use diesel_models::{errors::DatabaseError, refund::RefundUpdateInternal};
use error_stack::{IntoReport, ResultExt};

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as storage_types, enums},
};

#[cfg(feature = "olap")]
const MAX_LIMIT: usize = 100;

#[async_trait::async_trait]
pub trait RefundInterface {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError>;

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &str,
        connector_refund_id: &str,
        connector: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn update_refund(
        &self,
        this: storage_types::Refund,
        refund: storage_types::RefundUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &str,
        connector_transaction_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError>;

    async fn insert_refund(
        &self,
        new: storage_types::RefundNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_refund_by_constraints(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
        storage_scheme: enums::MerchantStorageScheme,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_refund_by_meta_constraints(
        &self,
        merchant_id: &str,
        refund_details: &api_models::payments::TimeRange,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;

    use super::RefundInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{self as storage_types, enums},
    };

    #[async_trait::async_trait]
    impl RefundInterface for Store {
                /// Asynchronously finds a refund by the given internal reference ID and merchant ID using the specified storage scheme.
        /// 
        /// # Arguments
        /// * `internal_reference_id` - A string slice representing the internal reference ID of the refund to find.
        /// * `merchant_id` - A string slice representing the merchant ID associated with the refund.
        /// * `_storage_scheme` - An enum value representing the storage scheme to use for the operation.
        /// 
        /// # Returns
        /// A `CustomResult` containing the found `storage_types::Refund` if successful, or a `errors::StorageError` if an error occurs.
        ///
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_internal_reference_id_merchant_id(
                &conn,
                internal_reference_id,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

                /// Asynchronously inserts a new refund into the storage using the specified merchant storage scheme.
        ///
        /// # Arguments
        ///
        /// * `new` - The new refund to insert into the storage.
        /// * `_storage_scheme` - The storage scheme to use for the merchant.
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing the inserted refund if successful, or a `StorageError` if an error occurs.
        ///
        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            new.insert(&conn).await.map_err(Into::into).into_report()
        }

                /// Asynchronously finds a refund by the merchant ID and connector transaction ID using the specified storage scheme.
        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &str,
            connector_transaction_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_merchant_id_connector_transaction_id(
                &conn,
                merchant_id,
                connector_transaction_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

                /// Asynchronously updates a refund in the database based on the provided refund update. 
        ///
        /// # Arguments
        ///
        /// * `this` - The original refund to be updated.
        /// * `refund` - The refund update containing the new information to be applied.
        /// * `_storage_scheme` - The storage scheme used for the merchant.
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing the updated refund or a `StorageError` if the update fails.
        ///
        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update(&conn, refund)
                .await
                .map_err(Into::into)
                .into_report()
        }

                /// Asynchronously finds a refund by the given merchant ID and refund ID using the specified storage scheme.
        /// 
        /// # Arguments
        /// 
        /// * `merchant_id` - A reference to a string representing the merchant ID.
        /// * `refund_id` - A reference to a string representing the refund ID.
        /// * `_storage_scheme` - An enum representing the storage scheme to be used for the refund.
        /// 
        /// # Returns
        /// 
        /// A `CustomResult` containing the `storage_types::Refund` if found, otherwise an `errors::StorageError`.
        /// 
        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &str,
            refund_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

                /// Asynchronously finds a refund by the given merchant ID, connector refund ID, and connector, using the specified merchant storage scheme. 
        ///
        /// # Arguments
        ///
        /// * `merchant_id` - A string reference representing the merchant ID
        /// * `connector_refund_id` - A string reference representing the connector refund ID
        /// * `connector` - A string reference representing the connector
        /// * `_storage_scheme` - An enum representing the storage scheme used by the merchant
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing the found refund or a `StorageError` if an error occurs during the operation
        ///
        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &str,
            connector_refund_id: &str,
            connector: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_merchant_id_connector_refund_id_connector(
                &conn,
                merchant_id,
                connector_refund_id,
                connector,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

                /// Asynchronously finds a refund by payment ID and merchant ID using the specified storage scheme.
        /// 
        /// # Arguments
        /// 
        /// * `payment_id` - A reference to a string representing the payment ID.
        /// * `merchant_id` - A reference to a string representing the merchant ID.
        /// * `_storage_scheme` - An enum value representing the storage scheme used by the merchant.
        /// 
        /// # Returns
        /// 
        /// A `CustomResult` containing a vector of `storage_types::Refund` or a `StorageError` in case of failure.
        /// 
        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }
        #[cfg(feature = "olap")]
                /// Asynchronously filters refunds based on the given constraints and returns a vector of refunds. It takes the merchant ID, refund details, storage scheme, limit, and offset as input parameters. It then establishes a read connection to the database, filters the refunds based on the provided constraints using the `filter_by_constraints` method from the `RefundDbExt` trait, and returns the result as a vector of refunds. If an error occurs during the process, it is converted into a `StorageError` and returned as a `CustomResult`.
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            _storage_scheme: enums::MerchantStorageScheme,
            limit: i64,
            offset: i64,
        ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::filter_by_constraints(
                &conn,
                merchant_id,
                refund_details,
                limit,
                offset,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        #[cfg(feature = "olap")]
                /// Filters refunds by meta constraints for a specific merchant and returns the metadata of the refunded list.
        ///
        /// # Arguments
        ///
        /// * `merchant_id` - A string reference representing the merchant's ID.
        /// * `refund_details` - A reference to a `TimeRange` object containing the details of the refund.
        /// * `_storage_scheme` - An enum representing the storage scheme used by the merchant.
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing the metadata of the refunded list or a `StorageError` in case of failure.
        ///
        async fn filter_refund_by_meta_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::payments::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::filter_by_meta_constraints(
                &conn,
                merchant_id,
                refund_details,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }
        #[cfg(feature = "olap")]
                /// Asynchronously retrieves the total count of refunds for a given merchant and refund details using the specified storage scheme.
        ///
        /// # Arguments
        ///
        /// * `merchant_id` - A reference to the merchant ID for which the refunds count is to be retrieved.
        /// * `refund_details` - A reference to the refund details (such as filters and pagination) used to query the refunds count.
        /// * `_storage_scheme` - The storage scheme used for retrieving the refunds count.
        ///
        /// # Returns
        ///
        /// A custom result containing the total count of refunds as an `i64` or a `StorageError` if the operation fails.
        ///
        async fn get_total_count_of_refunds(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refunds_count(
                &conn,
                merchant_id,
                refund_details,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::{date_time, fallback_reverse_lookup_not_found};
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation};

    use super::RefundInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{self as storage_types, enums, kv},
        utils::{self, db_utils},
    };
    #[async_trait::async_trait]
    impl RefundInterface for Store {
                /// Asynchronously finds a refund by its internal reference ID and merchant ID, based on the specified storage scheme. If the storage scheme is PostgresOnly, the method performs a database call to retrieve the refund. If the storage scheme is RedisKv, the method first attempts to retrieve the refund from a Redis key-value store, falling back to a database call if the refund is not found in the Redis store.
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_internal_reference_id_merchant_id(
                    &conn,
                    internal_reference_id,
                    merchant_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("ref_inter_ref_{merchant_id}_{internal_reference_id}");
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = &lookup.pk_id;
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            )
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

                /// This method is used to insert a new refund into the storage based on the specified storage scheme. 
        /// If the storage scheme is PostgresOnly, it inserts the refund into a PostgreSQL database. 
        /// If the storage scheme is RedisKv, it creates a new refund entry and associated reverse lookups in a Redis key-value store. 
        /// 
        /// # Arguments
        /// 
        /// * `new` - The new refund data to be inserted
        /// * `storage_scheme` - The storage scheme to be used for inserting the refund
        /// 
        /// # Returns
        /// 
        /// A `CustomResult` containing the inserted `storage_types::Refund` if successful, or a `errors::StorageError` if an error occurs during the insertion process.
        /// 
        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    new.insert(&conn).await.map_err(Into::into).into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", new.merchant_id, new.payment_id);
                    // TODO: need to add an application generated payment attempt id to distinguish between multiple attempts for the same payment id
                    // Check for database presence as well Maybe use a read replica here ?
                    let created_refund = storage_types::Refund {
                        id: 0i32,
                        refund_id: new.refund_id.clone(),
                        merchant_id: new.merchant_id.clone(),
                        attempt_id: new.attempt_id.clone(),
                        internal_reference_id: new.internal_reference_id.clone(),
                        payment_id: new.payment_id.clone(),
                        connector_transaction_id: new.connector_transaction_id.clone(),
                        connector: new.connector.clone(),
                        connector_refund_id: new.connector_refund_id.clone(),
                        external_reference_id: new.external_reference_id.clone(),
                        refund_type: new.refund_type,
                        total_amount: new.total_amount,
                        currency: new.currency,
                        refund_amount: new.refund_amount,
                        refund_status: new.refund_status,
                        sent_to_gateway: new.sent_to_gateway,
                        refund_error_message: None,
                        refund_error_code: None,
                        metadata: new.metadata.clone(),
                        refund_arn: new.refund_arn.clone(),
                        created_at: new.created_at.unwrap_or_else(date_time::now),
                        updated_at: new.created_at.unwrap_or_else(date_time::now),
                        description: new.description.clone(),
                        refund_reason: new.refund_reason.clone(),
                        profile_id: new.profile_id.clone(),
                        updated_by: new.updated_by.clone(),
                        merchant_connector_id: new.merchant_connector_id.clone(),
                    };

                    let field = format!(
                        "pa_{}_ref_{}",
                        &created_refund.attempt_id, &created_refund.refund_id
                    );

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: kv::Insertable::Refund(new),
                        },
                    };

                    let mut reverse_lookups = vec![
                        storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_ref_id_{}_{}",
                                created_refund.merchant_id, created_refund.refund_id
                            ),
                            pk_id: key.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        },
                        // [#492]: A discussion is required on whether this is required?
                        storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_inter_ref_{}_{}",
                                created_refund.merchant_id, created_refund.internal_reference_id
                            ),
                            pk_id: key.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        },
                    ];
                    if let Some(connector_refund_id) = created_refund.to_owned().connector_refund_id
                    {
                        reverse_lookups.push(storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_connector_{}_{}_{}",
                                created_refund.merchant_id,
                                connector_refund_id,
                                created_refund.connector
                            ),
                            pk_id: key.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        })
                    };
                    let rev_look = reverse_lookups
                        .into_iter()
                        .map(|rev| self.insert_reverse_lookup(rev, storage_scheme));

                    futures::future::try_join_all(rev_look).await?;

                    match kv_wrapper::<storage_types::Refund, _, _>(
                        self,
                        KvOperation::<storage_types::Refund>::HSetNx(
                            &field,
                            &created_refund,
                            redis_entry,
                        ),
                        &key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "refund",
                            key: Some(created_refund.refund_id),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => Ok(created_refund),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

                /// This method finds a refund by the specified merchant ID and connector transaction ID, based on the given storage scheme. If the scheme is PostgresOnly, it will make a database call to retrieve the refund. If the scheme is RedisKv, it will first attempt to retrieve the refund from a Redis key-value store, and if not found, it will fall back to making a database call. The method returns a vector of refunds or a storage error.
        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &str,
            connector_transaction_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_merchant_id_connector_transaction_id(
                    &conn,
                    merchant_id,
                    connector_transaction_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id =
                        format!("pa_conn_trans_{merchant_id}_{connector_transaction_id}");
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = &lookup.pk_id;

                    let pattern = db_utils::generate_hscan_pattern_for_refund(&lookup.sk_id);

                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::Scan(&pattern),
                                key,
                            )
                            .await?
                            .try_into_scan()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

                /// Asynchronously updates a refund based on the specified storage scheme. If the storage scheme is set to PostgresOnly, the method updates the refund in the Postgres database. If the storage scheme is set to RedisKv, the method updates the refund in the Redis key-value store. The method returns a CustomResult containing the updated refund or a StorageError if the operation fails.
        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    this.update(&conn, refund)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", this.merchant_id, this.payment_id);
                    let field = format!("pa_{}_ref_{}", &this.attempt_id, &this.refund_id);
                    let updated_refund = refund.clone().apply_changeset(this.clone());

                    let redis_value =
                        utils::Encode::<storage_types::Refund>::encode_to_string_of_json(
                            &updated_refund,
                        )
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::RefundUpdate(kv::RefundUpdateMems {
                                orig: this,
                                update_data: refund,
                            }),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<storage_types::Refund>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        &key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_refund)
                }
            }
        }

                /// Asynchronously finds a refund by the specified merchant ID and refund ID, based on the provided storage scheme.
        /// 
        /// # Arguments
        /// 
        /// * `merchant_id` - A reference to a string representing the merchant ID
        /// * `refund_id` - A reference to a string representing the refund ID
        /// * `storage_scheme` - An enum value representing the storage scheme to be used
        /// 
        /// # Returns
        /// 
        /// A `CustomResult` containing a `Refund` if successful, otherwise a `StorageError`
        /// 
        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &str,
            refund_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("ref_ref_id_{merchant_id}_{refund_id}");
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = &lookup.pk_id;
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            )
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

                /// Asynchronously finds a refund by merchant ID, connector refund ID, and connector, based on the specified storage scheme.
        ///
        /// # Arguments
        ///
        /// * `merchant_id` - The ID of the merchant
        /// * `connector_refund_id` - The ID of the connector refund
        /// * `connector` - The connector
        /// * `storage_scheme` - The storage scheme to be used
        ///
        /// # Returns
        ///
        /// A `CustomResult` containing the found refund or a `StorageError` if an error occurs.
        ///
        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &str,
            connector_refund_id: &str,
            connector: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_merchant_id_connector_refund_id_connector(
                    &conn,
                    merchant_id,
                    connector_refund_id,
                    connector,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id =
                        format!("ref_connector_{merchant_id}_{connector_refund_id}_{connector}");
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = &lookup.pk_id;
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            )
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

                /// Asynchronously finds a refund by payment ID and merchant ID based on the specified storage scheme.
        /// Returns a vector of refunds or a storage error.
        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_payment_id_merchant_id(
                    &conn,
                    payment_id,
                    merchant_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{merchant_id}_pid_{payment_id}");
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::Scan("pa_*_ref_*"),
                                key,
                            )
                            .await?
                            .try_into_scan()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

        #[cfg(feature = "olap")]
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            _storage_scheme: enums::MerchantStorageScheme,
            limit: i64,
            offset: i64,
        ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::filter_by_constraints(
                &conn,
                merchant_id,
                refund_details,
                limit,
                offset,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        #[cfg(feature = "olap")]
                /// Asynchronously filters refunds by the given meta constraints for a specific merchant and time range using the specified storage scheme. 
        /// 
        /// # Arguments
        /// 
        /// * `merchant_id` - A string reference representing the merchant ID.
        /// * `refund_details` - A reference to a `TimeRange` object containing the details of the refund time range.
        /// * `_storage_scheme` - An enum value representing the storage scheme to be used.
        /// 
        /// # Returns
        /// 
        /// A `CustomResult` containing the metadata of the filtered refunds, or a `StorageError` if an error occurs during the storage operation.
        /// 
        async fn filter_refund_by_meta_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::payments::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::filter_by_meta_constraints(&conn, merchant_id, refund_details)
                        .await
                        .map_err(Into::into)
                        .into_report()
        }

        #[cfg(feature = "olap")]
        async fn get_total_count_of_refunds(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refunds_count(
                &conn,
                merchant_id,
                refund_details,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }
    }
}

#[async_trait::async_trait]
impl RefundInterface for MockDb {
        /// Asynchronously finds a refund by its internal reference ID and merchant ID.
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;
        refunds
            .iter()
            .find(|refund| {
                refund.merchant_id == merchant_id
                    && refund.internal_reference_id == internal_reference_id
            })
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

        /// Inserts a new refund into the refunds collection and returns the inserted refund.
    ///
    /// # Arguments
    ///
    /// * `new` - The new refund to be inserted.
    /// * `_storage_scheme` - The storage scheme to be used for the merchant.
    ///
    /// # Returns
    ///
    /// The inserted refund if successful, otherwise a `StorageError`.
    ///
    async fn insert_refund(
        &self,
        new: storage_types::RefundNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let mut refunds = self.refunds.lock().await;
        let current_time = common_utils::date_time::now();

        let refund = storage_types::Refund {
            id: refunds
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            internal_reference_id: new.internal_reference_id,
            refund_id: new.refund_id,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            attempt_id: new.attempt_id,
            connector_transaction_id: new.connector_transaction_id,
            connector: new.connector,
            connector_refund_id: new.connector_refund_id,
            external_reference_id: new.external_reference_id,
            refund_type: new.refund_type,
            total_amount: new.total_amount,
            currency: new.currency,
            refund_amount: new.refund_amount,
            refund_status: new.refund_status,
            sent_to_gateway: new.sent_to_gateway,
            refund_error_message: None,
            refund_error_code: None,
            metadata: new.metadata,
            refund_arn: new.refund_arn.clone(),
            created_at: new.created_at.unwrap_or(current_time),
            updated_at: current_time,
            description: new.description,
            refund_reason: new.refund_reason.clone(),
            profile_id: new.profile_id,
            updated_by: new.updated_by,
            merchant_connector_id: new.merchant_connector_id,
        };
        refunds.push(refund.clone());
        Ok(refund)
    }
        /// Asynchronously finds refunds by the merchant ID and connector transaction ID using the provided merchant storage scheme.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `connector_transaction_id` - A reference to a string representing the connector transaction ID.
    /// * `_storage_scheme` - An enum representing the merchant storage scheme.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `Refund` items or a `StorageError`.
    /// 
    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &str,
        connector_transaction_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        Ok(refunds
            .iter()
            .take_while(|refund| {
                refund.merchant_id == merchant_id
                    && refund.connector_transaction_id == connector_transaction_id
            })
            .cloned()
            .collect::<Vec<_>>())
    }

        /// Asynchronously updates a refund in the storage based on the provided refund ID and refund update information.
    ///
    /// # Arguments
    ///
    /// * `this` - The original refund to be updated.
    /// * `refund` - The refund update information.
    /// * `_storage_scheme` - The storage scheme being used.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the updated refund if successful, otherwise a `StorageError` indicating the failure reason.
    ///
    async fn update_refund(
        &self,
        this: storage_types::Refund,
        refund: storage_types::RefundUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        self.refunds
            .lock()
            .await
            .iter_mut()
            .find(|refund| this.refund_id == refund.refund_id)
            .map(|r| {
                let refund_updated = RefundUpdateInternal::from(refund).create_refund(r.clone());
                *r = refund_updated.clone();
                refund_updated
            })
            .ok_or_else(|| {
                errors::StorageError::ValueNotFound("cannot find refund to update".to_string())
                    .into()
            })
    }

        /// Asynchronously finds a refund by the given merchant ID and refund ID using the specified storage scheme.
    /// Returns a Result containing a Refund if found, or a StorageError if not found.
    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &str,
        refund_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        refunds
            .iter()
            .find(|refund| refund.merchant_id == merchant_id && refund.refund_id == refund_id)
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

        /// Asynchronously finds a refund by the merchant ID, connector refund ID, and connector. It locks the refunds and then iterates through them to find the refund matching the provided criteria. If found, it returns the refund; otherwise, it returns a `StorageError` indicating that the refund was not found in the database. 
    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &str,
        connector_refund_id: &str,
        connector: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        refunds
            .iter()
            .find(|refund| {
                refund.merchant_id == merchant_id
                    && refund.connector_refund_id == Some(connector_refund_id.to_string())
                    && refund.connector == connector
            })
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

        /// Asynchronously finds a refund by payment ID and merchant ID in the merchant's storage scheme.
    /// 
    /// # Arguments
    /// 
    /// * `payment_id` - A reference to a string representing the payment ID.
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `_storage_scheme` - An enum representing the storage scheme used by the merchant.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of `Refund` objects if successful, otherwise a `StorageError`.
    /// 
    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        Ok(refunds
            .iter()
            .filter(|refund| refund.merchant_id == merchant_id && refund.payment_id == payment_id)
            .cloned()
            .collect::<Vec<_>>())
    }

    #[cfg(feature = "olap")]
        /// Asynchronously filters a list of refunds based on various constraints such as merchant ID, refund details, storage scheme, limit, and offset.
    async fn filter_refund_by_constraints(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
        _storage_scheme: enums::MerchantStorageScheme,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError> {
        let mut unique_connectors = HashSet::new();
        let mut unique_currencies = HashSet::new();
        let mut unique_statuses = HashSet::new();

        // Fill the hash sets with data from refund_details
        if let Some(connectors) = &refund_details.connector {
            connectors.iter().for_each(|connector| {
                unique_connectors.insert(connector);
            });
        }

        if let Some(currencies) = &refund_details.currency {
            currencies.iter().for_each(|currency| {
                unique_currencies.insert(currency);
            });
        }

        if let Some(refund_statuses) = &refund_details.refund_status {
            refund_statuses.iter().for_each(|refund_status| {
                unique_statuses.insert(refund_status);
            });
        }

        let refunds = self.refunds.lock().await;
        let filtered_refunds = refunds
            .iter()
            .filter(|refund| refund.merchant_id == merchant_id)
            .filter(|refund| {
                refund_details
                    .payment_id
                    .clone()
                    .map_or(true, |id| id == refund.payment_id)
            })
            .filter(|refund| {
                refund_details
                    .refund_id
                    .clone()
                    .map_or(true, |id| id == refund.refund_id)
            })
            .filter(|refund| refund_details.profile_id == refund.profile_id)
            .filter(|refund| {
                refund.created_at
                    >= refund_details.time_range.map_or(
                        common_utils::date_time::now() - time::Duration::days(60),
                        |range| range.start_time,
                    )
                    && refund.created_at
                        <= refund_details
                            .time_range
                            .map_or(common_utils::date_time::now(), |range| {
                                range.end_time.unwrap_or_else(common_utils::date_time::now)
                            })
            })
            .filter(|refund| {
                unique_connectors.is_empty() || unique_connectors.contains(&refund.connector)
            })
            .filter(|refund| {
                unique_currencies.is_empty() || unique_currencies.contains(&refund.currency)
            })
            .filter(|refund| {
                unique_statuses.is_empty() || unique_statuses.contains(&refund.refund_status)
            })
            .skip(usize::try_from(offset).unwrap_or_default())
            .take(usize::try_from(limit).unwrap_or(MAX_LIMIT))
            .cloned()
            .collect::<Vec<_>>();

        Ok(filtered_refunds)
    }

    #[cfg(feature = "olap")]
        /// Filters a list of refunds based on the provided time range and returns the metadata of the filtered refunds, including unique connectors, currencies, and refund statuses.
    async fn filter_refund_by_meta_constraints(
        &self,
        _merchant_id: &str,
        refund_details: &api_models::payments::TimeRange,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        let start_time = refund_details.start_time;
        let end_time = refund_details
            .end_time
            .unwrap_or_else(common_utils::date_time::now);

        let filtered_refunds = refunds
            .iter()
            .filter(|refund| refund.created_at >= start_time && refund.created_at <= end_time)
            .cloned()
            .collect::<Vec<diesel_models::refund::Refund>>();

        let mut refund_meta_data = api_models::refunds::RefundListMetaData {
            connector: vec![],
            currency: vec![],
            refund_status: vec![],
        };

        let mut unique_connectors = HashSet::new();
        let mut unique_currencies = HashSet::new();
        let mut unique_statuses = HashSet::new();

        for refund in filtered_refunds.into_iter() {
            unique_connectors.insert(refund.connector);

            let currency: api_models::enums::Currency = refund.currency;
            unique_currencies.insert(currency);

            let status: api_models::enums::RefundStatus = refund.refund_status;
            unique_statuses.insert(status);
        }

        refund_meta_data.connector = unique_connectors.into_iter().collect();
        refund_meta_data.currency = unique_currencies.into_iter().collect();
        refund_meta_data.refund_status = unique_statuses.into_iter().collect();

        Ok(refund_meta_data)
    }

    #[cfg(feature = "olap")]
    /// Asynchronously retrieves the total count of refunds based on the provided parameters.
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - The ID of the merchant for which refunds are being retrieved.
    /// * `refund_details` - The details of the refunds to filter on.
    /// * `_storage_scheme` - The storage scheme used for the refunds.
    /// 
    /// # Returns
    /// 
    /// Returns a `CustomResult` containing the total count of refunds or a `StorageError` if an error occurs.
    /// 
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &str,
        refund_details: &api_models::refunds::RefundListRequest,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        let mut unique_connectors = HashSet::new();
        let mut unique_currencies = HashSet::new();
        let mut unique_statuses = HashSet::new();

        // Fill the hash sets with data from refund_details
        if let Some(connectors) = &refund_details.connector {
            connectors.iter().for_each(|connector| {
                unique_connectors.insert(connector);
            });
        }

        if let Some(currencies) = &refund_details.currency {
            currencies.iter().for_each(|currency| {
                unique_currencies.insert(currency);
            });
        }

        if let Some(refund_statuses) = &refund_details.refund_status {
            refund_statuses.iter().for_each(|refund_status| {
                unique_statuses.insert(refund_status);
            });
        }

        let refunds = self.refunds.lock().await;
        let filtered_refunds = refunds
            .iter()
            .filter(|refund| refund.merchant_id == merchant_id)
            .filter(|refund| {
                refund_details
                    .payment_id
                    .clone()
                    .map_or(true, |id| id == refund.payment_id)
            })
            .filter(|refund| {
                refund_details
                    .refund_id
                    .clone()
                    .map_or(true, |id| id == refund.refund_id)
            })
            .filter(|refund| refund_details.profile_id == refund.profile_id)
            .filter(|refund| {
                refund.created_at
                    >= refund_details.time_range.map_or(
                        common_utils::date_time::now() - time::Duration::days(60),
                        |range| range.start_time,
                    )
                    && refund.created_at
                        <= refund_details
                            .time_range
                            .map_or(common_utils::date_time::now(), |range| {
                                range.end_time.unwrap_or_else(common_utils::date_time::now)
                            })
            })
            .filter(|refund| {
                unique_connectors.is_empty() || unique_connectors.contains(&refund.connector)
            })
            .filter(|refund| {
                unique_currencies.is_empty() || unique_currencies.contains(&refund.currency)
            })
            .filter(|refund| {
                unique_statuses.is_empty() || unique_statuses.contains(&refund.refund_status)
            })
            .cloned()
            .collect::<Vec<_>>();

        let filtered_refunds_count = filtered_refunds.len().try_into().unwrap_or_default();

        Ok(filtered_refunds_count)
    }
}
