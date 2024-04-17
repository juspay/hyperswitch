use error_stack::ResultExt;

use super::MockDb;
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as storage_types, enums::MerchantStorageScheme},
};

#[async_trait::async_trait]
pub trait MandateInterface {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError>;

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError>;

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError>;

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate_update: storage_types::MandateUpdate,
        mandate: storage_types::Mandate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError>;

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError>;

    async fn insert_mandate(
        &self,
        mandate: storage_types::MandateNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError>;
}

#[cfg(feature = "kv_store")]
mod storage {
    use common_utils::fallback_reverse_lookup_not_found;
    use diesel_models::kv;
    use error_stack::{report, ResultExt};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation, PartitionKey};

    use super::MandateInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage::{self as storage_types, enums::MerchantStorageScheme, MandateDbExt},
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl MandateInterface for Store {
        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_mandate_id(
            &self,
            merchant_id: &str,
            mandate_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Mandate::find_by_merchant_id_mandate_id(
                    &conn,
                    merchant_id,
                    mandate_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };

            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdMandateId {
                        merchant_id,
                        mandate_id,
                    };
                    let field = format!("mandate_{}", mandate_id);

                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Mandate>::HGet(&field),
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

        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_connector_mandate_id(
            &self,
            merchant_id: &str,
            connector_mandate_id: &str,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_types::Mandate::find_by_merchant_id_connector_mandate_id(
                    &conn,
                    merchant_id,
                    connector_mandate_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };

            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => database_call().await,
                MerchantStorageScheme::RedisKv => {
                    let lookup_id =
                        format!("mid_{}_conn_mandate_{}", merchant_id, connector_mandate_id);
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
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Mandate>::HGet(&lookup.sk_id),
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

        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_customer_id(
            &self,
            merchant_id: &str,
            customer_id: &str,
        ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn update_mandate_by_merchant_id_mandate_id(
            &self,
            merchant_id: &str,
            mandate_id: &str,
            mandate_update: storage_types::MandateUpdate,
            mandate: storage_types::Mandate,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;

            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => {
                    storage_types::Mandate::update_by_merchant_id_mandate_id(
                        &conn,
                        merchant_id,
                        mandate_id,
                        storage_types::MandateUpdateInternal::from(mandate_update),
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                }
                MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdMandateId {
                        merchant_id,
                        mandate_id,
                    };
                    let field = format!("mandate_{}", mandate_id);
                    let key_str = key.to_string();

                    if let diesel_models::MandateUpdate::ConnectorMandateIdUpdate {
                        connector_mandate_id: Some(val),
                        ..
                    } = &mandate_update
                    {
                        let rev_lookup = diesel_models::ReverseLookupNew {
                            sk_id: field.clone(),
                            pk_id: key_str.clone(),
                            lookup_id: format!("mid_{}_conn_mandate_{}", merchant_id, val),
                            source: "mandate".to_string(),
                            updated_by: storage_scheme.to_string(),
                        };
                        // dont fail request if reverse lookup entry fails, as it might be inserted during insert
                        let _ = self.insert_reverse_lookup(rev_lookup, storage_scheme)
                            .await;
                    }

                    let m_update = diesel_models::MandateUpdateInternal::from(mandate_update);
                    let updated_mandate = m_update.clone().apply_changeset(mandate.clone());

                    let redis_value = serde_json::to_string(&updated_mandate)
                        .change_context(errors::StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::MandateUpdate(kv::MandateUpdateMems {
                                orig: mandate,
                                update_data: m_update,
                            }),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::<diesel_models::Mandate>::Hset(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_mandate)
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_mandates_by_merchant_id(
            &self,
            merchant_id: &str,
            mandate_constraints: api_models::mandates::MandateListConstraints,
        ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::filter_by_constraints(&conn, merchant_id, mandate_constraints)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn insert_mandate(
            &self,
            mandate: storage_types::MandateNew,
            storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;

            match storage_scheme {
                MerchantStorageScheme::PostgresOnly => mandate
                    .insert(&conn)
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error))),
                MerchantStorageScheme::RedisKv => {
                    let mandate_id = mandate.mandate_id.clone();
                    let merchant_id = mandate.merchant_id.clone();
                    let connector_mandate_id = mandate.connector_mandate_id.clone();

                    let key = PartitionKey::MerchantIdMandateId {
                        merchant_id: merchant_id.as_str(),
                        mandate_id: mandate_id.as_str(),
                    };
                    let key_str = key.to_string();
                    let field = format!("mandate_{}", mandate_id);

                    let storage_mandate = storage_types::Mandate::from(&mandate);

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: kv::Insertable::Mandate(mandate),
                        },
                    };

                    if let Some(connector_val) = connector_mandate_id {
                        let lookup_id =
                            format!("mid_{}_conn_mandate_{}", merchant_id, connector_val);

                        let reverse_lookup_entry = diesel_models::ReverseLookupNew {
                            sk_id: field.clone(),
                            pk_id: key_str.clone(),
                            lookup_id,
                            source: "mandate".to_string(),
                            updated_by: storage_scheme.to_string(),
                        };

                        self.insert_reverse_lookup(reverse_lookup_entry, storage_scheme)
                            .await?;
                    }

                    match kv_wrapper::<diesel_models::Mandate, _, _>(
                        self,
                        KvOperation::<diesel_models::Mandate>::HSetNx(
                            &field,
                            &storage_mandate,
                            redis_entry,
                        ),
                        key,
                    )
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "mandate",
                            key: Some(storage_mandate.mandate_id),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(storage_mandate),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::report;
    use router_env::{instrument, tracing};

    use super::MandateInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::{self as storage_types, enums::MerchantStorageScheme, MandateDbExt},
    };

    #[async_trait::async_trait]
    impl MandateInterface for Store {
        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_mandate_id(
            &self,
            merchant_id: &str,
            mandate_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::find_by_merchant_id_mandate_id(&conn, merchant_id, mandate_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_connector_mandate_id(
            &self,
            merchant_id: &str,
            connector_mandate_id: &str,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::find_by_merchant_id_connector_mandate_id(
                &conn,
                merchant_id,
                connector_mandate_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_mandate_by_merchant_id_customer_id(
            &self,
            merchant_id: &str,
            customer_id: &str,
        ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn update_mandate_by_merchant_id_mandate_id(
            &self,
            merchant_id: &str,
            mandate_id: &str,
            mandate_update: storage_types::MandateUpdate,
            _mandate: storage_types::Mandate,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage_types::Mandate::update_by_merchant_id_mandate_id(
                &conn,
                merchant_id,
                mandate_id,
                storage_types::MandateUpdateInternal::from(mandate_update),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn find_mandates_by_merchant_id(
            &self,
            merchant_id: &str,
            mandate_constraints: api_models::mandates::MandateListConstraints,
        ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_types::Mandate::filter_by_constraints(&conn, merchant_id, mandate_constraints)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        #[instrument(skip_all)]
        async fn insert_mandate(
            &self,
            mandate: storage_types::MandateNew,
            _storage_scheme: MerchantStorageScheme,
        ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            mandate
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}

#[async_trait::async_trait]
impl MandateInterface for MockDb {
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
        self.mandates
            .lock()
            .await
            .iter()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
            .cloned()
            .ok_or_else(|| errors::StorageError::ValueNotFound("mandate not found".to_string()))
            .map_err(|err| err.into())
    }

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &str,
        connector_mandate_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
        self.mandates
            .lock()
            .await
            .iter()
            .find(|mandate| {
                mandate.merchant_id == merchant_id
                    && mandate.connector_mandate_id == Some(connector_mandate_id.to_string())
            })
            .cloned()
            .ok_or_else(|| errors::StorageError::ValueNotFound("mandate not found".to_string()))
            .map_err(|err| err.into())
    }

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &str,
        customer_id: &str,
    ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
        return Ok(self
            .mandates
            .lock()
            .await
            .iter()
            .filter(|mandate| {
                mandate.merchant_id == merchant_id && mandate.customer_id == customer_id
            })
            .cloned()
            .collect());
    }

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &str,
        mandate_id: &str,
        mandate_update: storage_types::MandateUpdate,
        _mandate: storage_types::Mandate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
        let mut mandates = self.mandates.lock().await;
        match mandates
            .iter_mut()
            .find(|mandate| mandate.merchant_id == merchant_id && mandate.mandate_id == mandate_id)
        {
            Some(mandate) => {
                match mandate_update {
                    storage_types::MandateUpdate::StatusUpdate { mandate_status } => {
                        mandate.mandate_status = mandate_status;
                    }
                    storage_types::MandateUpdate::CaptureAmountUpdate { amount_captured } => {
                        mandate.amount_captured = amount_captured;
                    }
                    storage_types::MandateUpdate::ConnectorReferenceUpdate {
                        connector_mandate_ids,
                    } => {
                        mandate.connector_mandate_ids = connector_mandate_ids;
                    }

                    diesel_models::MandateUpdate::ConnectorMandateIdUpdate {
                        connector_mandate_id,
                        connector_mandate_ids,
                        payment_method_id,
                        original_payment_id,
                    } => {
                        mandate.connector_mandate_ids = connector_mandate_ids;
                        mandate.connector_mandate_id = connector_mandate_id;
                        mandate.payment_method_id = payment_method_id;
                        mandate.original_payment_id = original_payment_id
                    }
                }
                Ok(mandate.clone())
            }
            None => {
                Err(errors::StorageError::ValueNotFound("mandate not found".to_string()).into())
            }
        }
    }

    async fn find_mandates_by_merchant_id(
        &self,
        merchant_id: &str,
        mandate_constraints: api_models::mandates::MandateListConstraints,
    ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
        let mandates = self.mandates.lock().await;
        let mandates_iter = mandates.iter().filter(|mandate| {
            let mut checker = mandate.merchant_id == merchant_id;
            if let Some(created_time) = mandate_constraints.created_time {
                checker &= mandate.created_at == created_time;
            }
            if let Some(created_time_lt) = mandate_constraints.created_time_lt {
                checker &= mandate.created_at < created_time_lt;
            }
            if let Some(created_time_gt) = mandate_constraints.created_time_gt {
                checker &= mandate.created_at > created_time_gt;
            }
            if let Some(created_time_lte) = mandate_constraints.created_time_lte {
                checker &= mandate.created_at <= created_time_lte;
            }
            if let Some(created_time_gte) = mandate_constraints.created_time_gte {
                checker &= mandate.created_at >= created_time_gte;
            }
            if let Some(connector) = &mandate_constraints.connector {
                checker &= mandate.connector == *connector;
            }
            if let Some(mandate_status) = mandate_constraints.mandate_status {
                checker &= mandate.mandate_status == mandate_status;
            }
            checker
        });

        #[allow(clippy::as_conversions)]
        let offset = (if mandate_constraints.offset.unwrap_or(0) < 0 {
            0
        } else {
            mandate_constraints.offset.unwrap_or(0)
        }) as usize;

        let mandates: Vec<storage_types::Mandate> = if let Some(limit) = mandate_constraints.limit {
            #[allow(clippy::as_conversions)]
            mandates_iter
                .skip(offset)
                .take((if limit < 0 { 0 } else { limit }) as usize)
                .cloned()
                .collect()
        } else {
            mandates_iter.skip(offset).cloned().collect()
        };
        Ok(mandates)
    }

    async fn insert_mandate(
        &self,
        mandate_new: storage_types::MandateNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
        let mut mandates = self.mandates.lock().await;
        let mandate = storage_types::Mandate {
            id: i32::try_from(mandates.len()).change_context(errors::StorageError::MockDbError)?,
            mandate_id: mandate_new.mandate_id.clone(),
            customer_id: mandate_new.customer_id,
            merchant_id: mandate_new.merchant_id,
            original_payment_id: mandate_new.original_payment_id,
            payment_method_id: mandate_new.payment_method_id,
            mandate_status: mandate_new.mandate_status,
            mandate_type: mandate_new.mandate_type,
            customer_accepted_at: mandate_new.customer_accepted_at,
            customer_ip_address: mandate_new.customer_ip_address,
            customer_user_agent: mandate_new.customer_user_agent,
            network_transaction_id: mandate_new.network_transaction_id,
            previous_attempt_id: mandate_new.previous_attempt_id,
            created_at: mandate_new
                .created_at
                .unwrap_or_else(common_utils::date_time::now),
            mandate_amount: mandate_new.mandate_amount,
            mandate_currency: mandate_new.mandate_currency,
            amount_captured: mandate_new.amount_captured,
            connector: mandate_new.connector,
            connector_mandate_id: mandate_new.connector_mandate_id,
            start_date: mandate_new.start_date,
            end_date: mandate_new.end_date,
            metadata: mandate_new.metadata,
            connector_mandate_ids: mandate_new.connector_mandate_ids,
            merchant_connector_id: mandate_new.merchant_connector_id,
        };
        mandates.push(mandate.clone());
        Ok(mandate)
    }
}
