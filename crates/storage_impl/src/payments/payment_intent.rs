use common_utils::date_time;
use common_utils::ext_traits::Encode;
use data_models::payments::payment_intent::{PaymentIntentInterface, PaymentIntentUpdate, PaymentIntent, PaymentIntentNew};
use data_models::payments::payment_attempt::{PaymentAttempt, PaymentAttemptNew};
use data_models::MerchantStorageScheme;
use data_models::errors::StorageError;
use diesel_models::kv;
use error_stack::{IntoReport, ResultExt};
use diesel_models::payment_intent::{PaymentIntent as DieselPaymentIntent, PaymentIntentNew as DieselPaymentIntentNew, PaymentIntentUpdate as DieselPaymentIntentUpdate};
use redis_interface::HsetnxReply;
use crate::{DatabaseStore, KVRouterStore, DataModelExt};
use crate::{redis::kv_store::RedisConnInterface, utils::{pg_connection_write, pg_connection_read}};

// #[async_trait::async_trait]
// impl<T: DatabaseStore> PaymentIntentInterface for KVRouterStore<T> {
//     async fn insert_payment_intent(
//         &self,
//         new: PaymentIntentNew,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<PaymentIntent, StorageError> {
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => {
//                 let conn = pg_connection_write(self).await?;
//                 new.insert(&conn).await.map_err(Into::into).into_report()
//             }

//             MerchantStorageScheme::RedisKv => {
//                 let key = format!("{}_{}", new.merchant_id, new.payment_id);
//                 let created_intent = PaymentIntent {
//                     id: 0i32,
//                     payment_id: new.payment_id.clone(),
//                     merchant_id: new.merchant_id.clone(),
//                     status: new.status,
//                     amount: new.amount,
//                     currency: new.currency,
//                     amount_captured: new.amount_captured,
//                     customer_id: new.customer_id.clone(),
//                     description: new.description.clone(),
//                     return_url: new.return_url.clone(),
//                     metadata: new.metadata.clone(),
//                     connector_id: new.connector_id.clone(),
//                     shipping_address_id: new.shipping_address_id.clone(),
//                     billing_address_id: new.billing_address_id.clone(),
//                     statement_descriptor_name: new.statement_descriptor_name.clone(),
//                     statement_descriptor_suffix: new.statement_descriptor_suffix.clone(),
//                     created_at: new.created_at.unwrap_or_else(date_time::now),
//                     modified_at: new.created_at.unwrap_or_else(date_time::now),
//                     last_synced: new.last_synced,
//                     setup_future_usage: new.setup_future_usage,
//                     off_session: new.off_session,
//                     client_secret: new.client_secret.clone(),
//                     business_country: new.business_country,
//                     business_label: new.business_label.clone(),
//                     active_attempt_id: new.active_attempt_id.to_owned(),
//                     order_details: new.order_details.clone(),
//                     allowed_payment_method_types: new.allowed_payment_method_types.clone(),
//                     connector_metadata: new.connector_metadata.clone(),
//                     feature_metadata: new.feature_metadata.clone(),
//                     attempt_count: new.attempt_count,
//                 };

//                 match self
//                     .get_redis_conn()
//                     StorageError>::into)?
//                     .serialize_and_set_hash_field_if_not_exist(&key, "pi", &created_intent)
//                     .await
//                 {
//                     Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
//                         entity: "payment_intent",
//                         key: Some(key),
//                     })
//                     .into_report(),
//                     Ok(HsetnxReply::KeySet) => {
//                         let redis_entry = kv::TypedSql {
//                             op: kv::DBOperation::Insert {
//                                 insertable: kv::Insertable::PaymentIntent(new),
//                             },
//                         };
//                         self.push_to_drainer_stream::<PaymentIntent>(
//                             redis_entry,
//                             storage_partitioning::PartitionKey::MerchantIdPaymentId {
//                                 merchant_id: &created_intent.merchant_id,
//                                 payment_id: &created_intent.payment_id,
//                             },
//                         )
//                         .await
//                         .change_context(errors::StorageError::KVError)?;
//                         Ok(created_intent)
//                     }
//                     Err(error) => Err(error.change_context(errors::StorageError::KVError)),
//                 }
//             }
//         }
//     }

//     async fn update_payment_intent(
//         &self,
//         this: PaymentIntent,
//         payment_intent: PaymentIntentUpdate,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<PaymentIntent, StorageError> {
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => {
//                 let conn = pg_connection_write(self).await?;
//                 this.update(&conn, payment_intent)
//                     .await
//                     .map_err(Into::into)
//                     .into_report()
//             }

//             MerchantStorageScheme::RedisKv => {
//                 let key = format!("{}_{}", this.merchant_id, this.payment_id);

//                 let updated_intent = payment_intent.clone().apply_changeset(this.clone());
//                 // Check for database presence as well Maybe use a read replica here ?

//                 let redis_value =
//                     Encode::<PaymentIntent>::encode_to_string_of_json(&updated_intent)
//                         .change_context(errors::StorageError::SerializationFailed)?;

//                 let updated_intent = self
//                     .get_redis_conn()
//                     StorageError>::into)?
//                     .set_hash_fields(&key, ("pi", &redis_value))
//                     .await
//                     .map(|_| updated_intent)
//                     .change_context(errors::StorageError::KVError)?;

//                 let redis_entry = kv::TypedSql {
//                     op: kv::DBOperation::Update {
//                         updatable: kv::Updateable::PaymentIntentUpdate(
//                             kv::PaymentIntentUpdateMems {
//                                 orig: this,
//                                 update_data: payment_intent,
//                             },
//                         ),
//                     },
//                 };

//                 self.push_to_drainer_stream::<PaymentIntent>(
//                     redis_entry,
//                     storage_partitioning::PartitionKey::MerchantIdPaymentId {
//                         merchant_id: &updated_intent.merchant_id,
//                         payment_id: &updated_intent.payment_id,
//                     },
//                 )
//                 .await
//                 .change_context(errors::StorageError::KVError)?;
//                 Ok(updated_intent)
//             }
//         }
//     }

//     async fn find_payment_intent_by_payment_id_merchant_id(
//         &self,
//         payment_id: &str,
//         merchant_id: &str,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<PaymentIntent, StorageError> {
//         let database_call = || async {
//             let conn = pg_connection_read(self).await?;
//             PaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
//                 .await
//                 .map_err(Into::into)
//                 .into_report()
//         };
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => database_call().await,

//             MerchantStorageScheme::RedisKv => {
//                 let key = format!("{merchant_id}_{payment_id}");
//                 crate::utils::try_redis_get_else_try_database_get(
//                     self.get_redis_conn()
//                         StorageError>::into)?
//                         .get_hash_field_and_deserialize(&key, "pi", "PaymentIntent"),
//                     database_call,
//                 )
//                 .await
//             }
//         }
//     }

//     #[cfg(feature = "olap")]
//     async fn filter_payment_intent_by_constraints(
//         &self,
//         merchant_id: &str,
//         pc: &api::PaymentListConstraints,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => {
//                 let conn = pg_connection_read(self).await?;
//                 PaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
//                     .await
//                     .map_err(Into::into)
//                     .into_report()
//             }

//             MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
//         }
//     }
//     #[cfg(feature = "olap")]
//     async fn filter_payment_intents_by_time_range_constraints(
//         &self,
//         merchant_id: &str,
//         time_range: &api::TimeRange,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => {
//                 let conn = pg_connection_read(self).await?;
//                 PaymentIntent::filter_by_time_constraints(&conn, merchant_id, time_range)
//                     .await
//                     .map_err(Into::into)
//                     .into_report()
//             }

//             MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
//         }
//     }

//     #[cfg(feature = "olap")]
//     async fn apply_filters_on_payments_list(
//         &self,
//         merchant_id: &str,
//         constraints: &api::PaymentListFilterConstraints,
//         storage_scheme: MerchantStorageScheme,
//     ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
//         match storage_scheme {
//             MerchantStorageScheme::PostgresOnly => {
//                 let conn = pg_connection_read(self).await?;
//                 PaymentIntent::apply_filters_on_payments(&conn, merchant_id, constraints)
//                     .await
//                     .map_err(Into::into)
//                     .into_report()
//             }

//             MerchantStorageScheme::RedisKv => Err(errors::StorageError::KVError.into()),
//         }
//     }
// }

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentIntentInterface for crate::RouterStore<T> 
where T::Config: Send
{
    async fn insert_payment_intent(
        &self,
        new: PaymentIntentNew,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        new.to_storage_model().insert(&conn).await.change_context(StorageError::TemporaryError).map(PaymentIntent::from_storage_model)
    }

    async fn update_payment_intent(
        &self,
        this: PaymentIntent,
        payment_intent: PaymentIntentUpdate,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let conn = pg_connection_write(self).await?;
        this.to_storage_model().update(&conn, payment_intent.to_storage_model())
            .await
            .change_context(StorageError::TemporaryError)
            .map(PaymentIntent::from_storage_model)
    }

    async fn find_payment_intent_by_payment_id_merchant_id(
        &self,
        _payment_id: &str,
        merchant_id: &str,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<PaymentIntent, StorageError> {
        let _conn = pg_connection_read(self).await?;
        // DieselPaymentIntent::find_by_payment_id_merchant_id(&conn, payment_id, merchant_id)
        //     .await
        //     .map_err(Into::into)
        //     .into_report()
        todo!("Create a common filter api")
    }

    #[cfg(feature = "olap")]
    async fn filter_payment_intent_by_constraints(
        &self,
        _merchant_id: &str,
        _pc: &api_models::payments::PaymentListConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        let _conn = pg_connection_read(self).await?;
        // DieselPaymentIntent::filter_by_constraints(&conn, merchant_id, pc)
        //     .await
        //     .map_err(Into::into)
        //     .into_report()
        todo!("Create a common filter api")
    }
    #[cfg(feature = "olap")]
    async fn filter_payment_intents_by_time_range_constraints(
        &self,
        merchant_id: &str,
        _time_range: &api_models::payments::TimeRange,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<PaymentIntent>, StorageError> {
        let _conn = pg_connection_read(self).await?;
        // DieselPaymentIntent::filter_by_time_constraints(&conn, merchant_id, time_range)
        //     .await
        //     .map_err(Into::into)
        //     .into_report()
        todo!("Create a common filter api")
    }

    #[cfg(feature = "olap")]
    async fn apply_filters_on_payments_list(
        &self,
        _merchant_id: &str,
        _constraints: &api_models::payments::PaymentListFilterConstraints,
        _storage_scheme: MerchantStorageScheme,
    ) -> error_stack::Result<Vec<(PaymentIntent, PaymentAttempt)>, StorageError> {
        let _conn = pg_connection_read(self).await?;
        // DieselPaymentIntent::apply_filters_on_payments(&conn, merchant_id, constraints)
        //     .await
        //     .map_err(Into::into)
        //     .into_report()
        todo!("Create a common filter api")
    }
}

impl DataModelExt for PaymentIntentNew {
    type StorageModel = DieselPaymentIntentNew;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentIntentNew {
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt_id,
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            amount_captured: storage_model.amount_captured,
            customer_id: storage_model.customer_id,
            description: storage_model.description,
            return_url: storage_model.return_url,
            metadata: storage_model.metadata,
            connector_id: storage_model.connector_id,
            shipping_address_id: storage_model.shipping_address_id,
            billing_address_id: storage_model.billing_address_id,
            statement_descriptor_name: storage_model.statement_descriptor_name,
            statement_descriptor_suffix: storage_model.statement_descriptor_suffix,
            created_at: storage_model.created_at,
            modified_at: storage_model.modified_at,
            last_synced: storage_model.last_synced,
            setup_future_usage: storage_model.setup_future_usage,
            off_session: storage_model.off_session,
            client_secret: storage_model.client_secret,
            active_attempt_id: storage_model.active_attempt_id,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            order_details: storage_model.order_details,
            allowed_payment_method_types: storage_model.allowed_payment_method_types,
            connector_metadata: storage_model.connector_metadata,
            feature_metadata: storage_model.feature_metadata,
            attempt_count: storage_model.attempt_count,
        }
    }

    
}

impl DataModelExt for PaymentIntent {
    type StorageModel = DieselPaymentIntent;

    fn to_storage_model(self) -> Self::StorageModel {
        DieselPaymentIntent {
            id: self.id,
            payment_id: self.payment_id,
            merchant_id: self.merchant_id,
            status: self.status,
            amount: self.amount,
            currency: self.currency,
            amount_captured: self.amount_captured,
            customer_id: self.customer_id,
            description: self.description,
            return_url: self.return_url,
            metadata: self.metadata,
            connector_id: self.connector_id,
            shipping_address_id: self.shipping_address_id,
            billing_address_id: self.billing_address_id,
            statement_descriptor_name: self.statement_descriptor_name,
            statement_descriptor_suffix: self.statement_descriptor_suffix,
            created_at: self.created_at,
            modified_at: self.modified_at,
            last_synced: self.last_synced,
            setup_future_usage: self.setup_future_usage,
            off_session: self.off_session,
            client_secret: self.client_secret,
            active_attempt_id: self.active_attempt_id,
            business_country: self.business_country,
            business_label: self.business_label,
            order_details: self.order_details,
            allowed_payment_method_types: self.allowed_payment_method_types,
            connector_metadata: self.connector_metadata,
            feature_metadata: self.feature_metadata,
            attempt_count: self.attempt_count,
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        Self {
            id: storage_model.id,
            payment_id: storage_model.payment_id,
            merchant_id: storage_model.merchant_id,
            status: storage_model.status,
            amount: storage_model.amount,
            currency: storage_model.currency,
            amount_captured: storage_model.amount_captured,
            customer_id: storage_model.customer_id,
            description: storage_model.description,
            return_url: storage_model.return_url,
            metadata: storage_model.metadata,
            connector_id: storage_model.connector_id,
            shipping_address_id: storage_model.shipping_address_id,
            billing_address_id: storage_model.billing_address_id,
            statement_descriptor_name: storage_model.statement_descriptor_name,
            statement_descriptor_suffix: storage_model.statement_descriptor_suffix,
            created_at: storage_model.created_at,
            modified_at: storage_model.modified_at,
            last_synced: storage_model.last_synced,
            setup_future_usage: storage_model.setup_future_usage,
            off_session: storage_model.off_session,
            client_secret: storage_model.client_secret,
            active_attempt_id: storage_model.active_attempt_id,
            business_country: storage_model.business_country,
            business_label: storage_model.business_label,
            order_details: storage_model.order_details,
            allowed_payment_method_types: storage_model.allowed_payment_method_types,
            connector_metadata: storage_model.connector_metadata,
            feature_metadata: storage_model.feature_metadata,
            attempt_count: storage_model.attempt_count,
        }
    }

    
    
}

impl DataModelExt for PaymentIntentUpdate {
    type StorageModel = DieselPaymentIntentUpdate;

    fn to_storage_model(self) -> Self::StorageModel {
        match self {
            Self::ResponseUpdate { status, amount_captured, return_url } => DieselPaymentIntentUpdate::ResponseUpdate { status, amount_captured, return_url },
            Self::MetadataUpdate { metadata } => DieselPaymentIntentUpdate::MetadataUpdate { metadata },
            Self::ReturnUrlUpdate { return_url, status, customer_id, shipping_address_id, billing_address_id } => DieselPaymentIntentUpdate::ReturnUrlUpdate { return_url, status, customer_id, shipping_address_id, billing_address_id },
            Self::MerchantStatusUpdate { status, shipping_address_id, billing_address_id } => DieselPaymentIntentUpdate::MerchantStatusUpdate { status, shipping_address_id, billing_address_id },
            Self::PGStatusUpdate { status } => DieselPaymentIntentUpdate::PGStatusUpdate { status },
            Self::Update { amount, currency, setup_future_usage, status, customer_id, shipping_address_id, billing_address_id, return_url, business_country, business_label, description, statement_descriptor_name, statement_descriptor_suffix, order_details, metadata } => DieselPaymentIntentUpdate::Update { amount, currency, setup_future_usage, status, customer_id, shipping_address_id, billing_address_id, return_url, business_country, business_label, description, statement_descriptor_name, statement_descriptor_suffix, order_details, metadata },
            Self::PaymentAttemptAndAttemptCountUpdate { active_attempt_id, attempt_count } => DieselPaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate { active_attempt_id, attempt_count },
            Self::StatusAndAttemptUpdate { status, active_attempt_id, attempt_count } => DieselPaymentIntentUpdate::StatusAndAttemptUpdate { status, active_attempt_id, attempt_count },
        }
    }

    fn from_storage_model(storage_model: Self::StorageModel) -> Self {
        match storage_model {
            DieselPaymentIntentUpdate::ResponseUpdate { status, amount_captured, return_url } => Self::ResponseUpdate { status, amount_captured, return_url },
            DieselPaymentIntentUpdate::MetadataUpdate { metadata } => Self::MetadataUpdate { metadata },
            DieselPaymentIntentUpdate::ReturnUrlUpdate { return_url, status, customer_id, shipping_address_id, billing_address_id } => Self::ReturnUrlUpdate { return_url, status, customer_id, shipping_address_id, billing_address_id },
            DieselPaymentIntentUpdate::MerchantStatusUpdate { status, shipping_address_id, billing_address_id } => Self::MerchantStatusUpdate { status, shipping_address_id, billing_address_id },
            DieselPaymentIntentUpdate::PGStatusUpdate { status } => Self::PGStatusUpdate { status },
            DieselPaymentIntentUpdate::Update { amount, currency, setup_future_usage, status, customer_id, shipping_address_id, billing_address_id, return_url, business_country, business_label, description, statement_descriptor_name, statement_descriptor_suffix, order_details, metadata } => Self::Update { amount, currency, setup_future_usage, status, customer_id, shipping_address_id, billing_address_id, return_url, business_country, business_label, description, statement_descriptor_name, statement_descriptor_suffix, order_details, metadata },
            DieselPaymentIntentUpdate::PaymentAttemptAndAttemptCountUpdate { active_attempt_id, attempt_count } => Self::PaymentAttemptAndAttemptCountUpdate { active_attempt_id, attempt_count },
            DieselPaymentIntentUpdate::StatusAndAttemptUpdate { status, active_attempt_id, attempt_count } => Self::StatusAndAttemptUpdate { status, active_attempt_id, attempt_count },
        }
    }
    
}