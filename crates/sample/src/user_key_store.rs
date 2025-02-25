use common_utils::{
    errors::CustomResult,
    types::keymanager::{KeyManagerState},
};
// use error_stack::{report, ResultExt};
use masking::Secret;
// use router_env::{instrument, tracing};
// use storage_impl::MockDb;

// use crate::{
//     connection,
//     core::errors,
//     services::Store,
//     types::domain::{
//         self,
//         behaviour::{Conversion, ReverseConversion},
//     },
// };

// use hyperswitch_domain_models::errors;
use crate::domain::user_key_store as domain;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait UserKeyStoreInterface {
    type Error;
    async fn insert_user_key_store(
        &self,
        state: &KeyManagerState,
        user_key_store: domain::UserKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, Self::Error>;

    async fn get_user_key_store_by_user_id(
        &self,
        state: &KeyManagerState,
        user_id: &str,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::UserKeyStore, Self::Error>;

    async fn get_all_user_key_store(
        &self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        from: u32,
        limit: u32,
    ) -> CustomResult<Vec<domain::UserKeyStore>, Self::Error>;
}

// #[async_trait::async_trait]
// impl UserKeyStoreInterface for MockDb {
//     #[instrument(skip_all)]
//     async fn insert_user_key_store(
//         &self,
//         state: &KeyManagerState,
//         user_key_store: domain::UserKeyStore,
//         key: &Secret<Vec<u8>>,
//     ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
//         let mut locked_user_key_store = self.user_key_store.lock().await;

//         if locked_user_key_store
//             .iter()
//             .any(|user_key| user_key.user_id == user_key_store.user_id)
//         {
//             Err(errors::StorageError::DuplicateValue {
//                 entity: "user_key_store",
//                 key: Some(user_key_store.user_id.clone()),
//             })?;
//         }

//         let user_key_store = Conversion::convert(user_key_store)
//             .await
//             .change_context(errors::StorageError::MockDbError)?;
//         locked_user_key_store.push(user_key_store.clone());
//         let user_id = user_key_store.user_id.clone();
//         user_key_store
//             .convert(state, key, keymanager::Identifier::User(user_id))
//             .await
//             .change_context(errors::StorageError::DecryptionError)
//     }

//     async fn get_all_user_key_store(
//         &self,
//         state: &KeyManagerState,
//         key: &Secret<Vec<u8>>,
//         _from: u32,
//         _limit: u32,
//     ) -> CustomResult<Vec<domain::UserKeyStore>, errors::StorageError> {
//         let user_key_store = self.user_key_store.lock().await;

//         futures::future::try_join_all(user_key_store.iter().map(|user_key| async {
//             let user_id = user_key.user_id.clone();
//             user_key
//                 .to_owned()
//                 .convert(state, key, keymanager::Identifier::User(user_id))
//                 .await
//                 .change_context(errors::StorageError::DecryptionError)
//         }))
//         .await
//     }

//     #[instrument(skip_all)]
//     async fn get_user_key_store_by_user_id(
//         &self,
//         state: &KeyManagerState,
//         user_id: &str,
//         key: &Secret<Vec<u8>>,
//     ) -> CustomResult<domain::UserKeyStore, errors::StorageError> {
//         self.user_key_store
//             .lock()
//             .await
//             .iter()
//             .find(|user_key_store| user_key_store.user_id == user_id)
//             .cloned()
//             .ok_or(errors::StorageError::ValueNotFound(format!(
//                 "No user_key_store is found for user_id={}",
//                 user_id
//             )))?
//             .convert(state, key, keymanager::Identifier::User(user_id.to_owned()))
//             .await
//             .change_context(errors::StorageError::DecryptionError)
//     }
// }
