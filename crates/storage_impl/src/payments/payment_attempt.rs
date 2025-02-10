#[cfg(feature = "v2")]
use common_utils::types::keymanager::KeyManagerState;
use common_utils::{
    errors::CustomResult,
    fallback_reverse_lookup_not_found,
    types::{ConnectorTransactionId, ConnectorTransactionIdTrait},
};
use diesel_models::{
    enums::{
        MandateAmountData as DieselMandateAmountData, MandateDataType as DieselMandateType,
        MandateDetails as DieselMandateDetails, MerchantStorageScheme,
    },
    kv,
    payment_attempt::{
        PaymentAttempt as DieselPaymentAttempt, PaymentAttemptNew as DieselPaymentAttemptNew,
    },
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptNew;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_key_store::MerchantKeyStore,
};
use hyperswitch_domain_models::{
    errors,
    mandates::{MandateAmountData, MandateDataType, MandateDetails},
    payments::payment_attempt::{PaymentAttempt, PaymentAttemptInterface, PaymentAttemptUpdate},
};
#[cfg(feature = "olap")]
use hyperswitch_domain_models::{
    payments::payment_attempt::PaymentListFilters, payments::PaymentIntent,
};
use redis_interface::HsetnxReply;
use router_env::{instrument, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    lookup::ReverseLookupInterface,
    redis::kv_store::{decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey},
    utils::{pg_connection_read, pg_connection_write, try_redis_get_else_try_database_get},
    DataModelExt, DatabaseStore, KVRouterStore, RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for RouterStore<T> {
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        payment_attempt
            .to_storage_model()
            .insert(&conn)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn insert_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        payment_attempt: PaymentAttempt,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        payment_attempt
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| {
                let new_error = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_error)
            })?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;
        this.to_storage_model()
            .update_with_attempt_id(&conn, payment_attempt.to_storage_model())
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_write(self).await?;

        Conversion::convert(this)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update_with_attempt_id(
                &conn,
                diesel_models::PaymentAttemptUpdateInternal::from(payment_attempt),
            )
            .await
            .map_err(|error| {
                let new_error = diesel_error_to_data_error(*error.current_context());
                error.change_context(new_error)
            })?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &ConnectorTransactionId,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_connector_transaction_id_payment_id_merchant_id(
            &conn,
            connector_transaction_id,
            payment_id,
            merchant_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_last_successful_attempt_by_payment_id_merchant_id(
            &conn,
            payment_id,
            merchant_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
            &conn,
            payment_id,
            merchant_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_txn_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_merchant_id_connector_txn_id(
            &conn,
            merchant_id,
            connector_txn_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_profile_id_connector_transaction_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_txn_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_profile_id_connector_transaction_id(
            &conn,
            profile_id,
            connector_txn_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })?
        .convert(
            key_manager_state,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        attempt_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_payment_id_merchant_id_attempt_id(
            &conn,
            payment_id,
            merchant_id,
            attempt_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentListFilters, errors::StorageError> {
        use hyperswitch_domain_models::behaviour::Conversion;

        let conn = pg_connection_read(self).await?;
        let intents = futures::future::try_join_all(pi.iter().cloned().map(|pi| async {
            Conversion::convert(pi)
                .await
                .change_context(errors::StorageError::EncryptionError)
        }))
        .await?;

        DieselPaymentAttempt::get_filters_for_payments(&conn, intents.as_slice(), merchant_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(
                |(
                    connector,
                    currency,
                    status,
                    payment_method,
                    payment_method_type,
                    authentication_type,
                )| PaymentListFilters {
                    connector,
                    currency,
                    status,
                    payment_method,
                    payment_method_type,
                    authentication_type,
                },
            )
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_merchant_id_preprocessing_id(
            &conn,
            merchant_id,
            preprocessing_id,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentAttempt>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(|a| {
                a.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_merchant_id_attempt_id(&conn, merchant_id, attempt_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        attempt_id: &common_utils::id_type::GlobalAttemptId,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_id(&conn, attempt_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_attempts_by_payment_intent_id(
        &self,
        key_manager_state: &KeyManagerState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        merchant_key_store: &MerchantKeyStore,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError> {
        use common_utils::ext_traits::AsyncExt;

        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_payment_id(&conn, payment_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(*er.current_context());
                er.change_context(new_err)
            })
            .async_and_then(|payment_attempts| async {
                let mut domain_payment_attempts = Vec::with_capacity(payment_attempts.len());
                for attempt in payment_attempts.into_iter() {
                    domain_payment_attempts.push(
                        attempt
                            .convert(
                                key_manager_state,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    );
                }
                Ok(domain_payment_attempts)
            })
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_attempt_ids: &[String],
        connector: Option<Vec<api_models::enums::Connector>>,
        payment_method: Option<Vec<common_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<common_enums::AuthenticationType>>,
        merchant_connector_id: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
        card_network: Option<Vec<common_enums::CardNetwork>>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = self
            .db_store
            .get_replica_pool()
            .get()
            .await
            .change_context(errors::StorageError::DatabaseConnectionError)?;
        let connector_strings = connector.as_ref().map(|connector| {
            connector
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
        });
        DieselPaymentAttempt::get_total_count_of_attempts(
            &conn,
            merchant_id,
            active_attempt_ids,
            connector_strings,
            payment_method,
            payment_method_type,
            authentication_type,
            merchant_connector_id,
            card_network,
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(*er.current_context());
            er.change_context(new_err)
        })
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for KVRouterStore<T> {
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Insert,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_attempt(payment_attempt, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let merchant_id = payment_attempt.merchant_id.clone();
                let payment_id = payment_attempt.payment_id.clone();
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id: &merchant_id,
                    payment_id: &payment_id,
                };
                let key_str = key.to_string();
                let created_attempt = PaymentAttempt {
                    payment_id: payment_attempt.payment_id.clone(),
                    merchant_id: payment_attempt.merchant_id.clone(),
                    attempt_id: payment_attempt.attempt_id.clone(),
                    status: payment_attempt.status,
                    net_amount: payment_attempt.net_amount.clone(),
                    currency: payment_attempt.currency,
                    save_to_locker: payment_attempt.save_to_locker,
                    connector: payment_attempt.connector.clone(),
                    error_message: payment_attempt.error_message.clone(),
                    offer_amount: payment_attempt.offer_amount,
                    payment_method_id: payment_attempt.payment_method_id.clone(),
                    payment_method: payment_attempt.payment_method,
                    connector_transaction_id: None,
                    capture_method: payment_attempt.capture_method,
                    capture_on: payment_attempt.capture_on,
                    confirm: payment_attempt.confirm,
                    authentication_type: payment_attempt.authentication_type,
                    created_at: payment_attempt
                        .created_at
                        .unwrap_or_else(common_utils::date_time::now),
                    modified_at: payment_attempt
                        .created_at
                        .unwrap_or_else(common_utils::date_time::now),
                    last_synced: payment_attempt.last_synced,
                    amount_to_capture: payment_attempt.amount_to_capture,
                    cancellation_reason: payment_attempt.cancellation_reason.clone(),
                    mandate_id: payment_attempt.mandate_id.clone(),
                    browser_info: payment_attempt.browser_info.clone(),
                    payment_token: payment_attempt.payment_token.clone(),
                    error_code: payment_attempt.error_code.clone(),
                    connector_metadata: payment_attempt.connector_metadata.clone(),
                    payment_experience: payment_attempt.payment_experience,
                    payment_method_type: payment_attempt.payment_method_type,
                    payment_method_data: payment_attempt.payment_method_data.clone(),
                    business_sub_label: payment_attempt.business_sub_label.clone(),
                    straight_through_algorithm: payment_attempt.straight_through_algorithm.clone(),
                    mandate_details: payment_attempt.mandate_details.clone(),
                    preprocessing_step_id: payment_attempt.preprocessing_step_id.clone(),
                    error_reason: payment_attempt.error_reason.clone(),
                    multiple_capture_count: payment_attempt.multiple_capture_count,
                    connector_response_reference_id: None,
                    charge_id: None,
                    amount_capturable: payment_attempt.amount_capturable,
                    updated_by: storage_scheme.to_string(),
                    authentication_data: payment_attempt.authentication_data.clone(),
                    encoded_data: payment_attempt.encoded_data.clone(),
                    merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                    unified_code: payment_attempt.unified_code.clone(),
                    unified_message: payment_attempt.unified_message.clone(),
                    external_three_ds_authentication_attempted: payment_attempt
                        .external_three_ds_authentication_attempted,
                    authentication_connector: payment_attempt.authentication_connector.clone(),
                    authentication_id: payment_attempt.authentication_id.clone(),
                    mandate_data: payment_attempt.mandate_data.clone(),
                    payment_method_billing_address_id: payment_attempt
                        .payment_method_billing_address_id
                        .clone(),
                    fingerprint_id: payment_attempt.fingerprint_id.clone(),
                    client_source: payment_attempt.client_source.clone(),
                    client_version: payment_attempt.client_version.clone(),
                    customer_acceptance: payment_attempt.customer_acceptance.clone(),
                    organization_id: payment_attempt.organization_id.clone(),
                    profile_id: payment_attempt.profile_id.clone(),
                    connector_mandate_detail: payment_attempt.connector_mandate_detail.clone(),
                    card_discovery: payment_attempt.card_discovery,
                    charges: None,
                };

                let field = format!("pa_{}", created_attempt.attempt_id);

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: Box::new(kv::Insertable::PaymentAttempt(Box::new(
                            payment_attempt.to_storage_model(),
                        ))),
                    },
                };

                //Reverse lookup for attempt_id
                let reverse_lookup = ReverseLookupNew {
                    lookup_id: format!(
                        "pa_{}_{}",
                        created_attempt.merchant_id.get_string_repr(),
                        &created_attempt.attempt_id,
                    ),
                    pk_id: key_str.clone(),
                    sk_id: field.clone(),
                    source: "payment_attempt".to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                    .await?;

                match Box::pin(kv_wrapper::<PaymentAttempt, _, _>(
                    self,
                    KvOperation::HSetNx(
                        &field,
                        &created_attempt.clone().to_storage_model(),
                        redis_entry,
                    ),
                    key,
                ))
                .await
                .map_err(|err| err.to_redis_failed_response(&key_str))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: "payment attempt",
                        key: Some(key_str),
                    }
                    .into()),
                    Ok(HsetnxReply::KeySet) => Ok(created_attempt),
                    Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                }
            }
        }
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn insert_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        payment_attempt: PaymentAttempt,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        // Ignoring storage scheme for v2 implementation
        self.router_store
            .insert_payment_attempt(
                key_manager_state,
                merchant_key_store,
                payment_attempt,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let key = PartitionKey::MerchantIdPaymentId {
            merchant_id: &this.merchant_id,
            payment_id: &this.payment_id,
        };
        let field = format!("pa_{}", this.attempt_id);
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Update(key.clone(), &field, Some(&this.updated_by)),
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_attempt_with_attempt_id(this, payment_attempt, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key_str = key.to_string();
                let old_connector_transaction_id = &this.get_connector_payment_id();
                let old_preprocessing_id = &this.preprocessing_step_id;
                let updated_attempt = PaymentAttempt::from_storage_model(
                    payment_attempt
                        .clone()
                        .to_storage_model()
                        .apply_changeset(this.clone().to_storage_model()),
                );
                // Check for database presence as well Maybe use a read replica here ?
                let redis_value = serde_json::to_string(&updated_attempt)
                    .change_context(errors::StorageError::KVError)?;

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: Box::new(kv::Updateable::PaymentAttemptUpdate(Box::new(
                            kv::PaymentAttemptUpdateMems {
                                orig: this.clone().to_storage_model(),
                                update_data: payment_attempt.to_storage_model(),
                            },
                        ))),
                    },
                };

                match (
                    old_connector_transaction_id,
                    &updated_attempt.get_connector_payment_id(),
                ) {
                    (None, Some(connector_transaction_id)) => {
                        add_connector_txn_id_to_reverse_lookup(
                            self,
                            key_str.as_str(),
                            &this.merchant_id,
                            updated_attempt.attempt_id.as_str(),
                            connector_transaction_id,
                            storage_scheme,
                        )
                        .await?;
                    }
                    (Some(old_connector_transaction_id), Some(connector_transaction_id)) => {
                        if old_connector_transaction_id.ne(connector_transaction_id) {
                            add_connector_txn_id_to_reverse_lookup(
                                self,
                                key_str.as_str(),
                                &this.merchant_id,
                                updated_attempt.attempt_id.as_str(),
                                connector_transaction_id,
                                storage_scheme,
                            )
                            .await?;
                        }
                    }
                    (_, _) => {}
                }

                match (old_preprocessing_id, &updated_attempt.preprocessing_step_id) {
                    (None, Some(preprocessing_id)) => {
                        add_preprocessing_id_to_reverse_lookup(
                            self,
                            key_str.as_str(),
                            &this.merchant_id,
                            updated_attempt.attempt_id.as_str(),
                            preprocessing_id.as_str(),
                            storage_scheme,
                        )
                        .await?;
                    }
                    (Some(old_preprocessing_id), Some(preprocessing_id)) => {
                        if old_preprocessing_id.ne(preprocessing_id) {
                            add_preprocessing_id_to_reverse_lookup(
                                self,
                                key_str.as_str(),
                                &this.merchant_id,
                                updated_attempt.attempt_id.as_str(),
                                preprocessing_id.as_str(),
                                storage_scheme,
                            )
                            .await?;
                        }
                    }
                    (_, _) => {}
                }

                Box::pin(kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::Hset::<DieselPaymentAttempt>((&field, redis_value), redis_entry),
                    key,
                ))
                .await
                .change_context(errors::StorageError::KVError)?
                .try_into_hset()
                .change_context(errors::StorageError::KVError)?;

                Ok(updated_attempt)
            }
        }
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn update_payment_attempt(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        // Ignoring storage scheme for v2 implementation
        self.router_store
            .update_payment_attempt(
                key_manager_state,
                merchant_key_store,
                this,
                payment_attempt,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &ConnectorTransactionId,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                        connector_transaction_id,
                        payment_id,
                        merchant_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                // We assume that PaymentAttempt <=> PaymentIntent is a one-to-one relation for now
                let lookup_id = format!(
                    "pa_conn_trans_{}_{}",
                    merchant_id.get_string_repr(),
                    connector_transaction_id.get_id()
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
                            connector_transaction_id,
                            payment_id,
                            merchant_id,
                            storage_scheme,
                        )
                        .await
                );

                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };

                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(self, KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id), key)).await?.try_into_hget()
                    },
                        || async {self.router_store.find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(connector_transaction_id, payment_id, merchant_id, storage_scheme).await},
                    ))
                    .await
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let database_call = || {
            self.router_store
                .find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
                    payment_id,
                    merchant_id,
                    storage_scheme,
                )
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id,
                    payment_id,
                };
                let pattern = "pa_*";

                let redis_fut = async {
                    let kv_result = Box::pin(kv_wrapper::<PaymentAttempt, _, _>(
                        self,
                        KvOperation::<DieselPaymentAttempt>::Scan(pattern),
                        key,
                    ))
                    .await?
                    .try_into_scan();
                    kv_result.and_then(|mut payment_attempts| {
                        payment_attempts.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
                        payment_attempts
                            .iter()
                            .find(|&pa| pa.status == api_models::enums::AttemptStatus::Charged)
                            .cloned()
                            .ok_or(error_stack::report!(
                                redis_interface::errors::RedisError::NotFound
                            ))
                    })
                };
                Box::pin(try_redis_get_else_try_database_get(
                    redis_fut,
                    database_call,
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let database_call = || {
            self.router_store
                .find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
                    payment_id,
                    merchant_id,
                    storage_scheme,
                )
        };
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id,
                    payment_id,
                };
                let pattern = "pa_*";

                let redis_fut = async {
                    let kv_result = Box::pin(kv_wrapper::<PaymentAttempt, _, _>(
                        self,
                        KvOperation::<DieselPaymentAttempt>::Scan(pattern),
                        key,
                    ))
                    .await?
                    .try_into_scan();
                    kv_result.and_then(|mut payment_attempts| {
                        payment_attempts.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
                        payment_attempts
                            .iter()
                            .find(|&pa| {
                                pa.status == api_models::enums::AttemptStatus::Charged
                                    || pa.status == api_models::enums::AttemptStatus::PartialCharged
                            })
                            .cloned()
                            .ok_or(error_stack::report!(
                                redis_interface::errors::RedisError::NotFound
                            ))
                    })
                };
                Box::pin(try_redis_get_else_try_database_get(
                    redis_fut,
                    database_call,
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn find_payment_attempt_by_profile_id_connector_transaction_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        profile_id: &common_utils::id_type::ProfileId,
        connector_transaction_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        // Ignoring storage scheme for v2 implementation
        self.router_store
            .find_payment_attempt_by_profile_id_connector_transaction_id(
                key_manager_state,
                merchant_key_store,
                profile_id,
                connector_transaction_id,
                storage_scheme,
            )
            .await
    }

    #[instrument(skip_all)]
    #[cfg(feature = "v1")]
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_txn_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_attempt_by_merchant_id_connector_txn_id(
                        merchant_id,
                        connector_txn_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id = format!(
                    "pa_conn_trans_{}_{connector_txn_id}",
                    merchant_id.get_string_repr()
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payment_attempt_by_merchant_id_connector_txn_id(
                            merchant_id,
                            connector_txn_id,
                            storage_scheme,
                        )
                        .await
                );

                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payment_attempt_by_merchant_id_connector_txn_id(
                                merchant_id,
                                connector_txn_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &common_utils::id_type::PaymentId,
        merchant_id: &common_utils::id_type::MerchantId,
        attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                        payment_id,
                        merchant_id,
                        attempt_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id,
                    payment_id,
                };
                let field = format!("pa_{attempt_id}");
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&field),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                                payment_id,
                                merchant_id,
                                attempt_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_attempt_by_attempt_id_merchant_id(
                        attempt_id,
                        merchant_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id = format!("pa_{}_{attempt_id}", merchant_id.get_string_repr());
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payment_attempt_by_attempt_id_merchant_id(
                            attempt_id,
                            merchant_id,
                            storage_scheme,
                        )
                        .await
                );

                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payment_attempt_by_attempt_id_merchant_id(
                                attempt_id,
                                merchant_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &MerchantKeyStore,
        attempt_id: &common_utils::id_type::GlobalAttemptId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        // Ignoring storage scheme for v2 implementation
        self.router_store
            .find_payment_attempt_by_id(
                key_manager_state,
                merchant_key_store,
                attempt_id,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v2")]
    #[instrument(skip_all)]
    async fn find_payment_attempts_by_payment_intent_id(
        &self,
        key_manager_state: &KeyManagerState,
        payment_id: &common_utils::id_type::GlobalPaymentId,
        merchant_key_store: &MerchantKeyStore,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError> {
        self.router_store
            .find_payment_attempts_by_payment_intent_id(
                key_manager_state,
                payment_id,
                merchant_key_store,
                storage_scheme,
            )
            .await
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_payment_attempt_by_preprocessing_id_merchant_id(
                        preprocessing_id,
                        merchant_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let lookup_id = format!(
                    "pa_preprocessing_{}_{preprocessing_id}",
                    merchant_id.get_string_repr()
                );
                let lookup = fallback_reverse_lookup_not_found!(
                    self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
                        .await,
                    self.router_store
                        .find_payment_attempt_by_preprocessing_id_merchant_id(
                            preprocessing_id,
                            merchant_id,
                            storage_scheme,
                        )
                        .await
                );
                let key = PartitionKey::CombinationKey {
                    combination: &lookup.pk_id,
                };

                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        ))
                        .await?
                        .try_into_hget()
                    },
                    || async {
                        self.router_store
                            .find_payment_attempt_by_preprocessing_id_merchant_id(
                                preprocessing_id,
                                merchant_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError> {
        let storage_scheme = Box::pin(decide_storage_scheme::<_, DieselPaymentAttempt>(
            self,
            storage_scheme,
            Op::Find,
        ))
        .await;
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .find_attempts_by_merchant_id_payment_id(
                        merchant_id,
                        payment_id,
                        storage_scheme,
                    )
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key = PartitionKey::MerchantIdPaymentId {
                    merchant_id,
                    payment_id,
                };
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        Box::pin(kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::Scan("pa_*"),
                            key,
                        ))
                        .await?
                        .try_into_scan()
                    },
                    || async {
                        self.router_store
                            .find_attempts_by_merchant_id_payment_id(
                                merchant_id,
                                payment_id,
                                storage_scheme,
                            )
                            .await
                    },
                ))
                .await
            }
        }
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &common_utils::id_type::MerchantId,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentListFilters, errors::StorageError> {
        self.router_store
            .get_filters_for_payments(pi, merchant_id, storage_scheme)
            .await
    }

    #[cfg(all(feature = "v1", feature = "olap"))]
    #[instrument(skip_all)]
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        active_attempt_ids: &[String],
        connector: Option<Vec<api_models::enums::Connector>>,
        payment_method: Option<Vec<common_enums::PaymentMethod>>,
        payment_method_type: Option<Vec<common_enums::PaymentMethodType>>,
        authentication_type: Option<Vec<common_enums::AuthenticationType>>,
        merchant_connector_id: Option<Vec<common_utils::id_type::MerchantConnectorAccountId>>,
        card_network: Option<Vec<common_enums::CardNetwork>>,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        self.router_store
            .get_total_count_of_filtered_payment_attempts(
                merchant_id,
                active_attempt_ids,
                connector,
                payment_method,
                payment_method_type,
                authentication_type,
                merchant_connector_id,
                card_network,
                storage_scheme,
            )
            .await
    }
}

impl DataModelExt for MandateAmountData {
    type StorageModel = DieselMandateAmountData;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselMandateAmountData {
            amount: self.amount,
            currency: self.currency,
            start_date: self.start_date,
            end_date: self.end_date,
            metadata: self.metadata,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            amount: storage_model.amount,
            currency: storage_model.currency,
            start_date: storage_model.start_date,
            end_date: storage_model.end_date,
            metadata: storage_model.metadata,
        }
    }
}
impl DataModelExt for MandateDetails {
    type StorageModel = DieselMandateDetails;
    fn to_storage_model(self) -> Self::StorageModel {
        DieselMandateDetails {
            update_mandate_id: self.update_mandate_id,
        }
    }
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            update_mandate_id: storage_model.update_mandate_id,
        }
    }
}

impl DataModelExt for MandateDataType {
    type StorageModel = DieselMandateType;

    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::SingleUse(data) => DieselMandateType::SingleUse(data.to_storage_model()),
            Self::MultiUse(None) => DieselMandateType::MultiUse(None),
            Self::MultiUse(Some(data)) => {
                DieselMandateType::MultiUse(Some(data.to_storage_model()))
            }
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselMandateType::SingleUse(data) => {
                Self::SingleUse(MandateAmountData::from_storage_model(data))
            }
            DieselMandateType::MultiUse(Some(data)) => {
                Self::MultiUse(Some(MandateAmountData::from_storage_model(data)))
            }
            DieselMandateType::MultiUse(None) => Self::MultiUse(None),
        }
    }
}

#[cfg(feature = "v1")]
impl DataModelExt for PaymentAttempt {
    type StorageModel = DieselPaymentAttempt;

    fn to_storage_model(self) -> Self::StorageModel {
        let (connector_transaction_id, connector_transaction_data) = self
            .connector_transaction_id
            .map(ConnectorTransactionId::form_id_and_data)
            .map(|(txn_id, txn_data)| (Some(txn_id), txn_data))
            .unwrap_or((None, None));
        DieselPaymentAttempt {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.net_amount.get_order_amount(),
            net_amount: Some(self.net_amount.get_total_amount()),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            error_code: self.error_code,
            payment_token: self.payment_token,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            card_network: self
                .payment_method_data
                .as_ref()
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card"))
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card_network"))
                .and_then(|network| network.as_str())
                .map(|network| network.to_string()),
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(|d| d.to_storage_model()),
            error_reason: self.error_reason,
            multiple_capture_count: self.multiple_capture_count,
            connector_response_reference_id: self.connector_response_reference_id,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            merchant_connector_id: self.merchant_connector_id,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(|d| d.to_storage_model()),
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            fingerprint_id: self.fingerprint_id,
            charge_id: self.charge_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            connector_transaction_data,
            shipping_cost: self.net_amount.get_shipping_cost(),
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            connector_mandate_detail: self.connector_mandate_detail,
            card_discovery: self.card_discovery,
            charges: self.charges,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        let connector_transaction_id = storage_model
            .get_optional_connector_transaction_id()
            .cloned();
        Self {
            net_amount: hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                storage_model.amount,
                storage_model.shipping_cost,
                storage_model.order_tax_amount,
                storage_model.surcharge_amount,
                storage_model.tax_amount,
            ),
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            attempt_id: storage_model.attempt_id,
            status: storage_model.status,
            currency: storage_model.currency,
            save_to_locker: storage_model.save_to_locker,
            connector: storage_model.connector,
            error_message: storage_model.error_message,
            offer_amount: storage_model.offer_amount,
            payment_method_id: storage_model.payment_method_id,
            payment_method: storage_model.payment_method,
            connector_transaction_id,
            capture_method: storage_model.capture_method,
            capture_on: storage_model.capture_on,
            confirm: storage_model.confirm,
            authentication_type: storage_model.authentication_type,
            created_at: storage_model.created_at,
            modified_at: storage_model.modified_at,
            last_synced: storage_model.last_synced,
            cancellation_reason: storage_model.cancellation_reason,
            amount_to_capture: storage_model.amount_to_capture,
            mandate_id: storage_model.mandate_id,
            browser_info: storage_model.browser_info,
            error_code: storage_model.error_code,
            payment_token: storage_model.payment_token,
            connector_metadata: storage_model.connector_metadata,
            payment_experience: storage_model.payment_experience,
            payment_method_type: storage_model.payment_method_type,
            payment_method_data: storage_model.payment_method_data,
            business_sub_label: storage_model.business_sub_label,
            straight_through_algorithm: storage_model.straight_through_algorithm,
            preprocessing_step_id: storage_model.preprocessing_step_id,
            mandate_details: storage_model
                .mandate_details
                .map(MandateDataType::from_storage_model),
            error_reason: storage_model.error_reason,
            multiple_capture_count: storage_model.multiple_capture_count,
            connector_response_reference_id: storage_model.connector_response_reference_id,
            amount_capturable: storage_model.amount_capturable,
            updated_by: storage_model.updated_by,
            authentication_data: storage_model.authentication_data,
            encoded_data: storage_model.encoded_data,
            merchant_connector_id: storage_model.merchant_connector_id,
            unified_code: storage_model.unified_code,
            unified_message: storage_model.unified_message,
            external_three_ds_authentication_attempted: storage_model
                .external_three_ds_authentication_attempted,
            authentication_connector: storage_model.authentication_connector,
            authentication_id: storage_model.authentication_id,
            mandate_data: storage_model
                .mandate_data
                .map(MandateDetails::from_storage_model),
            payment_method_billing_address_id: storage_model.payment_method_billing_address_id,
            fingerprint_id: storage_model.fingerprint_id,
            charge_id: storage_model.charge_id,
            client_source: storage_model.client_source,
            client_version: storage_model.client_version,
            customer_acceptance: storage_model.customer_acceptance,
            organization_id: storage_model.organization_id,
            profile_id: storage_model.profile_id,
            connector_mandate_detail: storage_model.connector_mandate_detail,
            card_discovery: storage_model.card_discovery,
            charges: storage_model.charges,
        }
    }
}

#[cfg(feature = "v1")]
impl DataModelExt for PaymentAttemptNew {
    type StorageModel = DieselPaymentAttemptNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttemptNew {
            net_amount: Some(self.net_amount.get_total_amount()),
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.net_amount.get_order_amount(),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.net_amount.get_surcharge_amount(),
            tax_amount: self.net_amount.get_tax_on_surcharge(),
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            capture_method: self.capture_method,
            capture_on: self.capture_on,
            confirm: self.confirm,
            authentication_type: self.authentication_type,
            created_at: self.created_at.unwrap_or_else(common_utils::date_time::now),
            modified_at: self
                .modified_at
                .unwrap_or_else(common_utils::date_time::now),
            last_synced: self.last_synced,
            cancellation_reason: self.cancellation_reason,
            amount_to_capture: self.amount_to_capture,
            mandate_id: self.mandate_id,
            browser_info: self.browser_info,
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
            card_network: self
                .payment_method_data
                .as_ref()
                .and_then(|data| data.as_object())
                .and_then(|card| card.get("card"))
                .and_then(|value| value.as_object())
                .and_then(|map| map.get("card_network"))
                .and_then(|network| network.as_str())
                .map(|network| network.to_string()),
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(|d| d.to_storage_model()),
            error_reason: self.error_reason,
            connector_response_reference_id: self.connector_response_reference_id,
            multiple_capture_count: self.multiple_capture_count,
            amount_capturable: self.amount_capturable,
            updated_by: self.updated_by,
            authentication_data: self.authentication_data,
            encoded_data: self.encoded_data,
            merchant_connector_id: self.merchant_connector_id,
            unified_code: self.unified_code,
            unified_message: self.unified_message,
            external_three_ds_authentication_attempted: self
                .external_three_ds_authentication_attempted,
            authentication_connector: self.authentication_connector,
            authentication_id: self.authentication_id,
            mandate_data: self.mandate_data.map(|d| d.to_storage_model()),
            payment_method_billing_address_id: self.payment_method_billing_address_id,
            fingerprint_id: self.fingerprint_id,
            client_source: self.client_source,
            client_version: self.client_version,
            customer_acceptance: self.customer_acceptance,
            organization_id: self.organization_id,
            profile_id: self.profile_id,
            shipping_cost: self.net_amount.get_shipping_cost(),
            order_tax_amount: self.net_amount.get_order_tax_amount(),
            connector_mandate_detail: self.connector_mandate_detail,
            card_discovery: self.card_discovery,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            net_amount: hyperswitch_domain_models::payments::payment_attempt::NetAmount::new(
                storage_model.amount,
                storage_model.shipping_cost,
                storage_model.order_tax_amount,
                storage_model.surcharge_amount,
                storage_model.tax_amount,
            ),
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            attempt_id: storage_model.attempt_id,
            status: storage_model.status,
            currency: storage_model.currency,
            save_to_locker: storage_model.save_to_locker,
            connector: storage_model.connector,
            error_message: storage_model.error_message,
            offer_amount: storage_model.offer_amount,
            payment_method_id: storage_model.payment_method_id,
            payment_method: storage_model.payment_method,
            capture_method: storage_model.capture_method,
            capture_on: storage_model.capture_on,
            confirm: storage_model.confirm,
            authentication_type: storage_model.authentication_type,
            created_at: Some(storage_model.created_at),
            modified_at: Some(storage_model.modified_at),
            last_synced: storage_model.last_synced,
            cancellation_reason: storage_model.cancellation_reason,
            amount_to_capture: storage_model.amount_to_capture,
            mandate_id: storage_model.mandate_id,
            browser_info: storage_model.browser_info,
            payment_token: storage_model.payment_token,
            error_code: storage_model.error_code,
            connector_metadata: storage_model.connector_metadata,
            payment_experience: storage_model.payment_experience,
            payment_method_type: storage_model.payment_method_type,
            payment_method_data: storage_model.payment_method_data,
            business_sub_label: storage_model.business_sub_label,
            straight_through_algorithm: storage_model.straight_through_algorithm,
            preprocessing_step_id: storage_model.preprocessing_step_id,
            mandate_details: storage_model
                .mandate_details
                .map(MandateDataType::from_storage_model),
            error_reason: storage_model.error_reason,
            connector_response_reference_id: storage_model.connector_response_reference_id,
            multiple_capture_count: storage_model.multiple_capture_count,
            amount_capturable: storage_model.amount_capturable,
            updated_by: storage_model.updated_by,
            authentication_data: storage_model.authentication_data,
            encoded_data: storage_model.encoded_data,
            merchant_connector_id: storage_model.merchant_connector_id,
            unified_code: storage_model.unified_code,
            unified_message: storage_model.unified_message,
            external_three_ds_authentication_attempted: storage_model
                .external_three_ds_authentication_attempted,
            authentication_connector: storage_model.authentication_connector,
            authentication_id: storage_model.authentication_id,
            mandate_data: storage_model
                .mandate_data
                .map(MandateDetails::from_storage_model),
            payment_method_billing_address_id: storage_model.payment_method_billing_address_id,
            fingerprint_id: storage_model.fingerprint_id,
            client_source: storage_model.client_source,
            client_version: storage_model.client_version,
            customer_acceptance: storage_model.customer_acceptance,
            organization_id: storage_model.organization_id,
            profile_id: storage_model.profile_id,
            connector_mandate_detail: storage_model.connector_mandate_detail,
            card_discovery: storage_model.card_discovery,
        }
    }
}

#[inline]
#[instrument(skip_all)]
async fn add_connector_txn_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    updated_attempt_attempt_id: &str,
    connector_transaction_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> CustomResult<ReverseLookup, errors::StorageError> {
    let field = format!("pa_{}", updated_attempt_attempt_id);
    let reverse_lookup_new = ReverseLookupNew {
        lookup_id: format!(
            "pa_conn_trans_{}_{}",
            merchant_id.get_string_repr(),
            connector_transaction_id
        ),
        pk_id: key.to_owned(),
        sk_id: field.clone(),
        source: "payment_attempt".to_string(),
        updated_by: storage_scheme.to_string(),
    };
    store
        .insert_reverse_lookup(reverse_lookup_new, storage_scheme)
        .await
}

#[inline]
#[instrument(skip_all)]
async fn add_preprocessing_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    updated_attempt_attempt_id: &str,
    preprocessing_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> CustomResult<ReverseLookup, errors::StorageError> {
    let field = format!("pa_{}", updated_attempt_attempt_id);
    let reverse_lookup_new = ReverseLookupNew {
        lookup_id: format!(
            "pa_preprocessing_{}_{}",
            merchant_id.get_string_repr(),
            preprocessing_id
        ),
        pk_id: key.to_owned(),
        sk_id: field.clone(),
        source: "payment_attempt".to_string(),
        updated_by: storage_scheme.to_string(),
    };
    store
        .insert_reverse_lookup(reverse_lookup_new, storage_scheme)
        .await
}
