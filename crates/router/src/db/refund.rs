use storage_models::errors::DatabaseError;

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as storage_types, enums},
};

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

    // async fn find_refund_by_payment_id_merchant_id_refund_id(
    //     &self,
    //     payment_id: &str,
    //     merchant_id: &str,
    //     refund_id: &str,
    // ) -> CustomResult<Refund, errors::StorageError>;

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
    ) -> CustomResult<Vec<storage_models::refund::Refund>, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;

    use super::RefundInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{self as storage_types, enums},
    };

    #[async_trait::async_trait]
    impl RefundInterface for Store {
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            storage_types::Refund::find_by_internal_reference_id_merchant_id(
                &conn,
                internal_reference_id,
                merchant_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            new.insert(&conn).await.map_err(Into::into).into_report()
        }

        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &str,
            connector_transaction_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            storage_types::Refund::find_by_merchant_id_connector_transaction_id(
                &conn,
                merchant_id,
                connector_transaction_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            this.update(&conn, refund)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &str,
            refund_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &str,
            connector_refund_id: &str,
            connector: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
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

        // async fn find_refund_by_payment_id_merchant_id_refund_id(
        //     &self,
        //     payment_id: &str,
        //     merchant_id: &str,
        //     refund_id: &str,
        // ) -> CustomResult<Refund, errors::StorageError> {
        //     let conn = pg_connection(&self.master_pool).await;
        //     Refund::find_by_payment_id_merchant_id_refund_id(&conn, payment_id, merchant_id, refund_id)
        //         .await
        // }

        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let conn = pg_connection(&self.master_pool).await?;
            storage_types::Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        }

        #[cfg(feature = "olap")]
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            _storage_scheme: enums::MerchantStorageScheme,
            limit: i64,
        ) -> CustomResult<Vec<storage_models::refund::Refund>, errors::StorageError> {
            let conn = pg_connection(&self.replica_pool).await?;
            <storage_models::refund::Refund as storage_types::RefundDbExt>::filter_by_constraints(
                &conn,
                merchant_id,
                refund_details,
                limit,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::date_time;
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;

    use super::RefundInterface;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        logger,
        services::Store,
        types::storage::{self as storage_types, enums, kv},
        utils::{self, db_utils, storage_partitioning::PartitionKey},
    };
    #[async_trait::async_trait]
    impl RefundInterface for Store {
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
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
                    let lookup_id = format!("{merchant_id}_{internal_reference_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;

                    let key = &lookup.pk_id;
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "Refund"),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    new.insert(&conn).await.map_err(Into::into).into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", new.merchant_id, new.payment_id);
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
                    };

                    let field = format!(
                        "pa_{}_ref_{}",
                        &created_refund.attempt_id, &created_refund.refund_id
                    );
                    match self
                        .redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .serialize_and_set_hash_field_if_not_exist(&key, &field, &created_refund)
                        .await
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "refund",
                            key: Some(created_refund.refund_id),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => {
                            let conn = pg_connection(&self.master_pool).await?;

                            let mut reverse_lookups = vec![
                                storage_types::ReverseLookupNew {
                                    sk_id: field.clone(),
                                    lookup_id: format!(
                                        "{}_{}",
                                        created_refund.merchant_id, created_refund.refund_id
                                    ),
                                    pk_id: key.clone(),
                                    source: "refund".to_string(),
                                },
                                // [#492]: A discussion is required on whether this is required?
                                storage_types::ReverseLookupNew {
                                    sk_id: field.clone(),
                                    lookup_id: format!(
                                        "{}_{}",
                                        created_refund.merchant_id,
                                        created_refund.internal_reference_id
                                    ),
                                    pk_id: key.clone(),
                                    source: "refund".to_string(),
                                },
                            ];
                            if let Some(connector_refund_id) =
                                created_refund.to_owned().connector_refund_id
                            {
                                reverse_lookups.push(storage_types::ReverseLookupNew {
                                    sk_id: field.clone(),
                                    lookup_id: format!(
                                        "{}_{}_{}",
                                        created_refund.merchant_id,
                                        connector_refund_id,
                                        created_refund.connector
                                    ),
                                    pk_id: key,
                                    source: "refund".to_string(),
                                })
                            };
                            storage_types::ReverseLookupNew::batch_insert(reverse_lookups, &conn)
                                .await
                                .change_context(errors::StorageError::KVError)?;

                            let redis_entry = kv::TypedSql {
                                op: kv::DBOperation::Insert {
                                    insertable: kv::Insertable::Refund(new),
                                },
                            };
                            self.push_to_drainer_stream::<storage_types::Refund>(
                                redis_entry,
                                PartitionKey::MerchantIdPaymentId {
                                    merchant_id: &created_refund.merchant_id,
                                    payment_id: &created_refund.payment_id,
                                },
                            )
                            .await?;

                            Ok(created_refund)
                        }
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &str,
            connector_transaction_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    storage_types::Refund::find_by_merchant_id_connector_transaction_id(
                        &conn,
                        merchant_id,
                        connector_transaction_id,
                    )
                    .await
                    .map_err(Into::into)
                    .into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("{merchant_id}_{connector_transaction_id}");
                    let lookup = match self.get_lookup_by_lookup_id(&lookup_id).await {
                        Ok(l) => l,
                        Err(err) => {
                            logger::error!(?err);
                            return Ok(vec![]);
                        }
                    };
                    let key = &lookup.pk_id;

                    let pattern = db_utils::generate_hscan_pattern_for_refund(&lookup.sk_id);

                    self.redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .hscan_and_deserialize(key, &pattern, None)
                        .await
                        .change_context(errors::StorageError::KVError)
                }
            }
        }

        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    this.update(&conn, refund)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{}_{}", this.merchant_id, this.refund_id);

                    let updated_refund = refund.clone().apply_changeset(this.clone());
                    // Check for database presence as well Maybe use a read replica here ?

                    let lookup = self.get_lookup_by_lookup_id(&key).await?;

                    let field = &lookup.sk_id;

                    let redis_value =
                        utils::Encode::<storage_types::Refund>::encode_to_string_of_json(
                            &updated_refund,
                        )
                        .change_context(errors::StorageError::SerializationFailed)?;

                    self.redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .set_hash_fields(&lookup.pk_id, (field, redis_value))
                        .await
                        .change_context(errors::StorageError::KVError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::RefundUpdate(kv::RefundUpdateMems {
                                orig: this,
                                update_data: refund,
                            }),
                        },
                    };
                    self.push_to_drainer_stream::<storage_types::Refund>(
                        redis_entry,
                        PartitionKey::MerchantIdPaymentId {
                            merchant_id: &updated_refund.merchant_id,
                            payment_id: &updated_refund.payment_id,
                        },
                    )
                    .await?;
                    Ok(updated_refund)
                }
            }
        }

        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &str,
            refund_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
                storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                    .await
                    .map_err(Into::into)
                    .into_report()
            };
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!("{merchant_id}_{refund_id}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;

                    let key = &lookup.pk_id;
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "Refund"),
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &str,
            connector_refund_id: &str,
            connector: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = pg_connection(&self.master_pool).await?;
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
                    let lookup_id = format!("{merchant_id}_{connector_refund_id}_{connector}");
                    let lookup = self.get_lookup_by_lookup_id(&lookup_id).await?;

                    let key = &lookup.pk_id;
                    db_utils::try_redis_get_else_try_database_get(
                        self.redis_conn()
                            .map_err(Into::<errors::StorageError>::into)?
                            .get_hash_field_and_deserialize(key, &lookup.sk_id, "Refund"),
                        database_call,
                    )
                    .await
                }
            }
        }

        // async fn find_refund_by_payment_id_merchant_id_refund_id(
        //     &self,
        //     payment_id: &str,
        //     merchant_id: &str,
        //     refund_id: &str,
        // ) -> CustomResult<Refund, errors::StorageError> {
        //     let conn = pg_connection(&self.master_pool).await;
        //     Refund::find_by_payment_id_merchant_id_refund_id(&conn, payment_id, merchant_id, refund_id)
        //         .await
        // }

        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.master_pool).await?;
                    storage_types::Refund::find_by_payment_id_merchant_id(
                        &conn,
                        payment_id,
                        merchant_id,
                    )
                    .await
                    .map_err(Into::into)
                    .into_report()
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("{merchant_id}_{payment_id}");
                    let lookup = self.get_lookup_by_lookup_id(&key).await?;

                    let pattern = db_utils::generate_hscan_pattern_for_refund(&lookup.sk_id);

                    self.redis_conn()
                        .map_err(Into::<errors::StorageError>::into)?
                        .hscan_and_deserialize(&key, &pattern, None)
                        .await
                        .change_context(errors::StorageError::KVError)
                }
            }
        }

        #[cfg(feature = "olap")]
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &str,
            refund_details: &api_models::refunds::RefundListRequest,
            storage_scheme: enums::MerchantStorageScheme,
            limit: i64,
        ) -> CustomResult<Vec<storage_models::refund::Refund>, errors::StorageError> {
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = pg_connection(&self.replica_pool).await?;
                    <storage_models::refund::Refund as storage_types::RefundDbExt>::filter_by_constraints(&conn, merchant_id, refund_details, limit)
                        .await
                        .map_err(Into::into)
                        .into_report()
                }

                enums::MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
            }
        }
    }
}

#[async_trait::async_trait]
impl RefundInterface for MockDb {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        _internal_reference_id: &str,
        _merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn insert_refund(
        &self,
        new: storage_types::RefundNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let mut refunds = self.refunds.lock().await;
        let current_time = common_utils::date_time::now();

        let refund = storage_types::Refund {
            #[allow(clippy::as_conversions)]
            id: refunds.len() as i32,
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
        };
        refunds.push(refund.clone());
        Ok(refund)
    }
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

    async fn update_refund(
        &self,
        _this: storage_types::Refund,
        _refund: storage_types::RefundUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

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

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    #[cfg(feature = "olap")]
    async fn filter_refund_by_constraints(
        &self,
        _merchant_id: &str,
        _refund_details: &api_models::refunds::RefundListRequest,
        _storage_scheme: enums::MerchantStorageScheme,
        _limit: i64,
    ) -> CustomResult<Vec<storage_models::refund::Refund>, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
