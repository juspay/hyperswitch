use common_utils::{
    errors::CustomResult,
    ext_traits::{AsyncExt, Encode},
    fallback_reverse_lookup_not_found,
};
use diesel_models::{
    authentication::Authentication as DieselAuthentication, reverse_lookup::ReverseLookupNew,
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

impl crate::redis::kv_store::KvStorePartition for DieselAuthentication {}

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
        let inserted_authentication = authentication
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
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
    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselAuthentication::find_by_merchant_id_authentication_id(
            &conn,
            merchant_id,
            authentication_id,
        )
        .await
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
    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        // Only keys the KV reverse lookup; the Postgres lookup stays merchant-id keyed.
        _merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselAuthentication::find_authentication_by_merchant_id_connector_authentication_id(
            &conn,
            &merchant_id,
            &connector_authentication_id,
        )
        .await
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
    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: Authentication,
        authentication_update: AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        DieselAuthentication::update_by_merchant_id_authentication_id(
            &conn,
            &previous_state.merchant_id,
            &previous_state.authentication_id,
            diesel_models::authentication::AuthenticationUpdateInternal::from(
                diesel_models::authentication::AuthenticationUpdate::from(authentication_update),
            ),
        )
        .await
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
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
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
                // Borrow: `authentication` is read-only here and returned at the end.
                let merchant_id = &authentication.merchant_id;
                let authentication_id = &authentication.authentication_id;
                let payment_id = &authentication.payment_id;

                // Co-locate on the payment's partition when present; otherwise
                // (modular auth) the authentication id is its own partition.
                let key = match payment_id {
                    Some(payment_id) => PartitionKey::MerchantIdPaymentId {
                        merchant_id,
                        payment_id,
                    },
                    None => PartitionKey::AuthenticationId { authentication_id },
                };
                let field = authentication_id.get_hash_key_for_kv_store();
                let key_str = key.to_string();

                let authentication_to_insert = authentication
                    .clone()
                    .construct_new()
                    .await
                    .change_context(errors::StorageError::EncryptionError)?;

                let mut query_gen_conn = pg_connection_read(self).await?;
                let drainer_query = authentication_to_insert
                    .generate_drainer_insert_query(&mut query_gen_conn)
                    .await
                    .change_context(errors::StorageError::KVError)
                    .attach_printable("Failed to generate authentication insert query")?;

                let diesel_authentication =
                    <Authentication as Conversion>::convert(authentication.clone())
                        .await
                        .change_context(errors::StorageError::EncryptionError)?;

                // Lets find-by-authentication-id locate the record even when it
                // lives under the payment's partition.
                let authentication_id_lookup = ReverseLookupNew {
                    lookup_id: authentication_id.get_hash_key_for_kv_store(),
                    pk_id: key_str.clone(),
                    sk_id: field.clone(),
                    source: "authentication".to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                self.insert_reverse_lookup(authentication_id_lookup, storage_scheme)
                    .await?;

                // Connector-auth-id lookup, MCA-keyed to avoid cross-connector collisions.
                if let (Some(connector_auth_id), Some(merchant_connector_id)) = (
                    authentication.connector_authentication_id.as_ref(),
                    authentication.merchant_connector_id.as_ref(),
                ) {
                    let reverse_lookup = ReverseLookupNew {
                        lookup_id: merchant_connector_id
                            .get_authentication_connector_lookup_id(connector_auth_id),
                        pk_id: key_str.clone(),
                        sk_id: field.clone(),
                        source: "authentication".to_string(),
                        updated_by: storage_scheme.to_string(),
                    };
                    self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                        .await?;
                }

                match Box::pin(kv_wrapper::<DieselAuthentication, _, _>(
                    self,
                    KvOperation::<DieselAuthentication>::HSetNx(
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
    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselAuthentication::find_by_merchant_id_authentication_id(
                &conn,
                merchant_id,
                authentication_id,
            )
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };

        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;

        let diesel_authentication = match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
            common_enums::MerchantStorageScheme::RedisKv => {
                // Resolve partition/field via the authentication-id reverse lookup,
                // since the record may live under the payment's partition.
                let lookup_id = authentication_id.get_hash_key_for_kv_store();
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
                        Box::pin(kv_wrapper::<DieselAuthentication, _, _>(
                            self,
                            KvOperation::<DieselAuthentication>::HGet(&lookup.sk_id),
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
    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let database_call = || async {
            let conn = pg_connection_read(self).await?;
            DieselAuthentication::find_authentication_by_merchant_id_connector_authentication_id(
                &conn,
                &merchant_id,
                &connector_authentication_id,
            )
            .await
            .map_err(|error| {
                let new_err = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
        };

        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;

        let diesel_authentication = match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => database_call().await,
            common_enums::MerchantStorageScheme::RedisKv => match merchant_connector_id.as_ref() {
                // No MCA id: the reverse lookup is MCA-keyed, so we can't hit Redis and
                // fall back to the DB. A KV-only (not-yet-drained) row won't be found.
                None => {
                    router_env::logger::warn!(
                        connector_authentication_id = %connector_authentication_id,
                        "RedisKv find-by-connector-auth-id without MCA id: reading from \
                         Postgres; a not-yet-drained KV record will not be found"
                    );
                    database_call().await
                }
                Some(merchant_connector_id) => {
                    let lookup_id = merchant_connector_id
                        .get_authentication_connector_lookup_id(&connector_authentication_id);
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
                            Box::pin(kv_wrapper::<DieselAuthentication, _, _>(
                                self,
                                KvOperation::<DieselAuthentication>::HGet(&lookup.sk_id),
                                key,
                            ))
                            .await?
                            .try_into_hget()
                        },
                        database_call,
                    ))
                    .await
                }
            },
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
    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: Authentication,
        authentication_update: AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &common_utils::types::keymanager::KeyManagerState,
        storage_scheme: common_enums::MerchantStorageScheme,
    ) -> error_stack::Result<Authentication, errors::StorageError> {
        let merchant_id = previous_state.merchant_id.clone();
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
        let field = authentication_id.get_hash_key_for_kv_store();

        // No `updated_by` column on authentication, so use the configured scheme as the
        // soft-kill KV-presence hint (any value != "postgres_only" triggers the HGet
        // probe). Bound to a local: the `Option<&str>` is borrowed across the `.await`.
        let updated_by = storage_scheme.to_string();
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, Some(updated_by.as_str())),
        ))
        .await;

        match storage_scheme {
            common_enums::MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_authentication_by_merchant_id_authentication_id(
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

                let current_authentication_as_new = previous_state
                    .clone()
                    .construct_new()
                    .await
                    .change_context(errors::StorageError::EncryptionError)?;

                let authentication_storage_update =
                    diesel_models::authentication::AuthenticationUpdate::from(
                        authentication_update,
                    );
                let authentication_update_internal =
                    diesel_models::authentication::AuthenticationUpdateInternal::from(
                        authentication_storage_update,
                    );

                let updated_authentication = authentication_update_internal
                    .clone()
                    .apply_changeset(current_authentication_as_new);

                // First update to set the connector auth id: create its MCA-keyed
                // reverse lookup so it can later be found by connector auth id.
                if previous_state.connector_authentication_id.is_none() {
                    if let (Some(connector_auth_id), Some(merchant_connector_id)) = (
                        updated_authentication.connector_authentication_id.as_ref(),
                        updated_authentication.merchant_connector_id.as_ref(),
                    ) {
                        let reverse_lookup = ReverseLookupNew {
                            lookup_id: merchant_connector_id
                                .get_authentication_connector_lookup_id(connector_auth_id),
                            pk_id: key_str.clone(),
                            sk_id: field.clone(),
                            source: "authentication".to_string(),
                            updated_by: storage_scheme.to_string(),
                        };
                        self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                            .await?;
                    }
                }

                let redis_value = updated_authentication
                    .encode_to_string_of_json()
                    .change_context(errors::StorageError::SerializationFailed)?;

                let mut query_gen_conn = pg_connection_read(self).await?;
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
                    KvOperation::<DieselAuthentication>::Hset((&field, redis_value), drainer_query),
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

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        _merchant_key_store: &MerchantKeyStore,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let authentications = self.authentications.lock().await;
        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == *merchant_id && auth.authentication_id == *authentication_id
            })
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Authentication not found for merchant_id: {} and authentication_id: {}",
                    merchant_id.get_string_repr(),
                    authentication_id.get_string_repr()
                ))
                .into(),
            )
    }

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        _merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
        connector_authentication_id: String,
        _merchant_key_store: &MerchantKeyStore,
        _state: &common_utils::types::keymanager::KeyManagerState,
        _storage_scheme: common_enums::MerchantStorageScheme,
    ) -> CustomResult<Authentication, errors::StorageError> {
        let authentications = self.authentications.lock().await;
        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == merchant_id
                    && auth.connector_authentication_id.as_ref()
                        == Some(&connector_authentication_id)
            })
            .cloned()
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "Authentication not found for merchant_id: {} and connector_authentication_id: {}",
                    merchant_id.get_string_repr(),
                    connector_authentication_id
                ))
                .into(),
            )
    }

    async fn update_authentication_by_merchant_id_authentication_id(
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

        // Apply the update the same way the real stores do, reusing the diesel
        // changeset instead of re-implementing the variant -> field mapping here.
        let diesel_authentication_new = previous_state
            .clone()
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?;

        let updated_authentication =
            diesel_models::authentication::AuthenticationUpdateInternal::from(
                diesel_models::authentication::AuthenticationUpdate::from(authentication_update),
            )
            .apply_changeset(diesel_authentication_new);

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
