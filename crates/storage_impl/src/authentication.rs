use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode},
    fallback_reverse_lookup_not_found,
};
use diesel_models::{
    authentication::Authentication as diesel_authentication, reverse_lookup::ReverseLookupNew,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    authentication::{Authentication, AuthenticationInterface, AuthenticationUpdate},
    behaviour::Conversion,
    merchant_key_store::MerchantKeyStore,
};
use redis_interface::HsetnxReply;
use router_env::{instrument, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::{self, RedisErrorExt},
    kv_router_store::KVRouterStore,
    lookup::ReverseLookupInterface,
    mock_db::MockDb,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{pg_connection_read, pg_connection_write, try_redis_get_else_try_database_get},
    DatabaseStore, RouterStore,
};

impl crate::redis::kv_store::KvStorePartition for diesel_authentication {}

/// Insert the connector-authentication-id reverse lookup (webhook find path).
#[inline]
#[instrument(skip_all)]
async fn add_connector_authentication_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    field: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    connector_authentication_id: &str,
    storage_scheme: common_enums::MerchantStorageScheme,
) -> error_stack::Result<diesel_models::reverse_lookup::ReverseLookup, errors::StorageError> {
    let reverse_lookup = ReverseLookupNew {
        lookup_id: diesel_authentication::get_connector_authentication_lookup_id(
            merchant_id,
            connector_authentication_id,
        ),
        pk_id: key.to_owned(),
        sk_id: field.to_owned(),
        source: "authentication".to_string(),
        updated_by: storage_scheme.to_string(),
    };
    store
        .insert_reverse_lookup(reverse_lookup, storage_scheme)
        .await
}

#[async_trait::async_trait]
impl<T: DatabaseStore> AuthenticationInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_authentication(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: Authentication,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        let inserted_authentication = Box::pin(
            authentication
                .construct_new()
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .insert(&conn),
        )
        .await
        .map_err(|error| {
            let new_err = diesel_error_to_data_error(*error.current_context());
            error.change_context(new_err)
        })?;
        Authentication::convert_back(
            state,
            inserted_authentication,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_processor_merchant_id_authentication_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        // Stagger release: try processor_merchant_id, fall back to merchant_id for legacy NULL rows.
        let result = diesel_authentication::find_by_processor_merchant_id_authentication_id(
            &conn,
            processor_merchant_id,
            authentication_id,
        )
        .await;
        let result = match result {
            Err(error)
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) =>
            {
                diesel_authentication::find_by_merchant_id_authentication_id(
                    &conn,
                    processor_merchant_id,
                    authentication_id,
                )
                .await
            }
            other => other,
        };
        result
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
            .async_and_then(|authentication| async {
                Authentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_processor_merchant_id_connector_authentication_id(
        &self,
        processor_merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        // Stagger release: try processor_merchant_id, fall back to merchant_id for legacy NULL rows.
        let result = diesel_authentication::find_authentication_by_processor_merchant_id_connector_authentication_id(
            &conn,
            &processor_merchant_id,
            &connector_authentication_id,
        )
        .await;
        let result = match result {
            Err(error)
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) =>
            {
                diesel_authentication::find_authentication_by_merchant_id_connector_authentication_id(
                    &conn,
                    &processor_merchant_id,
                    &connector_authentication_id,
                )
                .await
            }
            other => other,
        };
        result
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
            .async_and_then(|authentication| async {
                Authentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    #[instrument(skip_all)]
    async fn update_authentication_by_processor_merchant_id_authentication_id(
        &self,
        previous_state: Authentication,
        authentication_update: AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        let authentication_update_internal =
            diesel_models::authentication::AuthenticationUpdateInternal::from(
                diesel_models::authentication::AuthenticationUpdate::from(authentication_update),
            );
        // Stagger release: try processor_merchant_id, fall back to merchant_id for legacy NULL rows.
        let processor_merchant_id = previous_state
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| previous_state.merchant_id.clone());
        let result = Box::pin(
            diesel_authentication::update_by_processor_merchant_id_authentication_id(
                &conn,
                &processor_merchant_id,
                &previous_state.authentication_id,
                authentication_update_internal.clone(),
            ),
        )
        .await;
        let result = match result {
            Err(error)
                if matches!(
                    error.current_context(),
                    diesel_models::errors::DatabaseError::NotFound
                ) =>
            {
                Box::pin(
                    diesel_authentication::update_by_merchant_id_authentication_id(
                        &conn,
                        &processor_merchant_id,
                        &previous_state.authentication_id,
                        authentication_update_internal,
                    ),
                )
                .await
            }
            other => other,
        };
        result
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
            .async_and_then(|authentication| async {
                Authentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> AuthenticationInterface for KVRouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_authentication(
        &self,
        state: &common_utils::types::keymanager::KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: Authentication,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_authentication>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;

        match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_authentication(
                        state,
                        merchant_key_store,
                        authentication,
                        storage_scheme,
                    )
                    .await
            }
            common_enums::MerchantStorageScheme::RedisKv => {
                // Record which layer wrote this row; drives KV-vs-Postgres routing on later updates.
                let authentication = authentication.update_storage_scheme(storage_scheme);

                let merchant_id = authentication
                    .processor_merchant_id
                    .as_ref()
                    .unwrap_or(&authentication.merchant_id);
                let authentication_id = &authentication.authentication_id;
                let payment_id = &authentication.payment_id;

                // Co-locate on the payment's partition; modular auth is its own partition.
                let key = match payment_id {
                    Some(payment_id) => PartitionKey::MerchantIdPaymentId {
                        merchant_id,
                        payment_id,
                    },
                    None => PartitionKey::AuthenticationId { authentication_id },
                };
                let field = diesel_authentication::get_hash_key_for_kv_store(authentication_id);
                let key_str = key.to_string();

                let authentication_to_insert = authentication
                    .clone()
                    .construct_new()
                    .await
                    .change_context(errors::StorageError::EncryptionError)?;

                let mut query_gen_conn = pg_connection_write(self).await?;
                let drainer_query = authentication_to_insert
                    .generate_drainer_insert_query(&mut query_gen_conn)
                    .await
                    .change_context(errors::StorageError::KVError)
                    .attach_printable("Failed to generate authentication insert query")?;

                let diesel_authentication =
                    <Authentication as Conversion>::convert(authentication.clone())
                        .await
                        .change_context(errors::StorageError::EncryptionError)?;

                // Reverse lookup by authentication id (find when under the payment's partition).
                let authentication_id_lookup = ReverseLookupNew {
                    lookup_id: diesel_authentication::get_authentication_id_lookup_id(
                        merchant_id,
                        authentication_id,
                    ),
                    pk_id: key_str.clone(),
                    sk_id: field.clone(),
                    source: "authentication".to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                self.insert_reverse_lookup(authentication_id_lookup, storage_scheme)
                    .await?;

                // Reverse lookup by connector authentication id (webhook path), when present.
                if let Some(connector_authentication_id) =
                    authentication.connector_authentication_id.as_ref()
                {
                    add_connector_authentication_id_to_reverse_lookup(
                        self,
                        &key_str,
                        &field,
                        merchant_id,
                        connector_authentication_id,
                        storage_scheme,
                    )
                    .await?;
                }

                match Box::pin(kv_wrapper::<diesel_authentication, _, _>(
                    self,
                    KvOperation::<diesel_authentication>::HSetNx(
                        &field,
                        &diesel_authentication,
                        drainer_query,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: "authentication",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(authentication),
                    Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_processor_merchant_id_authentication_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            // Stagger release: try processor_merchant_id, fall back to merchant_id for legacy NULL rows.
            let result = diesel_authentication::find_by_processor_merchant_id_authentication_id(
                &conn,
                processor_merchant_id,
                authentication_id,
            )
            .await;
            let result = match result {
                Err(error)
                    if matches!(
                        error.current_context(),
                        diesel_models::errors::DatabaseError::NotFound
                    ) =>
                {
                    diesel_authentication::find_by_merchant_id_authentication_id(
                        &conn,
                        processor_merchant_id,
                        authentication_id,
                    )
                    .await
                }
                other => other,
            };
            result.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };

        let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_authentication>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;

        let diesel_authentication = match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
            common_enums::MerchantStorageScheme::RedisKv => {
                // Resolve partition/field via the authentication-id reverse lookup.
                let lookup_id = diesel_authentication::get_authentication_id_lookup_id(
                    processor_merchant_id,
                    authentication_id,
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    {
                        let diesel_authentication = database_call().await?;
                        Authentication::convert_back(
                            state,
                            diesel_authentication,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                    }
                );
                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<diesel_authentication, _, _>(
                            self,
                            KvOperation::<diesel_authentication>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }?;

        Authentication::convert_back(
            state,
            diesel_authentication,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_authentication_by_processor_merchant_id_connector_authentication_id(
        &self,
        processor_merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            // Stagger release: try processor_merchant_id, fall back to merchant_id for legacy NULL rows.
            let result =
                diesel_authentication::find_authentication_by_processor_merchant_id_connector_authentication_id(
                    &conn,
                    &processor_merchant_id,
                    &connector_authentication_id,
                )
                .await;
            let result = match result {
                Err(error)
                    if matches!(
                        error.current_context(),
                        diesel_models::errors::DatabaseError::NotFound
                    ) =>
                {
                    diesel_authentication::find_authentication_by_merchant_id_connector_authentication_id(
                        &conn,
                        &processor_merchant_id,
                        &connector_authentication_id,
                    )
                    .await
                }
                other => other,
            };
            result.map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };

        let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_authentication>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;

        let diesel_authentication = match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
            common_enums::MerchantStorageScheme::RedisKv => {
                // Resolve partition/field via the connector reverse lookup (webhook flow).
                let lookup_id = diesel_authentication::get_connector_authentication_lookup_id(
                    &processor_merchant_id,
                    &connector_authentication_id,
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    {
                        let diesel_authentication = database_call().await?;
                        Authentication::convert_back(
                            state,
                            diesel_authentication,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                    }
                );
                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper::<diesel_authentication, _, _>(
                            self,
                            KvOperation::<diesel_authentication>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    database_call,
                ))
                .await
            }
        }?;

        Authentication::convert_back(
            state,
            diesel_authentication,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_authentication_by_processor_merchant_id_authentication_id(
        &self,
        previous_state: Authentication,
        authentication_update: AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let merchant_id = previous_state
            .processor_merchant_id
            .clone()
            .unwrap_or_else(|| previous_state.merchant_id.clone());
        let authentication_id = previous_state.authentication_id.clone();
        let payment_id = previous_state.payment_id.clone();

        // Same partition key as on insert (payment's, else authentication's own).
        let key = match payment_id {
            Some(ref payment_id) => PartitionKey::MerchantIdPaymentId {
                merchant_id: &merchant_id,
                payment_id,
            },
            None => PartitionKey::AuthenticationId {
                authentication_id: &authentication_id,
            },
        };
        let field = diesel_authentication::get_hash_key_for_kv_store(&authentication_id);

        // The previous write location drives KV-vs-Postgres routing in decide_storage_scheme.
        let updated_by = previous_state.updated_by.clone();
        let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_authentication>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, updated_by.as_deref()),
        ))
        .await;

        match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_authentication_by_processor_merchant_id_authentication_id(
                        previous_state,
                        authentication_update,
                        merchant_key_store,
                        state,
                        storage_scheme,
                    )
                    .await
            }
            common_enums::MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();

                let current_authentication =
                    <Authentication as Conversion>::convert(previous_state.clone())
                        .await
                        .change_context(errors::StorageError::EncryptionError)?;

                // Captured before the changeset to detect a connector-authentication-id change.
                let old_connector_authentication_id =
                    previous_state.connector_authentication_id.clone();

                let authentication_update_internal =
                    diesel_models::authentication::AuthenticationUpdateInternal::from(
                        diesel_models::authentication::AuthenticationUpdate::from(
                            authentication_update,
                        ),
                    );

                let updated_authentication = authentication_update_internal
                    .clone()
                    .apply_changeset(current_authentication);

                // Add the connector reverse lookup when the update sets/changes that id.
                match (
                    old_connector_authentication_id,
                    &updated_authentication.connector_authentication_id,
                ) {
                    (None, Some(connector_authentication_id)) => {
                        add_connector_authentication_id_to_reverse_lookup(
                            self,
                            &key_str,
                            &field,
                            &merchant_id,
                            connector_authentication_id,
                            storage_scheme,
                        )
                        .await?;
                    }
                    (Some(old), Some(connector_authentication_id))
                        if &old != connector_authentication_id =>
                    {
                        add_connector_authentication_id_to_reverse_lookup(
                            self,
                            &key_str,
                            &field,
                            &merchant_id,
                            connector_authentication_id,
                            storage_scheme,
                        )
                        .await?;
                    }
                    (_, _) => {}
                }

                let redis_value = updated_authentication
                    .encode_to_string_of_json()
                    .change_context(errors::StorageError::SerializationFailed)?;

                let mut query_gen_conn = pg_connection_write(self).await?;
                let drainer_query = authentication_update_internal
                    .generate_drainer_update_query(
                        &mut query_gen_conn,
                        merchant_id.clone(),
                        authentication_id.clone(),
                    )
                    .await
                    .change_context(errors::StorageError::KVError)
                    .attach_printable("Failed to generate authentication update query")?;

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::<diesel_authentication>::Hset(
                        (&field, redis_value),
                        drainer_query,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hset()
                .change_context(errors::StorageError::KVError)?;

                Authentication::convert_back(
                    state,
                    updated_authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(errors::StorageError::DecryptionError)
            }
        }
    }
}

#[async_trait::async_trait]
impl AuthenticationInterface for MockDb {
    type Error = errors::StorageError;

    async fn insert_authentication(
        &self,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        authentication: Authentication,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let mut authentications = self.authentications.lock().await;
        if authentications.iter().any(|authentication_inner| {
            authentication_inner.authentication_id == authentication.authentication_id
        }) {
            Err(errors::StorageError::DuplicateValue {
                entity: "authentication_id",
                key: Some(
                    authentication
                        .authentication_id
                        .get_string_repr()
                        .to_string(),
                ),
            })?
        }
        authentications.push(authentication.clone());
        Ok(authentication)
    }

    async fn find_authentication_by_processor_merchant_id_authentication_id(
        &self,
        processor_merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        _merchant_key_store: &MerchantKeyStore,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let authentications = self.authentications.lock().await;
        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == *processor_merchant_id && auth.authentication_id == *authentication_id
            })
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Authentication not found for processor_merchant_id: {} and authentication_id: {}",
                    processor_merchant_id.get_string_repr(),
                    authentication_id.get_string_repr()
                ))
                .into(),
            )
    }

    async fn find_authentication_by_processor_merchant_id_connector_authentication_id(
        &self,
        processor_merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        _merchant_key_store: &MerchantKeyStore,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let authentications = self.authentications.lock().await;
        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == processor_merchant_id
                    && auth.connector_authentication_id.as_ref()
                        == Some(&connector_authentication_id)
            })
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Authentication not found for processor_merchant_id: {} and connector_authentication_id: {}",
                    processor_merchant_id.get_string_repr(),
                    connector_authentication_id
                ))
                .into(),
            )
    }

    async fn update_authentication_by_processor_merchant_id_authentication_id(
        &self,
        previous_state: Authentication,
        authentication_update: AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let mut authentications = self.authentications.lock().await;
        let item = authentications
            .iter_mut()
            .find(|auth| {
                auth.merchant_id == previous_state.merchant_id
                    && auth.authentication_id == previous_state.authentication_id
            })
            .ok_or(errors::StorageError::ValueNotFound(format!(
                "Authentication not found for merchant_id: {} and authentication_id: {}",
                previous_state.merchant_id.get_string_repr(),
                previous_state.authentication_id.get_string_repr()
            )))?;

        let current_authentication =
            <Authentication as Conversion>::convert(previous_state.clone())
                .await
                .change_context(errors::StorageError::EncryptionError)?;

        let updated_authentication =
            diesel_models::authentication::AuthenticationUpdateInternal::from(
                diesel_models::authentication::AuthenticationUpdate::from(authentication_update),
            )
            .apply_changeset(current_authentication);

        *item = Authentication::convert_back(
            state,
            updated_authentication,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)?;

        Ok(item.clone())
    }
}
