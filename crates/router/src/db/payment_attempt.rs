use crate::{
    core::errors::{self, CustomResult},
    types::storage::{PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate},
};

#[async_trait::async_trait]
pub trait IPaymentAttempt {
    async fn insert_payment_attempt(
        &self,
        payment_attempt: PaymentAttemptNew,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn update_payment_attempt(
        &self,
        this: PaymentAttempt,
        payment_attempt: PaymentAttemptUpdate,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_transaction_id_payment_id_merchant_id(
        &self,
        transaction_id: &str,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
        &self,
        payment_id: &str,
        merchant_id: &str,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_connector_txn_id(
        &self,
        merchant_id: &str,
        connector_txn_id: &str,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;

    async fn find_payment_attempt_by_merchant_id_txn_id(
        &self,
        merchant_id: &str,
        txn_id: &str,
    ) -> CustomResult<PaymentAttempt, errors::StorageError>;
}

#[cfg(not(feature = "kv_store"))]
mod storage {
    use super::IPaymentAttempt;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::Store,
        types::storage::payment_attempt::*,
    };

    #[async_trait::async_trait]
    impl IPaymentAttempt for Store {
        async fn insert_payment_attempt(
            &self,
            payment_attempt: PaymentAttemptNew,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            payment_attempt.insert(&conn).await
        }

        async fn update_payment_attempt(
            &self,
            this: PaymentAttempt,
            payment_attempt: PaymentAttemptUpdate,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            this.update(&conn, payment_attempt).await
        }

        async fn find_payment_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            PaymentAttempt::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id).await
        }

        async fn find_payment_attempt_by_transaction_id_payment_id_merchant_id(
            &self,
            transaction_id: &str,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            PaymentAttempt::find_by_transaction_id_payment_id_merchant_id(
                &conn,
                transaction_id,
                payment_id,
                merchant_id,
            )
            .await
        }

        async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            PaymentAttempt::find_last_successful_attempt_by_payment_id_merchant_id(
                &conn,
                payment_id,
                merchant_id,
            )
            .await
        }

        async fn find_payment_attempt_by_merchant_id_connector_txn_id(
            &self,
            merchant_id: &str,
            connector_txn_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;
            // TODO: update logic to lookup all payment attempts for an intent
            // and apply filter logic on top of them to get the desired one.
            PaymentAttempt::find_by_merchant_id_connector_txn_id(
                &conn,
                merchant_id,
                connector_txn_id,
            )
            .await
        }

        async fn find_payment_attempt_by_merchant_id_txn_id(
            &self,
            merchant_id: &str,
            txn_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let conn = pg_connection(&self.master_pool.conn).await;

            PaymentAttempt::find_by_merchant_id_transaction_id(&conn, merchant_id, txn_id).await
        }
    }
}

#[cfg(feature = "kv_store")]
mod storage {
    use error_stack::{IntoReport, ResultExt};
    use fred::prelude::*;

    use super::IPaymentAttempt;
    use crate::{
        connection::pg_connection,
        core::errors::{self, CustomResult},
        services::{redis::RedisEntryId, Store},
        types::storage::{enums, payment_attempt::*},
        utils::{date_time, storage_partitioning::KvStorePartition},
    };

    #[async_trait::async_trait]
    impl IPaymentAttempt for Store {
        async fn insert_payment_attempt(
            &self,
            payment_attempt: PaymentAttemptNew,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let key = format!(
                "{}_{}",
                payment_attempt.payment_id, payment_attempt.merchant_id
            );
            // TODO: need to add an application generated payment attempt id to distinguish between multiple attempts for the same payment id
            // Check for database presence as well Maybe use a read replica here ?
            let created_attempt = PaymentAttempt {
                id: 0i32,
                payment_id: payment_attempt.payment_id.clone(),
                merchant_id: payment_attempt.merchant_id.clone(),
                txn_id: payment_attempt.txn_id.clone(),
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
                payment_flow: payment_attempt.payment_flow,
                redirect: payment_attempt.redirect,
                connector_transaction_id: payment_attempt.connector_transaction_id.clone(),
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
            };
            // TODO: Add a proper error for serialization failure
            let redis_value = serde_json::to_string(&created_attempt)
                .into_report()
                .change_context(errors::StorageError::KVError)?;
            match self
                .redis_conn
                .pool
                .hsetnx::<u8, &str, &str, &str>(&key, "pa", &redis_value)
                .await
            {
                Ok(0) => Err(errors::StorageError::DuplicateValue(format!(
                    "Payment Attempt already exists for payment_id: {}",
                    key
                )))
                .into_report(),
                Ok(1) => {
                    let conn = pg_connection(&self.master_pool.conn).await;
                    let query = payment_attempt
                        .insert(&conn)
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    let stream_name = self.drainer_stream(&PaymentAttempt::shard_key(
                        crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                            merchant_id: &created_attempt.merchant_id,
                            payment_id: &created_attempt.payment_id,
                        },
                        self.config.drainer_num_partitions,
                    ));
                    self.redis_conn
                        .stream_append_entry(
                            &stream_name,
                            &RedisEntryId::AutoGeneratedID,
                            query.to_field_value_pairs(),
                        )
                        .await
                        .change_context(errors::StorageError::KVError)?;
                    Ok(created_attempt)
                }
                Ok(i) => Err(errors::StorageError::KVError)
                    .into_report()
                    .attach_printable_lazy(|| format!("Invalid response for HSETNX: {}", i)),
                Err(er) => Err(er)
                    .into_report()
                    .change_context(errors::StorageError::KVError),
            }
        }

        async fn update_payment_attempt(
            &self,
            this: PaymentAttempt,
            payment_attempt: PaymentAttemptUpdate,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let key = format!("{}_{}", this.payment_id, this.merchant_id);

            let updated_attempt = payment_attempt.clone().apply_changeset(this.clone());
            // Check for database presence as well Maybe use a read replica here ?
            // TODO: Add a proper error for serialization failure
            let redis_value = serde_json::to_string(&updated_attempt)
                .into_report()
                .change_context(errors::StorageError::KVError)?;
            let updated_attempt = self
                .redis_conn
                .pool
                .hset::<u8, &str, (&str, String)>(&key, ("pa", redis_value))
                .await
                .map(|_| updated_attempt)
                .into_report()
                .change_context(errors::StorageError::KVError)?;

            let conn = pg_connection(&self.master_pool.conn).await;
            let query = this
                .update(&conn, payment_attempt)
                .await
                .change_context(errors::StorageError::KVError)?;
            let stream_name = self.drainer_stream(&PaymentAttempt::shard_key(
                crate::utils::storage_partitioning::PartitionKey::MerchantIdPaymentId {
                    merchant_id: &updated_attempt.merchant_id,
                    payment_id: &updated_attempt.payment_id,
                },
                self.config.drainer_num_partitions,
            ));
            self.redis_conn
                .stream_append_entry(
                    &stream_name,
                    &RedisEntryId::AutoGeneratedID,
                    query.to_field_value_pairs(),
                )
                .await
                .change_context(errors::StorageError::KVError)?;
            Ok(updated_attempt)
        }

        async fn find_payment_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            let key = format!("{}_{}", payment_id, merchant_id);
            self.redis_conn
                .pool
                .hget::<String, String, &str>(key, "pa")
                .await
                .into_report()
                .change_context(errors::StorageError::KVError)
                .and_then(|redis_resp| {
                    serde_json::from_str::<PaymentAttempt>(&redis_resp)
                        .into_report()
                        .change_context(errors::StorageError::KVError)
                })
            // Check for database presence as well Maybe use a read replica here ?
        }

        async fn find_payment_attempt_by_transaction_id_payment_id_merchant_id(
            &self,
            transaction_id: &str,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            // We assume that PaymentAttempt <=> PaymentIntent is a one-to-one relation for now
            self.find_payment_attempt_by_payment_id_merchant_id(payment_id, merchant_id)
                .await
                .and_then(|attempt| {
                    if attempt.connector_transaction_id.as_deref() == Some(transaction_id) {
                        Ok(attempt)
                    } else {
                        Err(errors::StorageError::ValueNotFound(format!(
                            "Successful payment attempt does not exist for {}_{}",
                            payment_id, merchant_id
                        )))
                        .into_report()
                    }
                })
        }

        async fn find_payment_attempt_last_successful_attempt_by_payment_id_merchant_id(
            &self,
            payment_id: &str,
            merchant_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            self.find_payment_attempt_by_payment_id_merchant_id(payment_id, merchant_id)
                .await
                .and_then(|attempt| match attempt.status {
                    enums::AttemptStatus::Charged => Ok(attempt),
                    _ => Err(errors::StorageError::ValueNotFound(format!(
                        "Successful payment attempt does not exist for {}_{}",
                        payment_id, merchant_id
                    )))
                    .into_report(),
                })
        }

        async fn find_payment_attempt_by_merchant_id_connector_txn_id(
            &self,
            merchant_id: &str,
            connector_txn_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            Err(errors::StorageError::KVError).into_report()
        }

        async fn find_payment_attempt_by_merchant_id_txn_id(
            &self,
            merchant_id: &str,
            txn_id: &str,
        ) -> CustomResult<PaymentAttempt, errors::StorageError> {
            Err(errors::StorageError::KVError).into_report()
        }
    }
}
