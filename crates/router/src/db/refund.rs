#[cfg(feature = "olap")]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "olap")]
use common_utils::types::{ConnectorTransactionIdTrait, MinorUnit};
use diesel_models::{errors::DatabaseError, refund::RefundUpdateInternal};
use hyperswitch_domain_models::refunds;

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
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError>;

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError>;

    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
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
        merchant_id: &common_utils::id_type::MerchantId,
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
        merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        storage_scheme: enums::MerchantStorageScheme,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn filter_refund_by_meta_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &common_utils::types::TimeRange,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_refund_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        constraints: &common_utils::types::TimeRange,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<(common_enums::RefundStatus, i64)>, errors::StorageError>;

    #[cfg(feature = "olap")]
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::report;
    use router_env::{instrument, tracing};

    use super::RefundInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{self as storage_types, enums},
    };

    #[async_trait::async_trait]
    impl RefundInterface for Store {
        #[instrument(skip_all)]
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &common_utils::id_type::MerchantId,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_internal_reference_id_merchant_id(
                &conn,
                internal_reference_id,
                merchant_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            new.insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
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
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update(&conn, refund)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
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
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &common_utils::id_type::PaymentId,
            merchant_id: &common_utils::id_type::MerchantId,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Refund::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_details: &refunds::RefundListConstraints,
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
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn filter_refund_by_meta_constraints(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
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
            .map_err(|error|report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn get_refund_status_with_count(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
            time_range: &api_models::payments::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<(common_enums::RefundStatus, i64)>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refund_status_with_count(&conn, merchant_id,profile_id_list, time_range)
            .await
            .map_err(|error|report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn get_total_count_of_refunds(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_details: &refunds::RefundListConstraints,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refunds_count(
                &conn,
                merchant_id,
                refund_details,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::{
        ext_traits::Encode, fallback_reverse_lookup_not_found, types::ConnectorTransactionIdTrait,
    };
    use error_stack::{report, ResultExt};
    use hyperswitch_domain_models::refunds;
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{
        decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey,
    };

    use super::RefundInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{self as storage_types, enums, kv},
        utils::db_utils,
    };
    #[async_trait::async_trait]
    impl RefundInterface for Store {
        #[instrument(skip_all)]
        async fn find_refund_by_internal_reference_id_merchant_id(
            &self,
            internal_reference_id: &str,
            merchant_id: &common_utils::id_type::MerchantId,
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
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!(
                        "ref_inter_ref_{}_{internal_reference_id}",
                        merchant_id.get_string_repr()
                    );
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = PartitionKey::CombinationKey {
                        combination: &lookup.pk_id,
                    };
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            ))
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

        #[instrument(skip_all)]
        async fn insert_refund(
            &self,
            new: storage_types::RefundNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Insert,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    new.insert(&conn)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let merchant_id = new.merchant_id.clone();
                    let payment_id = new.payment_id.clone();
                    let key = PartitionKey::MerchantIdPaymentId {
                        merchant_id: &merchant_id,
                        payment_id: &payment_id,
                    };
                    let key_str = key.to_string();
                    // TODO: need to add an application generated payment attempt id to distinguish between multiple attempts for the same payment id
                    // Check for database presence as well Maybe use a read replica here ?
                    let created_refund = storage_types::Refund {
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
                        created_at: new.created_at,
                        modified_at: new.created_at,
                        description: new.description.clone(),
                        refund_reason: new.refund_reason.clone(),
                        profile_id: new.profile_id.clone(),
                        updated_by: new.updated_by.clone(),
                        merchant_connector_id: new.merchant_connector_id.clone(),
                        charges: new.charges.clone(),
                        split_refunds: new.split_refunds.clone(),
                        organization_id: new.organization_id.clone(),
                        unified_code: None,
                        unified_message: None,
                        processor_refund_data: new.processor_refund_data.clone(),
                        processor_transaction_data: new.processor_transaction_data.clone(),
                        // Below fields are deprecated. Please add any new fields above this line.
                        connector_refund_data: None,
                        connector_transaction_data: None,
                    };

                    let field = format!(
                        "pa_{}_ref_{}",
                        &created_refund.attempt_id, &created_refund.refund_id
                    );

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: Box::new(kv::Insertable::Refund(new)),
                        },
                    };

                    let mut reverse_lookups = vec![
                        storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_ref_id_{}_{}",
                                created_refund.merchant_id.get_string_repr(),
                                created_refund.refund_id
                            ),
                            pk_id: key_str.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        },
                        // [#492]: A discussion is required on whether this is required?
                        storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_inter_ref_{}_{}",
                                created_refund.merchant_id.get_string_repr(),
                                created_refund.internal_reference_id
                            ),
                            pk_id: key_str.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        },
                    ];
                    if let Some(connector_refund_id) =
                        created_refund.to_owned().get_optional_connector_refund_id()
                    {
                        reverse_lookups.push(storage_types::ReverseLookupNew {
                            sk_id: field.clone(),
                            lookup_id: format!(
                                "ref_connector_{}_{}_{}",
                                created_refund.merchant_id.get_string_repr(),
                                connector_refund_id,
                                created_refund.connector
                            ),
                            pk_id: key_str.clone(),
                            source: "refund".to_string(),
                            updated_by: storage_scheme.to_string(),
                        })
                    };
                    let rev_look = reverse_lookups
                        .into_iter()
                        .map(|rev| self.insert_reverse_lookup(rev, storage_scheme));

                    futures::future::try_join_all(rev_look).await?;

                    match Box::pin(kv_wrapper::<storage_types::Refund, _, _>(
                        self,
                        KvOperation::<storage_types::Refund>::HSetNx(
                            &field,
                            &created_refund,
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "refund",
                            key: Some(created_refund.refund_id),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(created_refund),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_connector_transaction_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
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
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!(
                        "pa_conn_trans_{}_{connector_transaction_id}",
                        merchant_id.get_string_repr()
                    );
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = PartitionKey::CombinationKey {
                        combination: &lookup.pk_id,
                    };

                    let pattern = db_utils::generate_hscan_pattern_for_refund(&lookup.sk_id);

                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::Scan(&pattern),
                                key,
                            ))
                            .await?
                            .try_into_scan()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

        #[instrument(skip_all)]
        async fn update_refund(
            &self,
            this: storage_types::Refund,
            refund: storage_types::RefundUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let merchant_id = this.merchant_id.clone();
            let payment_id = this.payment_id.clone();
            let key = PartitionKey::MerchantIdPaymentId {
                merchant_id: &merchant_id,
                payment_id: &payment_id,
            };
            let field = format!("pa_{}_ref_{}", &this.attempt_id, &this.refund_id);
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Update(key.clone(), &field, Some(&this.updated_by)),
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    this.update(&conn, refund)
                        .await
                        .map_err(|error| report!(errors::StorageError::from(error)))
                }
                enums::MerchantStorageScheme::RedisKv => {
                    let key_str = key.to_string();
                    let updated_refund = refund.clone().apply_changeset(this.clone());

                    let redis_value = updated_refund
                        .encode_to_string_of_json()
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: Box::new(kv::Updateable::RefundUpdate(Box::new(
                                kv::RefundUpdateMems {
                                    orig: this,
                                    update_data: refund,
                                },
                            ))),
                        },
                    };

                    Box::pin(kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<storage_types::Refund>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_refund)
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_refund_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_types::Refund, errors::StorageError> {
            let database_call = || async {
                let conn = connection::pg_connection_read(self).await?;
                storage_types::Refund::find_by_merchant_id_refund_id(&conn, merchant_id, refund_id)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id =
                        format!("ref_ref_id_{}_{refund_id}", merchant_id.get_string_repr());
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = PartitionKey::CombinationKey {
                        combination: &lookup.pk_id,
                    };
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            ))
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_refund_by_merchant_id_connector_refund_id_connector(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
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
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!(
                        "ref_connector_{}_{connector_refund_id}_{connector}",
                        merchant_id.get_string_repr()
                    );
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        database_call().await
                    );

                    let key = PartitionKey::CombinationKey {
                        combination: &lookup.pk_id,
                    };
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::HGet(&lookup.sk_id),
                                key,
                            ))
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_refund_by_payment_id_merchant_id(
            &self,
            payment_id: &common_utils::id_type::PaymentId,
            merchant_id: &common_utils::id_type::MerchantId,
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
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, storage_types::Refund>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;
            match storage_scheme {
                enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                enums::MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdPaymentId {
                        merchant_id,
                        payment_id,
                    };
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper(
                                self,
                                KvOperation::<storage_types::Refund>::Scan("pa_*_ref_*"),
                                key,
                            ))
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
        #[instrument(skip_all)]
        async fn filter_refund_by_constraints(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_details: &refunds::RefundListConstraints,
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
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn filter_refund_by_meta_constraints(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_details: &common_utils::types::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<api_models::refunds::RefundListMetaData, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::filter_by_meta_constraints(&conn, merchant_id, refund_details)
                        .await
                        .map_err(|error|report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn get_refund_status_with_count(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
            constraints: &common_utils::types::TimeRange,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<Vec<(common_enums::RefundStatus, i64)>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refund_status_with_count(&conn, merchant_id,profile_id_list, constraints)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[cfg(feature = "olap")]
        #[instrument(skip_all)]
        async fn get_total_count_of_refunds(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            refund_details: &refunds::RefundListConstraints,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<i64, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            <diesel_models::refund::Refund as storage_types::RefundDbExt>::get_refunds_count(
                &conn,
                merchant_id,
                refund_details,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[async_trait::async_trait]
impl RefundInterface for MockDb {
    async fn find_refund_by_internal_reference_id_merchant_id(
        &self,
        internal_reference_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;
        refunds
            .iter()
            .find(|refund| {
                refund.merchant_id == *merchant_id
                    && refund.internal_reference_id == internal_reference_id
            })
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

    async fn insert_refund(
        &self,
        new: storage_types::RefundNew,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let mut refunds = self.refunds.lock().await;
        let current_time = common_utils::date_time::now();

        let refund = storage_types::Refund {
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
            created_at: new.created_at,
            modified_at: current_time,
            description: new.description,
            refund_reason: new.refund_reason.clone(),
            profile_id: new.profile_id,
            updated_by: new.updated_by,
            merchant_connector_id: new.merchant_connector_id,
            charges: new.charges,
            split_refunds: new.split_refunds,
            organization_id: new.organization_id,
            unified_code: None,
            unified_message: None,
            processor_refund_data: new.processor_refund_data.clone(),
            processor_transaction_data: new.processor_transaction_data.clone(),
            // Below fields are deprecated. Please add any new fields above this line.
            connector_refund_data: None,
            connector_transaction_data: None,
        };
        refunds.push(refund.clone());
        Ok(refund)
    }
    async fn find_refund_by_merchant_id_connector_transaction_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_transaction_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        Ok(refunds
            .iter()
            .take_while(|refund| {
                refund.merchant_id == *merchant_id
                    && refund.get_connector_transaction_id() == connector_transaction_id
            })
            .cloned()
            .collect::<Vec<_>>())
    }

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

    async fn find_refund_by_merchant_id_refund_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        refunds
            .iter()
            .find(|refund| refund.merchant_id == *merchant_id && refund.refund_id == refund_id)
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

    async fn find_refund_by_merchant_id_connector_refund_id_connector(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_refund_id: &str,
        connector: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_types::Refund, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        refunds
            .iter()
            .find(|refund| {
                refund.merchant_id == *merchant_id
                    && refund
                        .get_optional_connector_refund_id()
                        .map(|refund_id| refund_id.as_str())
                        == Some(connector_refund_id)
                    && refund.connector == connector
            })
            .cloned()
            .ok_or_else(|| {
                errors::StorageError::DatabaseError(DatabaseError::NotFound.into()).into()
            })
    }

    async fn find_refund_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<storage_types::Refund>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        Ok(refunds
            .iter()
            .filter(|refund| refund.merchant_id == *merchant_id && refund.payment_id == *payment_id)
            .cloned()
            .collect::<Vec<_>>())
    }

    #[cfg(feature = "olap")]
    async fn filter_refund_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        _storage_scheme: enums::MerchantStorageScheme,
        limit: i64,
        offset: i64,
    ) -> CustomResult<Vec<diesel_models::refund::Refund>, errors::StorageError> {
        let mut unique_connectors = HashSet::new();
        let mut unique_merchant_connector_ids = HashSet::new();
        let mut unique_currencies = HashSet::new();
        let mut unique_statuses = HashSet::new();
        let mut unique_profile_ids = HashSet::new();

        // Fill the hash sets with data from refund_details
        if let Some(connectors) = &refund_details.connector {
            connectors.iter().for_each(|connector| {
                unique_connectors.insert(connector);
            });
        }

        if let Some(merchant_connector_ids) = &refund_details.merchant_connector_id {
            merchant_connector_ids
                .iter()
                .for_each(|unique_merchant_connector_id| {
                    unique_merchant_connector_ids.insert(unique_merchant_connector_id);
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

        if let Some(profile_id_list) = &refund_details.profile_id {
            unique_profile_ids = profile_id_list.iter().collect();
        }

        let refunds = self.refunds.lock().await;
        let filtered_refunds = refunds
            .iter()
            .filter(|refund| refund.merchant_id == *merchant_id)
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
            .filter(|refund| {
                refund.profile_id.as_ref().is_some_and(|profile_id| {
                    unique_profile_ids.is_empty() || unique_profile_ids.contains(profile_id)
                })
            })
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
                refund_details
                    .amount_filter
                    .as_ref()
                    .map_or(true, |amount| {
                        refund.refund_amount
                            >= MinorUnit::new(amount.start_amount.unwrap_or(i64::MIN))
                            && refund.refund_amount
                                <= MinorUnit::new(amount.end_amount.unwrap_or(i64::MAX))
                    })
            })
            .filter(|refund| {
                unique_connectors.is_empty() || unique_connectors.contains(&refund.connector)
            })
            .filter(|refund| {
                unique_merchant_connector_ids.is_empty()
                    || refund
                        .merchant_connector_id
                        .as_ref()
                        .is_some_and(|id| unique_merchant_connector_ids.contains(id))
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
    async fn filter_refund_by_meta_constraints(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &common_utils::types::TimeRange,
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
    async fn get_refund_status_with_count(
        &self,
        _merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<Vec<(api_models::enums::RefundStatus, i64)>, errors::StorageError> {
        let refunds = self.refunds.lock().await;

        let start_time = time_range.start_time;
        let end_time = time_range
            .end_time
            .unwrap_or_else(common_utils::date_time::now);

        let filtered_refunds = refunds
            .iter()
            .filter(|refund| {
                refund.created_at >= start_time
                    && refund.created_at <= end_time
                    && profile_id_list
                        .as_ref()
                        .zip(refund.profile_id.as_ref())
                        .map(|(received_profile_list, received_profile_id)| {
                            received_profile_list.contains(received_profile_id)
                        })
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<diesel_models::refund::Refund>>();

        let mut refund_status_counts: HashMap<api_models::enums::RefundStatus, i64> =
            HashMap::new();

        for refund in filtered_refunds {
            *refund_status_counts
                .entry(refund.refund_status)
                .or_insert(0) += 1;
        }

        let result: Vec<(api_models::enums::RefundStatus, i64)> =
            refund_status_counts.into_iter().collect();

        Ok(result)
    }

    #[cfg(feature = "olap")]
    async fn get_total_count_of_refunds(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        refund_details: &refunds::RefundListConstraints,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        let mut unique_connectors = HashSet::new();
        let mut unique_merchant_connector_ids = HashSet::new();
        let mut unique_currencies = HashSet::new();
        let mut unique_statuses = HashSet::new();
        let mut unique_profile_ids = HashSet::new();

        // Fill the hash sets with data from refund_details
        if let Some(connectors) = &refund_details.connector {
            connectors.iter().for_each(|connector| {
                unique_connectors.insert(connector);
            });
        }

        if let Some(merchant_connector_ids) = &refund_details.merchant_connector_id {
            merchant_connector_ids
                .iter()
                .for_each(|unique_merchant_connector_id| {
                    unique_merchant_connector_ids.insert(unique_merchant_connector_id);
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

        if let Some(profile_id_list) = &refund_details.profile_id {
            unique_profile_ids = profile_id_list.iter().collect();
        }

        let refunds = self.refunds.lock().await;
        let filtered_refunds = refunds
            .iter()
            .filter(|refund| refund.merchant_id == *merchant_id)
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
            .filter(|refund| {
                refund.profile_id.as_ref().is_some_and(|profile_id| {
                    unique_profile_ids.is_empty() || unique_profile_ids.contains(profile_id)
                })
            })
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
                refund_details
                    .amount_filter
                    .as_ref()
                    .map_or(true, |amount| {
                        refund.refund_amount
                            >= MinorUnit::new(amount.start_amount.unwrap_or(i64::MIN))
                            && refund.refund_amount
                                <= MinorUnit::new(amount.end_amount.unwrap_or(i64::MAX))
                    })
            })
            .filter(|refund| {
                unique_connectors.is_empty() || unique_connectors.contains(&refund.connector)
            })
            .filter(|refund| {
                unique_merchant_connector_ids.is_empty()
                    || refund
                        .merchant_connector_id
                        .as_ref()
                        .is_some_and(|id| unique_merchant_connector_ids.contains(id))
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
