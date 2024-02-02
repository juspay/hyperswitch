use api_models::enums::{AuthenticationType, Connector, PaymentMethod, PaymentMethodType};
use common_utils::{errors::CustomResult, fallback_reverse_lookup_not_found};
use data_models::{
    errors,
    mandates::{MandateAmountData, MandateDataType, MandateDetails, MandateTypeDetails},
    payments::{
        payment_attempt::{
            PaymentAttempt, PaymentAttemptInterface, PaymentAttemptNew, PaymentAttemptUpdate,
            PaymentListFilters,
        },
        PaymentIntent,
    },
};
use diesel_models::{
    enums::{
        MandateAmountData as DieselMandateAmountData, MandateDataType as DieselMandateType,
        MandateDetails as DieselMandateDetails, MandateTypeDetails as DieselMandateTypeOrDetails,
        MerchantStorageScheme,
    },
    kv,
    payment_attempt::{
        PaymentAttempt as DieselPaymentAttempt, PaymentAttemptNew as DieselPaymentAttemptNew,
        PaymentAttemptUpdate as DieselPaymentAttemptUpdate,
    },
    reverse_lookup::{ReverseLookup, ReverseLookupNew},
};
use error_stack::{IntoReport, ResultExt};
use redis_interface::HsetnxReply;
use router_env::{instrument, tracing};

use crate::{
    diesel_error_to_data_error,
    errors::RedisErrorExt,
    lookup::ReverseLookupInterface,
    redis::kv_store::{kv_wrapper, KvOperation},
    utils::{pg_connection_read, pg_connection_write, try_redis_get_else_try_database_get},
    DataModelExt, DatabaseStore, KVRouterStore, RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for RouterStore<T> {
    #[instrument(skip_all)]
    /// Inserts a new payment attempt into the database using the provided payment attempt data
    /// and storage scheme. Returns a result containing the inserted payment attempt or a storage error.
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
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    /// Updates a payment attempt with a specified attempt ID using the provided payment attempt update and merchant storage scheme.
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
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    /// Asynchronously finds a payment attempt by the given connector transaction ID, payment ID, and merchant ID using the specified storage scheme.
    /// Returns a `CustomResult` containing the found `PaymentAttempt` or a `StorageError` in case of an error.
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    /// Asynchronously finds the last successful payment attempt by the given payment ID and merchant ID using the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `payment_id` - The ID of the payment to search for
    /// * `merchant_id` - The ID of the merchant associated with the payment
    /// * `_storage_scheme` - The storage scheme to use for retrieving the payment attempt
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the found `PaymentAttempt` or a `StorageError` if the operation fails.
    ///
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    /// Asynchronously finds the last successful or partially captured payment attempt by the given payment ID and merchant ID, using the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `payment_id` - A string slice representing the payment ID
    /// * `merchant_id` - A string slice representing the merchant ID
    /// * `_storage_scheme` - The storage scheme to be used for the merchant
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the found `PaymentAttempt` or a `StorageError` if an error occurs during the database operation.
    ///
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    /// Asynchronously finds a payment attempt by the given merchant ID and connector transaction ID using the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID
    /// * `connector_txn_id` - A reference to a string representing the connector transaction ID
    /// * `_storage_scheme` - The storage scheme to be used for the merchant
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing either the found `PaymentAttempt` or a `StorageError`
    ///
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    #[instrument(skip_all)]
    /// Asynchronously finds a payment attempt by the given payment ID, merchant ID, and attempt ID using the specified merchant storage scheme.
    /// Returns a `CustomResult` containing a `PaymentAttempt` or a `StorageError` from the `errors` module.
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    /// Retrieves payment filters based on the provided PaymentIntent and merchant ID.
    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentListFilters, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        let intents = pi
            .iter()
            .cloned()
            .map(|pi| pi.to_storage_model())
            .collect::<Vec<diesel_models::PaymentIntent>>();
        DieselPaymentAttempt::get_filters_for_payments(&conn, intents.as_slice(), merchant_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(er.current_context());
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

    /// Asynchronously finds a payment attempt by its preprocessing ID and merchant ID, using the specified storage scheme. Returns a custom result containing the payment attempt if found, or a storage error if not.
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &str,
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
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    /// Asynchronously finds payment attempts by merchant ID and payment ID using the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - A reference to a string representing the merchant ID.
    /// * `payment_id` - A reference to a string representing the payment ID.
    /// * `_storage_scheme` - The storage scheme to be used for retrieving payment attempts.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a vector of `PaymentAttempt` objects, or a `StorageError` if an error occurs.
    ///
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<Vec<PaymentAttempt>, errors::StorageError> {
        let conn = pg_connection_read(self).await?;
        DieselPaymentAttempt::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(|a| {
                a.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }

    /// Asynchronously finds a payment attempt by its attempt ID and merchant ID using the specified storage scheme.
    /// Returns a custom result containing a PaymentAttempt or a StorageError if an error occurs during the process.
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_merchant_id_attempt_id(&conn, merchant_id, attempt_id)
            .await
            .map_err(|er| {
                let new_err = diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

    /// Asynchronously retrieves the total count of filtered payment attempts based on the provided criteria.
    ///
    /// # Arguments
    /// * `merchant_id` - The ID of the merchant for which the payment attempts should be filtered.
    /// * `active_attempt_ids` - A list of active attempt IDs.
    /// * `connector` - An optional list of connectors.
    /// * `payment_method` - An optional list of payment methods.
    /// * `payment_method_type` - An optional list of payment method types.
    /// * `authentication_type` - An optional list of authentication types.
    /// * `_storage_scheme` - The storage scheme used by the merchant.
    ///
    /// # Returns
    /// A Result containing the total count of filtered payment attempts if successful, or a StorageError if an error occurs.
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &str,
        active_attempt_ids: &[String],
        connector: Option<Vec<Connector>>,
        payment_method: Option<Vec<PaymentMethod>>,
        payment_method_type: Option<Vec<PaymentMethodType>>,
        authentication_type: Option<Vec<AuthenticationType>>,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<i64, errors::StorageError> {
        let conn = self
            .db_store
            .get_replica_pool()
            .get()
            .await
            .into_report()
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
        )
        .await
        .map_err(|er| {
            let new_err = diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for KVRouterStore<T> {
    #[instrument(skip_all)]
    /// Inserts a new payment attempt into the database based on the specified storage scheme. If the storage scheme is PostgresOnly, the payment attempt is inserted into the Postgres database. If the storage scheme is RedisKv, the payment attempt is inserted into the Redis key-value store along with a reverse lookup entry. Returns the created payment attempt if successful.
    /// Inserts a payment attempt into the appropriate data storage based on the provided storage scheme.
    /// Returns the inserted PaymentAttempt if successful, otherwise returns a StorageError.
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .insert_payment_attempt(payment_attempt, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let payment_attempt = payment_attempt.populate_derived_fields();
                let key = format!(
                    "mid_{}_pid_{}",
                    payment_attempt.merchant_id, payment_attempt.payment_id
                );

                let created_attempt = PaymentAttempt {
                    id: Default::default(),
                    payment_id: payment_attempt.payment_id.clone(),
                    merchant_id: payment_attempt.merchant_id.clone(),
                    attempt_id: payment_attempt.attempt_id.clone(),
                    status: payment_attempt.status,
                    amount: payment_attempt.amount,
                    net_amount: payment_attempt.net_amount,
                    currency: payment_attempt.currency,
                    save_to_locker: payment_attempt.save_to_locker,
                    connector: payment_attempt.connector.clone(),
                    error_message: payment_attempt.error_message.clone(),
                    offer_amount: payment_attempt.offer_amount,
                    surcharge_amount: payment_attempt.surcharge_amount,
                    tax_amount: payment_attempt.tax_amount,
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
                    amount_capturable: payment_attempt.amount_capturable,
                    updated_by: storage_scheme.to_string(),
                    authentication_data: payment_attempt.authentication_data.clone(),
                    encoded_data: payment_attempt.encoded_data.clone(),
                    merchant_connector_id: payment_attempt.merchant_connector_id.clone(),
                    unified_code: payment_attempt.unified_code.clone(),
                    unified_message: payment_attempt.unified_message.clone(),
                };

                let field = format!("pa_{}", created_attempt.attempt_id);

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Insert {
                        insertable: kv::Insertable::PaymentAttempt(
                            payment_attempt.to_storage_model(),
                        ),
                    },
                };

                //Reverse lookup for attempt_id
                let reverse_lookup = ReverseLookupNew {
                    lookup_id: format!(
                        "pa_{}_{}",
                        &created_attempt.merchant_id, &created_attempt.attempt_id,
                    ),
                    pk_id: key.clone(),
                    sk_id: field.clone(),
                    source: "payment_attempt".to_string(),
                    updated_by: storage_scheme.to_string(),
                };
                self.insert_reverse_lookup(reverse_lookup, storage_scheme)
                    .await?;

                match kv_wrapper::<PaymentAttempt, _, _>(
                    self,
                    KvOperation::HSetNx(
                        &field,
                        &created_attempt.clone().to_storage_model(),
                        redis_entry,
                    ),
                    &key,
                )
                .await
                .map_err(|err| err.to_redis_failed_response(&key))?
                .try_into_hsetnx()
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: "payment attempt",
                        key: Some(key),
                    })
                    .into_report(),
                    Ok(HsetnxReply::KeySet) => Ok(created_attempt),
                    Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                }
            }
        }
    }

    /// Asynchronously updates a payment attempt with the given attempt ID based on the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `this` - The original PaymentAttempt to be updated
    /// * `payment_attempt` - The PaymentAttemptUpdate containing the changes to be applied
    /// * `storage_scheme` - The storage scheme to be used for updating the payment attempt
    ///
    /// # Returns
    ///
    /// Returns a Result containing the updated PaymentAttempt if the update is successful, or a StorageError if an error occurs.
    ///
    #[instrument(skip_all)]
        /// Updates a payment attempt with the given attempt ID based on the specified storage scheme.
    /// If the storage scheme is PostgresOnly, the payment attempt is updated in the Postgres database using the router store.
    /// If the storage scheme is RedisKv, the payment attempt is updated in the Redis key-value store, including adding and updating reverse lookups for connector transaction ID and preprocessing ID.
    /// 
    /// # Arguments
    /// 
    /// * `this` - The payment attempt to be updated
    /// * `payment_attempt` - The updated payment attempt data
    /// * `storage_scheme` - The storage scheme to be used for updating the payment attempt
    /// 
    /// # Returns
    /// 
    /// The updated payment attempt if the update is successful, otherwise returns a StorageError.
    ///
    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => {
                self.router_store
                    .update_payment_attempt_with_attempt_id(this, payment_attempt, storage_scheme)
                    .await
            }
            MerchantStorageScheme::RedisKv => {
                let key = format!("mid_{}_pid_{}", this.merchant_id, this.payment_id);
                let old_connector_transaction_id = &this.connector_transaction_id;
                let old_preprocessing_id = &this.preprocessing_step_id;
                let updated_attempt = PaymentAttempt::from_storage_model(
                    payment_attempt
                        .clone()
                        .to_storage_model()
                        .apply_changeset(this.clone().to_storage_model()),
                );
                // Check for database presence as well Maybe use a read replica here ?
                let redis_value = serde_json::to_string(&updated_attempt)
                    .into_report()
                    .change_context(errors::StorageError::KVError)?;
                let field = format!("pa_{}", updated_attempt.attempt_id);

                let redis_entry = kv::TypedSql {
                    op: kv::DBOperation::Update {
                        updatable: kv::Updateable::PaymentAttemptUpdate(
                            kv::PaymentAttemptUpdateMems {
                                orig: this.clone().to_storage_model(),
                                update_data: payment_attempt.to_storage_model(),
                            },
                        ),
                    },
                };

                match (
                    old_connector_transaction_id,
                    &updated_attempt.connector_transaction_id,
                ) {
                    (None, Some(connector_transaction_id)) => {
                        add_connector_txn_id_to_reverse_lookup(
                            self,
                            key.as_str(),
                            this.merchant_id.as_str(),
                            updated_attempt.attempt_id.as_str(),
                            connector_transaction_id.as_str(),
                            storage_scheme,
                        )
                        .await?;
                    }
                    (Some(old_connector_transaction_id), Some(connector_transaction_id)) => {
                        if old_connector_transaction_id.ne(connector_transaction_id) {
                            add_connector_txn_id_to_reverse_lookup(
                                self,
                                key.as_str(),
                                this.merchant_id.as_str(),
                                updated_attempt.attempt_id.as_str(),
                                connector_transaction_id.as_str(),
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
                            key.as_str(),
                            this.merchant_id.as_str(),
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
                                key.as_str(),
                                this.merchant_id.as_str(),
                                updated_attempt.attempt_id.as_str(),
                                preprocessing_id.as_str(),
                                storage_scheme,
                            )
                            .await?;
                        }
                    }
                    (_, _) => {}
                }

                kv_wrapper::<(), _, _>(
                    self,
                    KvOperation::Hset::<DieselPaymentAttempt>((&field, redis_value), redis_entry),
                    &key,
                )
                .await
                .change_context(errors::StorageError::KVError)?
                .try_into_hset()
                .change_context(errors::StorageError::KVError)?;

                Ok(updated_attempt)
            }
        }
    }

    /// This method finds a payment attempt based on the given connector transaction ID, payment ID, and merchant ID using the specified storage scheme. It returns a Result containing the found PaymentAttempt or a StorageError.
    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
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
                let lookup_id = format!("pa_conn_trans_{merchant_id}_{connector_transaction_id}");
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

                let key = &lookup.pk_id;

                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(self, KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id), key).await?.try_into_hget()
                    },
                        || async {self.router_store.find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(connector_transaction_id, payment_id, merchant_id, storage_scheme).await},
                    ))
                    .await
            }
        }
    }

    /// Asynchronously finds the last successful payment attempt by payment ID and merchant ID using the specified storage scheme. If the storage scheme is PostgresOnly, it makes a database call to retrieve the payment attempt. If the storage scheme is RedisKv, it constructs a key and pattern, then performs a scan operation on the Redis key-value store to find the last successful payment attempt. If the attempt is found in Redis, it is returned; otherwise, a database call is made to retrieve the attempt.
    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = format!("mid_{merchant_id}_pid_{payment_id}");
                let pattern = "pa_*";

                let redis_fut = async {
                    let kv_result = kv_wrapper::<PaymentAttempt, _, _>(
                        self,
                        KvOperation::<DieselPaymentAttempt>::Scan(pattern),
                        key,
                    )
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

    /// Asynchronously finds the last successful or partially captured payment attempt by payment ID and merchant ID. The method takes the payment ID, merchant ID, and the storage scheme as input parameters and returns a Result containing the PaymentAttempt or a StorageError. Depending on the storage scheme, the method makes a database or Redis key-value store call to retrieve the payment attempt. If the storage scheme is PostgresOnly, it directly calls the database to retrieve the payment attempt. If the storage scheme is RedisKv, it constructs a key and pattern, then asynchronously scans the Redis key-value store to find the payment attempts. It then filters the payment attempts based on their status and returns the last successful or partially captured attempt. If the attempt is not found in the Redis store, it falls back to the database call to retrieve the attempt.
    async fn find_payment_attempt_last_successful_or_partially_captured_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
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
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => database_call().await,
            MerchantStorageScheme::RedisKv => {
                let key = format!("mid_{merchant_id}_pid_{payment_id}");
                let pattern = "pa_*";

                let redis_fut = async {
                    let kv_result = kv_wrapper::<PaymentAttempt, _, _>(
                        self,
                        KvOperation::<DieselPaymentAttempt>::Scan(pattern),
                        key,
                    )
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

    /// Asynchronously finds a payment attempt by merchant ID and connector transaction ID based on the specified storage scheme.
    ///
    /// # Arguments
    ///
    /// * `merchant_id` - The ID of the merchant
    /// * `connector_txn_id` - The connector transaction ID
    /// * `storage_scheme` - The storage scheme to be used (PostgresOnly or RedisKv)
    ///
    /// # Returns
    ///
    /// `Result` containing a `PaymentAttempt` if successful, otherwise a `StorageError`
    ///
    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
        connector_txn_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
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
                let lookup_id = format!("pa_conn_trans_{merchant_id}_{connector_txn_id}");
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

                let key = &lookup.pk_id;
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        )
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

    #[instrument(skip_all)]
    /// Asynchronously finds a payment attempt by its payment ID, merchant ID, and attempt ID based on the specified storage scheme.
    /// Returns a `Result` containing a `PaymentAttempt` if successful, or a `StorageError` if an error occurs.
    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
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
                let key = format!("mid_{merchant_id}_pid_{payment_id}");
                let field = format!("pa_{attempt_id}");
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(self, KvOperation::<DieselPaymentAttempt>::HGet(&field), key)
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

    /// Asynchronously finds a payment attempt by the given attempt ID and merchant ID using the specified storage scheme.
    ///
    /// # Arguments
    /// * `attempt_id` - The ID of the payment attempt to find
    /// * `merchant_id` - The ID of the merchant associated with the payment attempt
    /// * `storage_scheme` - The storage scheme to use for the lookup
    ///
    /// # Returns
    /// The result of the payment attempt lookup, wrapped in a `Result` with the error type `errors::StorageError`
    ///
    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
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
                let lookup_id = format!("pa_{merchant_id}_{attempt_id}");
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

                let key = &lookup.pk_id;
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        )
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

    /// Asynchronously finds a payment attempt by preprocessing ID and merchant ID using the specified storage scheme.
    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
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
                let lookup_id = format!("pa_preprocessing_{merchant_id}_{preprocessing_id}");
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
                let key = &lookup.pk_id;

                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(
                            self,
                            KvOperation::<DieselPaymentAttempt>::HGet(&lookup.sk_id),
                            key,
                        )
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

    /// Asynchronously finds payment attempts by merchant ID and payment ID based on the specified storage scheme.
    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError> {
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
                let key = format!("mid_{merchant_id}_pid_{payment_id}");
                Box::pin(try_redis_get_else_try_database_get(
                    async {
                        kv_wrapper(self, KvOperation::<DieselPaymentAttempt>::Scan("pa_*"), key)
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

    /// Asynchronously retrieves the filters for payments based on the provided PaymentIntent, merchant ID, and storage scheme.
    ///
    /// # Arguments
    ///
    /// * `pi` - A reference to an array of PaymentIntent objects
    /// * `merchant_id` - A reference to the merchant ID
    /// * `storage_scheme` - The storage scheme used by the merchant
    ///
    /// # Returns
    ///
    /// Returns a Result containing the PaymentListFilters if successful, or a StorageError if an error occurs.
    ///
    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentListFilters, errors::StorageError> {
        self.router_store
            .get_filters_for_payments(pi, merchant_id, storage_scheme)
            .await
    }

    /// Retrieves the total count of filtered payment attempts based on the provided parameters.
    async fn get_total_count_of_filtered_payment_attempts(
        &self,
        merchant_id: &str,
        active_attempt_ids: &[String],
        connector: Option<Vec<Connector>>,
        payment_method: Option<Vec<PaymentMethod>>,
        payment_method_type: Option<Vec<PaymentMethodType>>,
        authentication_type: Option<Vec<AuthenticationType>>,
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
                storage_scheme,
            )
            .await
    }
}

impl DataModelExt for MandateAmountData {
    type StorageModel = DieselMandateAmountData;

    /// Converts the current object into its corresponding storage model representation
    fn to_storage_model(self) -> Self::StorageModel {
        DieselMandateAmountData {
            amount: self.amount,
            currency: self.currency,
            start_date: self.start_date,
            end_date: self.end_date,
            metadata: self.metadata,
        }
    }

    /// Converts a storage model to the current model, extracting and assigning its properties.
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
    /// Converts a MandateDetails struct to its corresponding storage model representation
    fn to_storage_model(self) -> Self::StorageModel {
        DieselMandateDetails {
            update_mandate_id: self.update_mandate_id,
            mandate_type: self
                .mandate_type
                .map(|mand_type| mand_type.to_storage_model()),
        }
    }
    /// Creates a new instance of `Self` from the provided `storage_model`.
    ///
    /// # Arguments
    ///
    /// * `storage_model` - The storage model to create the instance from.
    ///
    /// # Returns
    ///
    /// The new instance of `Self` created from the `storage_model`.
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            update_mandate_id: storage_model.update_mandate_id,
            mandate_type: storage_model
                .mandate_type
                .map(MandateDataType::from_storage_model),
        }
    }
}
impl DataModelExt for MandateTypeDetails {
    type StorageModel = DieselMandateTypeOrDetails;

    /// Converts the enum variant into its corresponding storage model variant.
    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::MandateType(mandate_type) => {
                DieselMandateTypeOrDetails::MandateType(mandate_type.to_storage_model())
            }
            Self::MandateDetails(mandate_details) => {
                DieselMandateTypeOrDetails::MandateDetails(mandate_details.to_storage_model())
            }
        }
    }

    /// Converts the given storage model into the corresponding enum variant of the current type.
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselMandateTypeOrDetails::MandateType(data) => {
                Self::MandateType(MandateDataType::from_storage_model(data))
            }
            DieselMandateTypeOrDetails::MandateDetails(data) => {
                Self::MandateDetails(MandateDetails::from_storage_model(data))
            }
        }
    }
}

impl DataModelExt for MandateDataType {
    type StorageModel = DieselMandateType;

    /// Converts the current instance of DieselMandateType enum to its corresponding StorageModel enum.
    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::SingleUse(data) => DieselMandateType::SingleUse(data.to_storage_model()),
            Self::MultiUse(None) => DieselMandateType::MultiUse(None),
            Self::MultiUse(Some(data)) => {
                DieselMandateType::MultiUse(Some(data.to_storage_model()))
            }
        }
    }

    /// Converts the given `storage_model` into the corresponding `Self` enum variant.
    /// If the `storage_model` is a `DieselMandateType::SingleUse`, it creates a `Self::SingleUse` variant
    /// with the data converted from the storage model.
    /// If the `storage_model` is a `DieselMandateType::MultiUse` with some data, it creates a `Self::MultiUse`
    /// variant with the data converted from the storage model.
    /// If the `storage_model` is a `DieselMandateType::MultiUse` with no data, it creates a `Self::MultiUse` variant
    /// with no data.
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

impl DataModelExt for PaymentAttempt {
    type StorageModel = DieselPaymentAttempt;

    /// Converts the current payment attempt object into a storage model for Diesel.
    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttempt {
            id: self.id,
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            net_amount: Some(self.net_amount),
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
            connector_transaction_id: self.connector_transaction_id,
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
            payment_method_data: self.payment_method_data,
            business_sub_label: self.business_sub_label,
            straight_through_algorithm: self.straight_through_algorithm,
            preprocessing_step_id: self.preprocessing_step_id,
            mandate_details: self.mandate_details.map(|md| md.to_storage_model()),
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
        }
    }

        /// Creates a new instance of the current struct from the provided storage model by mapping its fields.
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            net_amount: storage_model.get_or_calculate_net_amount(),
            id: storage_model.id,
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            attempt_id: storage_model.attempt_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            save_to_locker: storage_model.save_to_locker,
            connector: storage_model.connector,
            error_message: storage_model.error_message,
            offer_amount: storage_model.offer_amount,
            surcharge_amount: storage_model.surcharge_amount,
            tax_amount: storage_model.tax_amount,
            payment_method_id: storage_model.payment_method_id,
            payment_method: storage_model.payment_method,
            connector_transaction_id: storage_model.connector_transaction_id,
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
                .map(MandateTypeDetails::from_storage_model),
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
        }
    }
}

impl DataModelExt for PaymentAttemptNew {
    type StorageModel = DieselPaymentAttemptNew;

    /// Converts a PaymentAttempt struct to its corresponding StorageModel struct
    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttemptNew {
            net_amount: Some(self.net_amount),
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            attempt_id: self.attempt_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            save_to_locker: self.save_to_locker,
            connector: self.connector,
            error_message: self.error_message,
            offer_amount: self.offer_amount,
            surcharge_amount: self.surcharge_amount,
            tax_amount: self.tax_amount,
            payment_method_id: self.payment_method_id,
            payment_method: self.payment_method,
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
            payment_token: self.payment_token,
            error_code: self.error_code,
            connector_metadata: self.connector_metadata,
            payment_experience: self.payment_experience,
            payment_method_type: self.payment_method_type,
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
        }
    }

        /// Transforms the given storage model into the current struct by mapping its fields
    /// to the corresponding fields in the current struct, and performing any necessary
    /// calculations or transformations.
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            net_amount: storage_model.get_or_calculate_net_amount(),
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            attempt_id: storage_model.attempt_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            save_to_locker: storage_model.save_to_locker,
            connector: storage_model.connector,
            error_message: storage_model.error_message,
            offer_amount: storage_model.offer_amount,
            surcharge_amount: storage_model.surcharge_amount,
            tax_amount: storage_model.tax_amount,
            payment_method_id: storage_model.payment_method_id,
            payment_method: storage_model.payment_method,
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
                .map(MandateTypeDetails::from_storage_model),
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
        }
    }
}

impl DataModelExt for PaymentAttemptUpdate {
    type StorageModel = DieselPaymentAttemptUpdate;

    /// Converts the enum variant into its corresponding StorageModel enum variant.
    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::Update {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount,
                tax_amount,
                updated_by,
            } => DieselPaymentAttemptUpdate::Update {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount,
                tax_amount,
                updated_by,
            },
            Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                updated_by,
                surcharge_amount,
                tax_amount,
                merchant_connector_id,
            } => DieselPaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id,
            },
            Self::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            } => DieselPaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            },
            Self::ConfirmUpdate {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: connector_id,
            } => DieselPaymentAttemptUpdate::ConfirmUpdate {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: connector_id,
            },
            Self::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            } => DieselPaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            },
            Self::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
            } => DieselPaymentAttemptUpdate::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
            },
            Self::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            },
            Self::StatusUpdate { status, updated_by } => {
                DieselPaymentAttemptUpdate::StatusUpdate { status, updated_by }
            }
            Self::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            } => DieselPaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            },
            Self::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            } => DieselPaymentAttemptUpdate::CaptureUpdate {
                multiple_capture_count,
                updated_by,
                amount_to_capture,
            },
            Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            } => DieselPaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            },
            Self::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => DieselPaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            Self::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            } => DieselPaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            },
            Self::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                updated_by,
            } => DieselPaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                updated_by,
            },
            Self::IncrementalAuthorizationAmountUpdate {
                amount,
                amount_capturable,
            } => DieselPaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                amount,
                amount_capturable,
            },
        }
    }

        /// Converts a `StorageModel` into the corresponding `Self` enum variant.
    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselPaymentAttemptUpdate::Update {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount,
                tax_amount,
                updated_by,
            } => Self::Update {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                amount_to_capture,
                capture_method,
                surcharge_amount,
                tax_amount,
                updated_by,
            },
            DieselPaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                updated_by,
                surcharge_amount,
                tax_amount,
                merchant_connector_id: connector_id,
            } => Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: connector_id,
            },
            DieselPaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            } => Self::AuthenticationTypeUpdate {
                authentication_type,
                updated_by,
            },
            DieselPaymentAttemptUpdate::ConfirmUpdate {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: connector_id,
            } => Self::ConfirmUpdate {
                amount,
                currency,
                status,
                authentication_type,
                payment_method,
                browser_info,
                connector,
                payment_token,
                payment_method_data,
                payment_method_type,
                payment_experience,
                business_sub_label,
                straight_through_algorithm,
                error_code,
                error_message,
                amount_capturable,
                surcharge_amount,
                tax_amount,
                updated_by,
                merchant_connector_id: connector_id,
            },
            DieselPaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            } => Self::VoidUpdate {
                status,
                cancellation_reason,
                updated_by,
            },
            DieselPaymentAttemptUpdate::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
            } => Self::ResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                authentication_type,
                payment_method_id,
                mandate_id,
                connector_metadata,
                payment_token,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                amount_capturable,
                updated_by,
                authentication_data,
                encoded_data,
                unified_code,
                unified_message,
            },
            DieselPaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            } => Self::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
                updated_by,
            },
            DieselPaymentAttemptUpdate::StatusUpdate { status, updated_by } => {
                Self::StatusUpdate { status, updated_by }
            }
            DieselPaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            } => Self::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
                amount_capturable,
                updated_by,
                unified_code,
                unified_message,
                connector_transaction_id,
            },
            DieselPaymentAttemptUpdate::CaptureUpdate {
                amount_to_capture,
                multiple_capture_count,
                updated_by,
            } => Self::CaptureUpdate {
                amount_to_capture,
                multiple_capture_count,
                updated_by,
            },
            DieselPaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            } => Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
                updated_by,
            },
            DieselPaymentAttemptUpdate::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            } => Self::RejectUpdate {
                status,
                error_code,
                error_message,
                updated_by,
            },
            DieselPaymentAttemptUpdate::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            } => Self::AmountToCaptureUpdate {
                status,
                amount_capturable,
                updated_by,
            },
            DieselPaymentAttemptUpdate::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                updated_by,
            } => Self::ConnectorResponse {
                authentication_data,
                encoded_data,
                connector_transaction_id,
                connector,
                updated_by,
            },
            DieselPaymentAttemptUpdate::IncrementalAuthorizationAmountUpdate {
                amount,
                amount_capturable,
            } => Self::IncrementalAuthorizationAmountUpdate {
                amount,
                amount_capturable,
            },
        }
    }
}

#[inline]
/// Adds a new connector transaction ID to the reverse lookup in the key-value store. This method takes the key-value store, the key to update, the merchant ID, the updated attempt ID, the connector transaction ID, and the storage scheme as input parameters. It creates a new reverse lookup entry with the provided information and inserts it into the key-value store using the specified storage scheme. Returns a CustomResult containing the updated ReverseLookup or an errors::StorageError if the operation fails.
async fn add_connector_txn_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    merchant_id: &str,
    updated_attempt_attempt_id: &str,
    connector_transaction_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> CustomResult<ReverseLookup, errors::StorageError> {
    let field = format!("pa_{}", updated_attempt_attempt_id);
    let reverse_lookup_new = ReverseLookupNew {
        lookup_id: format!("pa_conn_trans_{}_{}", merchant_id, connector_transaction_id),
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
/// Adds a preprocessing id to the reverse lookup for a given key and merchant id in the KVRouterStore.
///
/// # Arguments
///
/// * `store` - A reference to the KVRouterStore where the reverse lookup will be added.
/// * `key` - A reference to the key for the reverse lookup.
/// * `merchant_id` - A reference to the merchant id for the reverse lookup.
/// * `updated_attempt_attempt_id` - A reference to the updated attempt id for the reverse lookup.
/// * `preprocessing_id` - A reference to the preprocessing id that will be added to the reverse lookup.
/// * `storage_scheme` - The storage scheme for the reverse lookup.
///
/// # Returns
///
/// A CustomResult containing the ReverseLookup if successful, or a StorageError if an error occurs.
///
async fn add_preprocessing_id_to_reverse_lookup<T: DatabaseStore>(
    store: &KVRouterStore<T>,
    key: &str,
    merchant_id: &str,
    updated_attempt_attempt_id: &str,
    preprocessing_id: &str,
    storage_scheme: MerchantStorageScheme,
) -> CustomResult<ReverseLookup, errors::StorageError> {
    let field = format!("pa_{}", updated_attempt_attempt_id);
    let reverse_lookup_new = ReverseLookupNew {
        lookup_id: format!("pa_preprocessing_{}_{}", merchant_id, preprocessing_id),
        pk_id: key.to_owned(),
        sk_id: field.clone(),
        source: "payment_attempt".to_string(),
        updated_by: storage_scheme.to_string(),
    };
    store
        .insert_reverse_lookup(reverse_lookup_new, storage_scheme)
        .await
}
