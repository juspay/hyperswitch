use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    core::errors::{self, CustomResult},
    types::storage::{self as storage_type, enums},
};

#[async_trait::async_trait]
pub trait ConnectorResponseInterface {
    async fn insert_connector_response(
        &self,
        connector_response: storage_type::ConnectorResponseNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError>;

    async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError>;

    async fn update_connector_response(
        &self,
        this: storage_type::ConnectorResponse,
        payment_attempt: storage_type::ConnectorResponseUpdate,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use error_stack::IntoReport;
    use router_env::{instrument, tracing};

    use super::Store;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        types::storage::{self as storage_type, enums},
    };

    #[async_trait::async_trait]
    impl super::ConnectorResponseInterface for Store {
        #[instrument(skip_all)]
        async fn insert_connector_response(
            &self,
            connector_response: storage_type::ConnectorResponseNew,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            connector_response
                .insert(&conn)
                .await
                .map_err(Into::into)
                .into_report()
        }

        #[instrument(skip_all)]
        async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            attempt_id: &str,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage_type::ConnectorResponse::find_by_payment_id_merchant_id_attempt_id(
                &conn,
                payment_id,
                merchant_id,
                attempt_id,
            )
            .await
            .map_err(Into::into)
            .into_report()
        }

        async fn update_connector_response(
            &self,
            this: storage_type::ConnectorResponse,
            connector_response_update: storage_type::ConnectorResponseUpdate,
            _storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update(&conn, connector_response_update)
                .await
                .map_err(Into::into)
                .into_report()
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {

    use diesel_models::enums as storage_enums;
    use error_stack::{IntoReport, ResultExt};
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{kv_wrapper, KvOperation};

    use super::Store;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        types::storage::{self as storage_type, enums, kv},
        utils::db_utils,
    };

    #[async_trait::async_trait]
    impl super::ConnectorResponseInterface for Store {
        #[instrument(skip_all)]
        async fn insert_connector_response(
            &self,
            connector_response: storage_type::ConnectorResponseNew,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;

            match storage_scheme {
                storage_enums::MerchantStorageScheme::PostgresOnly => connector_response
                    .insert(&conn)
                    .await
                    .map_err(Into::into)
                    .into_report(),
                storage_enums::MerchantStorageScheme::RedisKv => {
                    let merchant_id = &connector_response.merchant_id;
                    let payment_id = &connector_response.payment_id;
                    let attempt_id = &connector_response.attempt_id;

                    let key = format!("mid_{merchant_id}_pid_{payment_id}");
                    let field = format!("connector_resp_{merchant_id}_{payment_id}_{attempt_id}");

                    let created_connector_resp = storage_type::ConnectorResponse {
                        id: Default::default(),
                        payment_id: connector_response.payment_id.clone(),
                        merchant_id: connector_response.merchant_id.clone(),
                        attempt_id: connector_response.attempt_id.clone(),
                        created_at: connector_response.created_at,
                        modified_at: connector_response.modified_at,
                        connector_name: connector_response.connector_name.clone(),
                        connector_transaction_id: connector_response
                            .connector_transaction_id
                            .clone(),
                        authentication_data: connector_response.authentication_data.clone(),
                        encoded_data: connector_response.encoded_data.clone(),
                        updated_by: storage_scheme.to_string(),
                    };

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: kv::Insertable::ConnectorResponse(
                                connector_response.clone(),
                            ),
                        },
                    };

                    match kv_wrapper::<storage_type::ConnectorResponse, _, _>(
                        self,
                        KvOperation::HSetNx(&field, &created_connector_resp, redis_entry),
                        &key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                            entity: "address",
                            key: Some(key),
                        })
                        .into_report(),
                        Ok(HsetnxReply::KeySet) => Ok(created_connector_resp),
                        Err(er) => Err(er).change_context(errors::StorageError::KVError),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
            attempt_id: &str,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage_type::ConnectorResponse::find_by_payment_id_merchant_id_attempt_id(
                    &conn,
                    payment_id,
                    merchant_id,
                    attempt_id,
                )
                .await
                .map_err(Into::into)
                .into_report()
            };
            match storage_scheme {
                storage_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
                storage_enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{merchant_id}_pid_{payment_id}");
                    let field = format!("connector_resp_{merchant_id}_{payment_id}_{attempt_id}");

                    db_utils::try_redis_get_else_try_database_get(
                        async {
                            kv_wrapper(
                                self,
                                KvOperation::<diesel_models::Address>::HGet(&field),
                                key,
                            )
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    )
                    .await
                }
            }
        }

        async fn update_connector_response(
            &self,
            this: storage_type::ConnectorResponse,
            connector_response_update: storage_type::ConnectorResponseUpdate,
            storage_scheme: enums::MerchantStorageScheme,
        ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            match storage_scheme {
                storage_enums::MerchantStorageScheme::PostgresOnly => this
                    .update(&conn, connector_response_update)
                    .await
                    .map_err(Into::into)
                    .into_report(),
                storage_enums::MerchantStorageScheme::RedisKv => {
                    let key = format!("mid_{}_pid_{}", this.merchant_id, this.payment_id);
                    let updated_connector_response = connector_response_update
                        .clone()
                        .apply_changeset(this.clone());
                    let redis_value = serde_json::to_string(&updated_connector_response)
                        .into_report()
                        .change_context(errors::StorageError::KVError)?;
                    let field = format!(
                        "connector_resp_{}_{}_{}",
                        &updated_connector_response.merchant_id,
                        &updated_connector_response.payment_id,
                        &updated_connector_response.attempt_id
                    );

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: kv::Updateable::ConnectorResponseUpdate(
                                kv::ConnectorResponseUpdateMems {
                                    orig: this,
                                    update_data: connector_response_update,
                                },
                            ),
                        },
                    };

                    kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::Hset::<storage_type::ConnectorResponse>(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        &key,
                    )
                    .await
                    .change_context(errors::StorageError::KVError)?
                    .try_into_hset()
                    .change_context(errors::StorageError::KVError)?;

                    Ok(updated_connector_response)
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl ConnectorResponseInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_connector_response(
        &self,
        new: storage_type::ConnectorResponseNew,
        storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = storage_type::ConnectorResponse {
            id: connector_response
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
            payment_id: new.payment_id,
            merchant_id: new.merchant_id,
            attempt_id: new.attempt_id,
            created_at: new.created_at,
            modified_at: new.modified_at,
            connector_name: new.connector_name,
            connector_transaction_id: new.connector_transaction_id,
            authentication_data: new.authentication_data,
            encoded_data: new.encoded_data,
            updated_by: storage_scheme.to_string(),
        };
        connector_response.push(response.clone());
        Ok(response)
    }

    #[instrument(skip_all)]
    async fn find_connector_response_by_payment_id_merchant_id_attempt_id(
        &self,
        _payment_id: &str,
        _merchant_id: &str,
        _attempt_id: &str,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    // safety: interface only used for testing
    #[allow(clippy::unwrap_used)]
    async fn update_connector_response(
        &self,
        this: storage_type::ConnectorResponse,
        connector_response_update: storage_type::ConnectorResponseUpdate,
        _storage_scheme: enums::MerchantStorageScheme,
    ) -> CustomResult<storage_type::ConnectorResponse, errors::StorageError> {
        let mut connector_response = self.connector_response.lock().await;
        let response = connector_response
            .iter_mut()
            .find(|item| item.id == this.id)
            .unwrap();
        *response = connector_response_update.apply_changeset(response.clone());
        Ok(response.clone())
    }
}
