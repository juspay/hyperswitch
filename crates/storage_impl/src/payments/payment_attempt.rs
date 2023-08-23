use common_utils::errors::CustomResult;
use data_models::{
    errors,
    mandates::{MandateAmountData, MandateDataType},
    payments::payment_attempt::{
        PaymentAttempt, PaymentAttemptInterface, PaymentAttemptNew, PaymentAttemptUpdate, PaymentListFilters
    },
    MerchantStorageScheme,
};
use diesel_models::{
    enums::{MandateAmountData as DieselMandateAmountData, MandateDataType as DieselMandateType},
    payment_attempt::{
        PaymentAttempt as DieselPaymentAttempt, PaymentAttemptNew as DieselPaymentAttemptNew,
        PaymentAttemptUpdate as DieselPaymentAttemptUpdate,
    },
};

use crate::{
    utils::{pg_connection_read, pg_connection_write},
    DataModelExt, DatabaseStore, RouterStore, KVRouterStore
};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for RouterStore<T> {
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
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

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
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }

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
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

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
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

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
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

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
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

    async fn get_filters_for_payments(
        &self,
        pi: &[data_models::payments::payment_intent::PaymentIntent],
        merchant_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<
        PaymentListFilters,
        errors::StorageError,
    > {
        let conn = pg_connection_read(self).await?;
        let intents = pi
            .iter()
            .cloned()
            .map(|pi| pi.to_storage_model())
            .collect::<Vec<diesel_models::payment_intent::PaymentIntent>>();
        DieselPaymentAttempt::get_filters_for_payments(&conn, intents.as_slice(), merchant_id)
            .await
            .map_err(|er| {
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(|(connector, currency, status, payment_method)| {
                PaymentListFilters {
                    connector,
                    currency,
                    status,
                    payment_method,
                }
            })
    }

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
            let new_err = crate::diesel_error_to_data_error(er.current_context());
            er.change_context(new_err)
        })
        .map(PaymentAttempt::from_storage_model)
    }

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
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(|a| {
                a.into_iter()
                    .map(PaymentAttempt::from_storage_model)
                    .collect()
            })
    }

    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        merchant_id: &str,
        attempt_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<PaymentAttempt, errors::StorageError> {
        let conn = pg_connection_read(self).await?;

        DieselPaymentAttempt::find_by_merchant_id_attempt_id(&conn, merchant_id, attempt_id)
            .await
            .map_err(|er| {
                let new_err = crate::diesel_error_to_data_error(er.current_context());
                er.change_context(new_err)
            })
            .map(PaymentAttempt::from_storage_model)
    }
}


#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentAttemptInterface for KVRouterStore<T> {
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError> {
        match storage_scheme {
            MerchantStorageScheme::PostgresOnly => self.router_store.insert_payment_attempt(payment_attempt, storage_scheme).await,
            MerchantStorageScheme::RedisKv => {
                let key = format!(
                    "{}_{}",
                    payment_attempt.merchant_id, payment_attempt.payment_id
                );

                let created_attempt = PaymentAttempt {
                    id: Default::default(),
                    payment_id: payment_attempt.payment_id.clone(),
                    merchant_id: payment_attempt.merchant_id.clone(),
                    attempt_id: payment_attempt.attempt_id.clone(),
                    status: payment_attempt.status,
                    amount: payment_attempt.amount,
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
                    created_at: payment_attempt.created_at.unwrap_or_else(date_time::now),
                    modified_at: payment_attempt.created_at.unwrap_or_else(date_time::now),
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
                    straight_through_algorithm: payment_attempt
                        .straight_through_algorithm
                        .clone(),
                    mandate_details: payment_attempt.mandate_details.clone(),
                    preprocessing_step_id: payment_attempt.preprocessing_step_id.clone(),
                    error_reason: payment_attempt.error_reason.clone(),
                    multiple_capture_count: payment_attempt.multiple_capture_count,
                    connector_response_reference_id: None,
                };

                let field = format!("pa_{}", created_attempt.attempt_id);
                match self
                    .get_redis_conn()
                    .map_err(Into::<errors::StorageError>::into)?
                    .serialize_and_set_hash_field_if_not_exist(&key, &field, &created_attempt)
                    .await
                {
                    Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
                        entity: "payment attempt",
                        key: Some(key),
                    })
                    .into_report(),
                    Ok(HsetnxReply::KeySet) => {
                        let conn = connection::pg_connection_write(self).await?;

                        //Reverse lookup for attempt_id
                        ReverseLookupNew {
                            lookup_id: format!(
                                "{}_{}",
                                &created_attempt.merchant_id, &created_attempt.attempt_id,
                            ),
                            pk_id: key,
                            sk_id: field,
                            source: "payment_attempt".to_string(),
                        }
                        .insert(&conn)
                        .await
                        .map_err(Into::<errors::StorageError>::into)
                        .into_report()?;

                        let redis_entry = kv::TypedSql {
                            op: kv::DBOperation::Insert {
                                insertable: kv::Insertable::PaymentAttempt(payment_attempt),
                            },
                        };
                        self.push_to_drainer_stream::<PaymentAttempt>(
                            redis_entry,
                            crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                                merchant_id: &created_attempt.merchant_id,
                                payment_id: &created_attempt.payment_id,
                            }
                        )
                        .await.change_context(errors::StorageError::KVError)?;
                        Ok(created_attempt)
                    }
                    Err(error) => Err(error.change_context(errors::StorageError::KVError)),
                }
            }
        }
    }

    async fn update_payment_attempt_with_attempt_id(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_connector_transaction_id_payment_id_merchant_id(
        &self,
        connector_transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
        connector_txn_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_payment_id_merchant_id_attempt_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
        attempt_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_attempt_id_merchant_id(
        &self,
        attempt_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_preprocessing_id_merchant_id(
        &self,
        preprocessing_id: &str,
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentAttempt, errors::StorageError>;

    async fn find_attempts_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentAttempt>, errors::StorageError>;

    async fn get_filters_for_payments(
        &self,
        pi: &[PaymentIntent],
        merchant_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentListFilters, errors::StorageError>;
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

impl DataModelExt for PaymentAttempt {
    type StorageModel = DieselPaymentAttempt;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttempt {
            id: self.id,
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
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
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
                .map(MandateDataType::from_storage_model),
            error_reason: storage_model.error_reason,
            multiple_capture_count: storage_model.multiple_capture_count,
            connector_response_reference_id: storage_model.connector_response_reference_id,
        }
    }
}

impl DataModelExt for PaymentAttemptNew {
    type StorageModel = DieselPaymentAttemptNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentAttemptNew {
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
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
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
                .map(MandateDataType::from_storage_model),
            error_reason: storage_model.error_reason,
            connector_response_reference_id: storage_model.connector_response_reference_id,
            multiple_capture_count: storage_model.multiple_capture_count,
        }
    }
}

impl DataModelExt for PaymentAttemptUpdate {
    type StorageModel = DieselPaymentAttemptUpdate;

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
            },
            Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
            } => DieselPaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
            },
            Self::AuthenticationTypeUpdate {
                authentication_type,
            } => DieselPaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
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
            },
            Self::VoidUpdate {
                status,
                cancellation_reason,
            } => DieselPaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
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
            } => DieselPaymentAttemptUpdate::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
            },
            Self::StatusUpdate { status } => DieselPaymentAttemptUpdate::StatusUpdate { status },
            Self::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
            } => DieselPaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
            },
            Self::MultipleCaptureUpdate {
                status,
                multiple_capture_count,
            } => DieselPaymentAttemptUpdate::MultipleCaptureUpdate {
                status,
                multiple_capture_count,
            },
            Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
            } => DieselPaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
            },
        }
    }

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
            },
            DieselPaymentAttemptUpdate::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
            } => Self::UpdateTrackers {
                payment_token,
                connector,
                straight_through_algorithm,
            },
            DieselPaymentAttemptUpdate::AuthenticationTypeUpdate {
                authentication_type,
            } => Self::AuthenticationTypeUpdate {
                authentication_type,
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
            },
            DieselPaymentAttemptUpdate::VoidUpdate {
                status,
                cancellation_reason,
            } => Self::VoidUpdate {
                status,
                cancellation_reason,
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
            } => Self::UnresolvedResponseUpdate {
                status,
                connector,
                connector_transaction_id,
                payment_method_id,
                error_code,
                error_message,
                error_reason,
                connector_response_reference_id,
            },
            DieselPaymentAttemptUpdate::StatusUpdate { status } => Self::StatusUpdate { status },
            DieselPaymentAttemptUpdate::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
            } => Self::ErrorUpdate {
                connector,
                status,
                error_code,
                error_message,
                error_reason,
            },
            DieselPaymentAttemptUpdate::MultipleCaptureUpdate {
                status,
                multiple_capture_count,
            } => Self::MultipleCaptureUpdate {
                status,
                multiple_capture_count,
            },
            DieselPaymentAttemptUpdate::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
            } => Self::PreprocessingUpdate {
                status,
                payment_method_id,
                connector_metadata,
                preprocessing_step_id,
                connector_transaction_id,
                connector_response_reference_id,
            },
        }
    }
}
