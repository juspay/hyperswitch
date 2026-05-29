use common_utils::{ext_traits::AsyncExt, types::keymanager::KeyManagerState};
use diesel_models::enums::MerchantStorageScheme;
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};
use router_env::{instrument, tracing};
use storage_impl::StorageError;

use super::{MockDb, Store};
use crate::{connection, core::errors::CustomResult, types::storage};

#[async_trait::async_trait]
pub trait AuthenticationInterface {
    async fn insert_authentication(
        &self,
        state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        authentication: hyperswitch_domain_models::authentication::Authentication,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        authentication_id: &common_utils::id_type::AuthenticationId,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn find_authentication_by_merchant_id_connector_authentication_id(
        &self,
        merchant_id: common_utils::id_type::MerchantId,
        connector_authentication_id: String,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: hyperswitch_domain_models::authentication::Authentication,
        authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
        merchant_key_store: &MerchantKeyStore,
        state: &KeyManagerState,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError>;
}

#[cfg(feature = "kv_store")]
mod kv_impl {
    use common_utils::{
        ext_traits::{AsyncExt, Encode},
        fallback_reverse_lookup_not_found,
    };
    use diesel_models::{authentication::Authentication as DieselAuthentication, kv};
    use error_stack::{report, ResultExt};
    use hyperswitch_domain_models::{
        authentication::Authentication as DomainAuthentication, behaviour::Conversion,
    };
    use redis_interface::HsetnxReply;
    use router_env::{instrument, tracing};
    use storage_impl::redis::kv_store::{
        decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey,
    };

    use super::AuthenticationInterface;
    use crate::{
        connection,
        core::errors::{self, utils::RedisErrorExt, CustomResult},
        db::reverse_lookup::ReverseLookupInterface,
        services::Store,
        types::storage,
        utils::db_utils,
    };

    type StorageError = storage_impl::StorageError;

    #[async_trait::async_trait]
    impl AuthenticationInterface for Store {
        #[instrument(skip_all)]
        async fn insert_authentication(
            &self,
            state: &common_utils::types::keymanager::KeyManagerState,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            authentication: DomainAuthentication,
            storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
                self,
                storage_scheme,
                Op::Insert,
            ))
            .await;

            match storage_scheme {
                diesel_models::enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    let inserted_authentication = authentication
                        .construct_new()
                        .await
                        .change_context(StorageError::EncryptionError)?
                        .insert(&conn)
                        .await
                        .map_err(|error| report!(StorageError::from(error)))?;
                    DomainAuthentication::convert_back(
                        state,
                        inserted_authentication,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
                }
                diesel_models::enums::MerchantStorageScheme::RedisKv => {
                    let merchant_id = authentication.merchant_id.clone();
                    let authentication_id = authentication.authentication_id.clone();
                    let connector_authentication_id =
                        authentication.connector_authentication_id.clone();

                    let key = PartitionKey::MerchantIdAuthenticationId {
                        merchant_id: &merchant_id,
                        authentication_id: &authentication_id,
                    };
                    let field = authentication_id.get_hash_key_for_kv_store();
                    let key_str = key.to_string();

                    let authentication_to_insert = authentication
                        .clone()
                        .construct_new()
                        .await
                        .change_context(StorageError::EncryptionError)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Insert {
                            insertable: Box::new(kv::Insertable::Authentication(Box::new(
                                authentication_to_insert,
                            ))),
                        },
                    };

                    let diesel_authentication =
                        <DomainAuthentication as Conversion>::convert(authentication.clone())
                            .await
                            .change_context(StorageError::EncryptionError)?;

                    if let Some(ref connector_auth_id) = connector_authentication_id {
                        let lookup_id = format!(
                            "auth_connector_{}_{}",
                            merchant_id.get_string_repr(),
                            connector_auth_id
                        );
                        let reverse_lookup = diesel_models::reverse_lookup::ReverseLookupNew {
                            lookup_id,
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
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hsetnx()
                    {
                        Ok(HsetnxReply::KeyNotSet) => Err(StorageError::DuplicateValue {
                            entity: "authentication",
                            key: Some(key_str),
                        }
                        .into()),
                        Ok(HsetnxReply::KeySet) => Ok(authentication),
                        Err(error) => Err(error.change_context(StorageError::KVError)),
                    }
                }
            }
        }

        #[instrument(skip_all)]
        async fn find_authentication_by_merchant_id_authentication_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            authentication_id: &common_utils::id_type::AuthenticationId,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage::Authentication::find_by_merchant_id_authentication_id(
                    &conn,
                    merchant_id,
                    authentication_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;

            let diesel_authentication = match storage_scheme {
                diesel_models::enums::MerchantStorageScheme::PostgresOnly => {
                    database_call().await
                }
                diesel_models::enums::MerchantStorageScheme::RedisKv => {
                    let key = PartitionKey::MerchantIdAuthenticationId {
                        merchant_id,
                        authentication_id,
                    };
                    let field = authentication_id.get_hash_key_for_kv_store();
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
                        async {
                            Box::pin(kv_wrapper::<DieselAuthentication, _, _>(
                                self,
                                KvOperation::<DieselAuthentication>::HGet(&field),
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

            DomainAuthentication::convert_back(
                state,
                diesel_authentication,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn find_authentication_by_merchant_id_connector_authentication_id(
            &self,
            merchant_id: common_utils::id_type::MerchantId,
            connector_authentication_id: String,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            let database_call = || async {
                storage::Authentication::find_authentication_by_merchant_id_connector_authentication_id(
                    &conn,
                    &merchant_id,
                    &connector_authentication_id,
                )
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
            };
            let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
                self,
                storage_scheme,
                Op::Find,
            ))
            .await;

            let diesel_authentication = match storage_scheme {
                diesel_models::enums::MerchantStorageScheme::PostgresOnly => {
                    database_call().await
                }
                diesel_models::enums::MerchantStorageScheme::RedisKv => {
                    let lookup_id = format!(
                        "auth_connector_{}_{}",
                        merchant_id.get_string_repr(),
                        connector_authentication_id
                    );
                    let lookup = fallback_reverse_lookup_not_found!(
                        self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                            .await,
                        {
                            let diesel_authentication = database_call().await?;
                            DomainAuthentication::convert_back(
                                state,
                                diesel_authentication,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(StorageError::DecryptionError)
                        }
                    );
                    let key = PartitionKey::CombinationKey {
                        combination: &lookup.pk_id,
                    };
                    Box::pin(db_utils::try_redis_get_else_try_database_get(
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

            DomainAuthentication::convert_back(
                state,
                diesel_authentication,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn update_authentication_by_merchant_id_authentication_id(
            &self,
            previous_state: DomainAuthentication,
            authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let merchant_id = previous_state.merchant_id.clone();
            let authentication_id = previous_state.authentication_id.clone();

            let key = PartitionKey::MerchantIdAuthenticationId {
                merchant_id: &merchant_id,
                authentication_id: &authentication_id,
            };
            let field = authentication_id.get_hash_key_for_kv_store();

            let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselAuthentication>(
                self,
                storage_scheme,
                Op::Update(key.clone(), &field, None),
            ))
            .await;

            match storage_scheme {
                diesel_models::enums::MerchantStorageScheme::PostgresOnly => {
                    let conn = connection::pg_connection_write(self).await?;
                    storage::Authentication::update_by_merchant_id_authentication_id(
                        &conn,
                        previous_state.merchant_id,
                        previous_state.authentication_id,
                        authentication_update.into(),
                    )
                    .await
                    .map_err(|error| report!(errors::StorageError::from(error)))
                    .async_and_then(|authentication| async {
                        DomainAuthentication::convert_back(
                            state,
                            authentication,
                            merchant_key_store.key.get_inner(),
                            merchant_key_store.merchant_id.clone().into(),
                        )
                        .await
                        .change_context(StorageError::DecryptionError)
                    })
                    .await
                }
                diesel_models::enums::MerchantStorageScheme::RedisKv => {
                    let key_str = key.to_string();

                    let current_authentication =
                        <DomainAuthentication as Conversion>::convert(previous_state.clone())
                            .await
                            .change_context(StorageError::EncryptionError)?;

                    let current_authentication_as_new = previous_state
                        .clone()
                        .construct_new()
                        .await
                        .change_context(StorageError::EncryptionError)?;

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

                    let redis_value = updated_authentication
                        .encode_to_string_of_json()
                        .change_context(StorageError::SerializationFailed)?;

                    let redis_entry = kv::TypedSql {
                        op: kv::DBOperation::Update {
                            updatable: Box::new(kv::Updateable::AuthenticationUpdate(Box::new(
                                kv::AuthenticationUpdateMems {
                                    orig: current_authentication,
                                    update_data: authentication_update_internal,
                                },
                            ))),
                        },
                    };

                    Box::pin(kv_wrapper::<(), _, _>(
                        self,
                        KvOperation::<DieselAuthentication>::Hset(
                            (&field, redis_value),
                            redis_entry,
                        ),
                        key,
                    ))
                    .await
                    .map_err(|err| err.to_redis_failed_response(&key_str))?
                    .try_into_hset()
                    .change_context(StorageError::KVError)?;

                    DomainAuthentication::convert_back(
                        state,
                        updated_authentication,
                        merchant_key_store.key.get_inner(),
                        merchant_key_store.merchant_id.clone().into(),
                    )
                    .await
                    .change_context(StorageError::DecryptionError)
                }
            }
        }
    }
}

#[cfg(not(feature = "kv_store"))]
mod storage_impl_no_kv {
    use common_utils::ext_traits::AsyncExt;
    use error_stack::{report, ResultExt};
    use hyperswitch_domain_models::{
        authentication::Authentication as DomainAuthentication, behaviour::Conversion,
    };
    use router_env::{instrument, tracing};

    use super::AuthenticationInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage,
    };

    type StorageError = storage_impl::StorageError;

    #[async_trait::async_trait]
    impl AuthenticationInterface for Store {
        #[instrument(skip_all)]
        async fn insert_authentication(
            &self,
            state: &common_utils::types::keymanager::KeyManagerState,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            authentication: DomainAuthentication,
            _storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            let inserted_authentication = authentication
                .construct_new()
                .await
                .change_context(StorageError::EncryptionError)?
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;
            DomainAuthentication::convert_back(
                state,
                inserted_authentication,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
        }

        #[instrument(skip_all)]
        async fn find_authentication_by_merchant_id_authentication_id(
            &self,
            merchant_id: &common_utils::id_type::MerchantId,
            authentication_id: &common_utils::id_type::AuthenticationId,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            _storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage::Authentication::find_by_merchant_id_authentication_id(
                &conn,
                merchant_id,
                authentication_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|authentication| async {
                DomainAuthentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
            })
            .await
        }

        #[instrument(skip_all)]
        async fn find_authentication_by_merchant_id_connector_authentication_id(
            &self,
            merchant_id: common_utils::id_type::MerchantId,
            connector_authentication_id: String,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            _storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            storage::Authentication::find_authentication_by_merchant_id_connector_authentication_id(
                &conn,
                &merchant_id,
                &connector_authentication_id,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|authentication| async {
                DomainAuthentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
            })
            .await
        }

        #[instrument(skip_all)]
        async fn update_authentication_by_merchant_id_authentication_id(
            &self,
            previous_state: DomainAuthentication,
            authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
            merchant_key_store: &hyperswitch_domain_models::merchant_key_store::MerchantKeyStore,
            state: &common_utils::types::keymanager::KeyManagerState,
            _storage_scheme: diesel_models::enums::MerchantStorageScheme,
        ) -> CustomResult<DomainAuthentication, StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            storage::Authentication::update_by_merchant_id_authentication_id(
                &conn,
                previous_state.merchant_id,
                previous_state.authentication_id,
                authentication_update.into(),
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|authentication| async {
                DomainAuthentication::convert_back(
                    state,
                    authentication,
                    merchant_key_store.key.get_inner(),
                    merchant_key_store.merchant_id.clone().into(),
                )
                .await
                .change_context(StorageError::DecryptionError)
            })
            .await
        }
    }
}

#[async_trait::async_trait]
impl AuthenticationInterface for MockDb {
    async fn insert_authentication(
        &self,
        _state: &KeyManagerState,
        _merchant_key_store: &MerchantKeyStore,
        authentication: hyperswitch_domain_models::authentication::Authentication,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let mut authentications = self.authentications.lock().await;
        if authentications.iter().any(|authentication_inner| {
            authentication_inner.authentication_id == authentication.authentication_id
        }) {
            Err(StorageError::DuplicateValue {
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
        _state: &KeyManagerState,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let authentications = self.authentications.lock().await;

        authentications
            .iter()
            .find(|auth| {
                auth.merchant_id == *merchant_id && auth.authentication_id == *authentication_id
            })
            .cloned()
            .ok_or(
                StorageError::ValueNotFound(format!(
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
        connector_authentication_id: String,
        _merchant_key_store: &MerchantKeyStore,
        _state: &KeyManagerState,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
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
                StorageError::ValueNotFound(format!(
                "Authentication not found for merchant_id: {} and connector_authentication_id: {}",
                merchant_id.get_string_repr(),
                connector_authentication_id
            ))
                .into(),
            )
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: hyperswitch_domain_models::authentication::Authentication,
        authentication_update: hyperswitch_domain_models::authentication::AuthenticationUpdate,
        _merchant_key_store: &MerchantKeyStore,
        _state: &KeyManagerState,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<hyperswitch_domain_models::authentication::Authentication, StorageError> {
        let mut authentications = self.authentications.lock().await;

        let auth_to_update = authentications
            .iter_mut()
            .find(|auth| {
                auth.merchant_id == previous_state.merchant_id
                    && auth.authentication_id == previous_state.authentication_id
            })
            .ok_or(StorageError::ValueNotFound(format!(
                "Authentication not found for merchant_id: {} and authentication_id: {}",
                previous_state.merchant_id.get_string_repr(),
                previous_state.authentication_id.get_string_repr()
            )))?;

        match authentication_update {
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationVersionCallUpdate {
                maximum_supported_3ds_version,
                message_version,
            } => {
                auth_to_update.maximum_supported_version = Some(maximum_supported_3ds_version);
                auth_to_update.message_version = Some(message_version);
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationThreeDsMethodCall {
                threeds_server_transaction_id,
                three_ds_method_data,
                three_ds_method_url,
                acquirer_bin,
                acquirer_merchant_id,
                connector_metadata,
            } => {
                auth_to_update.threeds_server_transaction_id = Some(threeds_server_transaction_id);
                auth_to_update.three_ds_method_data = three_ds_method_data;
                auth_to_update.three_ds_method_url = three_ds_method_url;
                auth_to_update.acquirer_bin = acquirer_bin;
                auth_to_update.acquirer_merchant_id = acquirer_merchant_id;
                auth_to_update.connector_metadata = connector_metadata;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PreAuthenticationUpdate {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
                authentication_status,
                acquirer_bin,
                acquirer_merchant_id,
                directory_server_id,
                acquirer_country_code,
                billing_address,
                shipping_address,
                browser_info,
                email,
                scheme_id,
                merchant_category_code,
                merchant_country_code,
                billing_country,
                shipping_country,
                earliest_supported_version,
                latest_supported_version,
            } => {
                auth_to_update.threeds_server_transaction_id = Some(threeds_server_transaction_id);
                auth_to_update.maximum_supported_version = Some(maximum_supported_3ds_version);
                auth_to_update.connector_authentication_id = Some(connector_authentication_id);
                auth_to_update.three_ds_method_data = three_ds_method_data;
                auth_to_update.three_ds_method_url = three_ds_method_url;
                auth_to_update.message_version = Some(message_version);
                auth_to_update.connector_metadata = connector_metadata;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.acquirer_bin = acquirer_bin;
                auth_to_update.acquirer_merchant_id = acquirer_merchant_id;
                auth_to_update.directory_server_id = directory_server_id;
                auth_to_update.acquirer_country_code = acquirer_country_code;
                auth_to_update.billing_address = *billing_address;
                auth_to_update.shipping_address = *shipping_address;
                auth_to_update.browser_info = *browser_info;
                auth_to_update.email = email;
                auth_to_update.scheme_name = scheme_id;
                auth_to_update.mcc = merchant_category_code;
                auth_to_update.merchant_country_code = merchant_country_code;
                auth_to_update.billing_country = billing_country;
                auth_to_update.shipping_country = shipping_country;
                auth_to_update.earliest_supported_version = earliest_supported_version;
                auth_to_update.latest_supported_version = latest_supported_version;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::AuthenticationUpdate {
                trans_status,
                authentication_type,
                acs_url,
                challenge_request,
                acs_reference_number,
                acs_trans_id,
                acs_signed_content,
                connector_metadata,
                authentication_status,
                ds_trans_id,
                eci,
                challenge_code,
                challenge_cancel,
                challenge_code_reason,
                message_extension,
                challenge_request_key,
                device_type,
                device_brand,
                device_os,
                device_display,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.authentication_type = Some(authentication_type);
                auth_to_update.acs_url = acs_url;
                auth_to_update.challenge_request = challenge_request;
                auth_to_update.acs_reference_number = acs_reference_number;
                auth_to_update.acs_trans_id = acs_trans_id;
                auth_to_update.acs_signed_content = acs_signed_content;
                auth_to_update.connector_metadata = connector_metadata;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.ds_trans_id = ds_trans_id;
                auth_to_update.eci = eci;
                auth_to_update.challenge_code = challenge_code;
                auth_to_update.challenge_cancel = challenge_cancel;
                auth_to_update.challenge_code_reason = challenge_code_reason;
                auth_to_update.message_extension = message_extension;
                auth_to_update.challenge_request_key = challenge_request_key;
                auth_to_update.device_type = device_type;
                auth_to_update.device_brand = device_brand;
                auth_to_update.device_os = device_os;
                auth_to_update.device_display = device_display;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthenticationUpdate {
                trans_status,
                eci,
                authentication_status,
                challenge_cancel,
                challenge_code_reason,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.eci = eci;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.challenge_cancel = challenge_cancel;
                auth_to_update.challenge_code_reason = challenge_code_reason;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => {
                auth_to_update.error_message = error_message;
                auth_to_update.error_code = error_code;
                auth_to_update.authentication_status = authentication_status;
                auth_to_update.connector_authentication_id = connector_authentication_id;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => {
                auth_to_update.authentication_lifecycle_status = authentication_lifecycle_status;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::AuthenticationStatusUpdate {
                trans_status,
                authentication_status,
            } => {
                auth_to_update.trans_status = Some(trans_status);
                auth_to_update.authentication_status = authentication_status;
            }
            hyperswitch_domain_models::authentication::AuthenticationUpdate::AcquirerDetailsUpdate {
                acquirer_bin,
                acquirer_merchant_id,
                acquirer_country_code,
            } => {
                auth_to_update.acquirer_bin = acquirer_bin;
                auth_to_update.acquirer_merchant_id = acquirer_merchant_id;
                auth_to_update.acquirer_country_code = acquirer_country_code;
            }
        }

        auth_to_update.modified_at = common_utils::date_time::now();

        Ok(auth_to_update.clone())
    }
}
