use common_utils::id_type;

// use super::MockDb;
// use crate::{
//     core::errors::{self, CustomResult},
//     types::storage::{self as storage_types, enums::MerchantStorageScheme},
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::mandate as storage_types;
use diesel_models::enums::MerchantStorageScheme;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait MandateInterface {
    type Error;
    async fn find_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, Self::Error>;

    async fn find_mandate_by_merchant_id_connector_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        connector_mandate_id: &str,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, Self::Error>;

    async fn find_mandate_by_merchant_id_customer_id(
        &self,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<Vec<storage_types::Mandate>, Self::Error>;

    // Fix this function once we move to mandate v2
    #[cfg(all(feature = "v2", feature = "customer_v2"))]
    async fn find_mandate_by_global_customer_id(
        &self,
        id: &id_type::GlobalCustomerId,
    ) -> CustomResult<Vec<storage_types::Mandate>, Self::Error>;

    async fn update_mandate_by_merchant_id_mandate_id(
        &self,
        merchant_id: &id_type::MerchantId,
        mandate_id: &str,
        mandate_update: storage_types::MandateUpdate,
        mandate: storage_types::Mandate,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, Self::Error>;

    // (TODO:jarnura)Fix this api models used in this function
    // async fn find_mandates_by_merchant_id(
    //     &self,
    //     merchant_id: &id_type::MerchantId,
    //     mandate_constraints: api_models::mandates::MandateListConstraints,
    // ) -> CustomResult<Vec<storage_types::Mandate>, Self::Error>;

    async fn insert_mandate(
        &self,
        mandate: storage_types::MandateNew,
        storage_scheme: MerchantStorageScheme,
    ) -> CustomResult<storage_types::Mandate, Self::Error>;
}

// #[cfg(feature = "kv_store")]
// mod storage {
//     use common_utils::{fallback_reverse_lookup_not_found, id_type};
//     use diesel_models::kv;
//     use error_stack::{report, ResultExt};
//     use redis_interface::HsetnxReply;
//     use router_env::{instrument, tracing};
//     use storage_impl::redis::kv_store::{
//         decide_storage_scheme, kv_wrapper, KvOperation, Op, PartitionKey,
//     };

//     use super::MandateInterface;
//     use crate::{
//         connection,
//         core::errors::{self, utils::RedisErrorExt, CustomResult},
//         db::reverse_lookup::ReverseLookupInterface,
//         services::Store,
//         types::storage::{self as storage_types, enums::MerchantStorageScheme, MandateDbExt},
//         utils::db_utils,
//     };

//     #[async_trait::async_trait]
//     impl MandateInterface for Store {
//         #[instrument(skip_all)]
//         async fn find_mandate_by_merchant_id_mandate_id(
//             &self,
//             merchant_id: &id_type::MerchantId,
//             mandate_id: &str,
//             storage_scheme: MerchantStorageScheme,
//         ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
//             let conn = connection::pg_connection_read(self).await?;
//             let database_call = || async {
//                 storage_types::Mandate::find_by_merchant_id_mandate_id(
//                     &conn,
//                     merchant_id,
//                     mandate_id,
//                 )
//                 .await
//                 .map_err(|error| report!(errors::StorageError::from(error)))
//             };
//             let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Mandate>(
//                 self,
//                 storage_scheme,
//                 Op::Find,
//             ))
//             .await;
//             match storage_scheme {
//                 MerchantStorageScheme::PostgresOnly => database_call().await,
//                 MerchantStorageScheme::RedisKv => {
//                     let key = PartitionKey::MerchantIdMandateId {
//                         merchant_id,
//                         mandate_id,
//                     };
//                     let field = format!("mandate_{}", mandate_id);

//                     Box::pin(db_utils::try_redis_get_else_try_database_get(
//                         async {
//                             Box::pin(kv_wrapper(
//                                 self,
//                                 KvOperation::<diesel_models::Mandate>::HGet(&field),
//                                 key,
//                             ))
//                             .await?
//                             .try_into_hget()
//                         },
//                         database_call,
//                     ))
//                     .await
//                 }
//             }
//         }

//         #[instrument(skip_all)]
//         async fn find_mandate_by_merchant_id_connector_mandate_id(
//             &self,
//             merchant_id: &id_type::MerchantId,
//             connector_mandate_id: &str,
//             storage_scheme: MerchantStorageScheme,
//         ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
//             let conn = connection::pg_connection_read(self).await?;
//             let database_call = || async {
//                 storage_types::Mandate::find_by_merchant_id_connector_mandate_id(
//                     &conn,
//                     merchant_id,
//                     connector_mandate_id,
//                 )
//                 .await
//                 .map_err(|error| report!(errors::StorageError::from(error)))
//             };
//             let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Mandate>(
//                 self,
//                 storage_scheme,
//                 Op::Find,
//             ))
//             .await;
//             match storage_scheme {
//                 MerchantStorageScheme::PostgresOnly => database_call().await,
//                 MerchantStorageScheme::RedisKv => {
//                     let lookup_id = format!(
//                         "mid_{}_conn_mandate_{}",
//                         merchant_id.get_string_repr(),
//                         connector_mandate_id
//                     );
//                     let lookup = fallback_reverse_lookup_not_found!(
//                         self.get_lookup_by_lookup_id(&lookup_id, storage_scheme)
//                             .await,
//                         database_call().await
//                     );

//                     let key = PartitionKey::CombinationKey {
//                         combination: &lookup.pk_id,
//                     };

//                     Box::pin(db_utils::try_redis_get_else_try_database_get(
//                         async {
//                             Box::pin(kv_wrapper(
//                                 self,
//                                 KvOperation::<diesel_models::Mandate>::HGet(&lookup.sk_id),
//                                 key,
//                             ))
//                             .await?
//                             .try_into_hget()
//                         },
//                         database_call,
//                     ))
//                     .await
//                 }
//             }
//         }

//         #[instrument(skip_all)]
//         async fn find_mandate_by_merchant_id_customer_id(
//             &self,
//             merchant_id: &id_type::MerchantId,
//             customer_id: &id_type::CustomerId,
//         ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
//             let conn = connection::pg_connection_read(self).await?;
//             storage_types::Mandate::find_by_merchant_id_customer_id(&conn, merchant_id, customer_id)
//                 .await
//                 .map_err(|error| report!(errors::StorageError::from(error)))
//         }

//         #[cfg(all(feature = "v2", feature = "customer_v2"))]
//         #[instrument(skip_all)]
//         async fn find_mandate_by_global_customer_id(
//             &self,
//             id: &id_type::GlobalCustomerId,
//         ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
//             let conn = connection::pg_connection_read(self).await?;
//             storage_types::Mandate::find_by_global_customer_id(&conn, id)
//                 .await
//                 .map_err(|error| report!(errors::StorageError::from(error)))
//         }

//         #[instrument(skip_all)]
//         async fn update_mandate_by_merchant_id_mandate_id(
//             &self,
//             merchant_id: &id_type::MerchantId,
//             mandate_id: &str,
//             mandate_update: storage_types::MandateUpdate,
//             mandate: storage_types::Mandate,
//             storage_scheme: MerchantStorageScheme,
//         ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
//             let conn = connection::pg_connection_write(self).await?;
//             let key = PartitionKey::MerchantIdMandateId {
//                 merchant_id,
//                 mandate_id,
//             };
//             let field = format!("mandate_{}", mandate_id);
//             let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Mandate>(
//                 self,
//                 storage_scheme,
//                 Op::Update(key.clone(), &field, mandate.updated_by.as_deref()),
//             ))
//             .await;
//             match storage_scheme {
//                 MerchantStorageScheme::PostgresOnly => {
//                     storage_types::Mandate::update_by_merchant_id_mandate_id(
//                         &conn,
//                         merchant_id,
//                         mandate_id,
//                         mandate_update.convert_to_mandate_update(storage_scheme),
//                     )
//                     .await
//                     .map_err(|error| report!(errors::StorageError::from(error)))
//                 }
//                 MerchantStorageScheme::RedisKv => {
//                     let key_str = key.to_string();

//                     if let diesel_models::MandateUpdate::ConnectorMandateIdUpdate {
//                         connector_mandate_id: Some(val),
//                         ..
//                     } = &mandate_update
//                     {
//                         let rev_lookup = diesel_models::ReverseLookupNew {
//                             sk_id: field.clone(),
//                             pk_id: key_str.clone(),
//                             lookup_id: format!(
//                                 "mid_{}_conn_mandate_{}",
//                                 merchant_id.get_string_repr(),
//                                 val
//                             ),
//                             source: "mandate".to_string(),
//                             updated_by: storage_scheme.to_string(),
//                         };
//                         self.insert_reverse_lookup(rev_lookup, storage_scheme)
//                             .await?;
//                     }

//                     let m_update = mandate_update.convert_to_mandate_update(storage_scheme);
//                     let updated_mandate = m_update.clone().apply_changeset(mandate.clone());

//                     let redis_value = serde_json::to_string(&updated_mandate)
//                         .change_context(errors::StorageError::SerializationFailed)?;

//                     let redis_entry = kv::TypedSql {
//                         op: kv::DBOperation::Update {
//                             updatable: Box::new(kv::Updateable::MandateUpdate(
//                                 kv::MandateUpdateMems {
//                                     orig: mandate,
//                                     update_data: m_update,
//                                 },
//                             )),
//                         },
//                     };

//                     Box::pin(kv_wrapper::<(), _, _>(
//                         self,
//                         KvOperation::<diesel_models::Mandate>::Hset(
//                             (&field, redis_value),
//                             redis_entry,
//                         ),
//                         key,
//                     ))
//                     .await
//                     .map_err(|err| err.to_redis_failed_response(&key_str))?
//                     .try_into_hset()
//                     .change_context(errors::StorageError::KVError)?;

//                     Ok(updated_mandate)
//                 }
//             }
//         }

//         #[instrument(skip_all)]
//         async fn find_mandates_by_merchant_id(
//             &self,
//             merchant_id: &id_type::MerchantId,
//             mandate_constraints: api_models::mandates::MandateListConstraints,
//         ) -> CustomResult<Vec<storage_types::Mandate>, errors::StorageError> {
//             let conn = connection::pg_connection_read(self).await?;
//             storage_types::Mandate::filter_by_constraints(&conn, merchant_id, mandate_constraints)
//                 .await
//                 .map_err(|error| report!(errors::StorageError::from(error)))
//         }

//         #[instrument(skip_all)]
//         async fn insert_mandate(
//             &self,
//             mut mandate: storage_types::MandateNew,
//             storage_scheme: MerchantStorageScheme,
//         ) -> CustomResult<storage_types::Mandate, errors::StorageError> {
//             let conn = connection::pg_connection_write(self).await?;
//             let storage_scheme = Box::pin(decide_storage_scheme::<_, diesel_models::Mandate>(
//                 self,
//                 storage_scheme,
//                 Op::Insert,
//             ))
//             .await;
//             mandate.update_storage_scheme(storage_scheme);
//             match storage_scheme {
//                 MerchantStorageScheme::PostgresOnly => mandate
//                     .insert(&conn)
//                     .await
//                     .map_err(|error| report!(errors::StorageError::from(error))),
//                 MerchantStorageScheme::RedisKv => {
//                     let mandate_id = mandate.mandate_id.clone();
//                     let merchant_id = &mandate.merchant_id.to_owned();
//                     let connector_mandate_id = mandate.connector_mandate_id.clone();

//                     let key = PartitionKey::MerchantIdMandateId {
//                         merchant_id,
//                         mandate_id: mandate_id.as_str(),
//                     };
//                     let key_str = key.to_string();
//                     let field = format!("mandate_{}", mandate_id);

//                     let storage_mandate = storage_types::Mandate::from(&mandate);

//                     let redis_entry = kv::TypedSql {
//                         op: kv::DBOperation::Insert {
//                             insertable: Box::new(kv::Insertable::Mandate(mandate)),
//                         },
//                     };

//                     if let Some(connector_val) = connector_mandate_id {
//                         let lookup_id = format!(
//                             "mid_{}_conn_mandate_{}",
//                             merchant_id.get_string_repr(),
//                             connector_val
//                         );

//                         let reverse_lookup_entry = diesel_models::ReverseLookupNew {
//                             sk_id: field.clone(),
//                             pk_id: key_str.clone(),
//                             lookup_id,
//                             source: "mandate".to_string(),
//                             updated_by: storage_scheme.to_string(),
//                         };

//                         self.insert_reverse_lookup(reverse_lookup_entry, storage_scheme)
//                             .await?;
//                     }

//                     match Box::pin(kv_wrapper::<diesel_models::Mandate, _, _>(
//                         self,
//                         KvOperation::<diesel_models::Mandate>::HSetNx(
//                             &field,
//                             &storage_mandate,
//                             redis_entry,
//                         ),
//                         key,
//                     ))
//                     .await
//                     .map_err(|err| err.to_redis_failed_response(&key_str))?
//                     .try_into_hsetnx()
//                     {
//                         Ok(HsetnxReply::KeyNotSet) => Err(errors::StorageError::DuplicateValue {
//                             entity: "mandate",
//                             key: Some(storage_mandate.mandate_id),
//                         }
//                         .into()),
//                         Ok(HsetnxReply::KeySet) => Ok(storage_mandate),
//                         Err(er) => Err(er).change_context(errors::StorageError::KVError),
//                     }
//                 }
//             }
//         }
//     }
// }