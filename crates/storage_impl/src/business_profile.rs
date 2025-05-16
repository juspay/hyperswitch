use common_utils::{errors::CustomResult as StorageResult, ext_traits::AsyncExt, id_type, types::keymanager::KeyManagerState};
use diesel_models::{
    business_profile as diesel_business_profile,
    business_profile::ProfileUpdateInternal as DieselProfileUpdateInternal,
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::Conversion, 
    business_profile as hdm_business_profile,
    db::business_profile::ProfileInterface,
    merchant_key_store as hdm_merchant_key_store,
};
use hyperswitch_domain_models::behaviour::ReverseConversion;
use router_env::{instrument, tracing};

use crate::{
    connection, 
    errors,     
    kv_router_store::KVRouterStore,
    mock_db::MockDb, 
    DatabaseStore,   
    RouterStore,     
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        business_profile: hdm_business_profile::Profile,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        
        business_profile
            .construct_new() 
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn) 
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert( 
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_merchant_id_profile_id(
            &conn,
            merchant_id,
            profile_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?
        .convert(
            key_manager_state,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_profile_name_merchant_id(
            &conn,
            profile_name,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?
        .convert(
            key_manager_state,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        current_state: hdm_business_profile::Profile,
        profile_update: hdm_business_profile::ProfileUpdate,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        Conversion::convert(current_state) 
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update_by_profile_id(&conn, DieselProfileUpdateInternal::from(profile_update)) 
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &id_type::ProfileId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<bool, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        diesel_business_profile::Profile::delete_by_profile_id_merchant_id(
            &conn,
            profile_id,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Vec<hdm_business_profile::Profile>, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::list_profile_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|profiles_diesel| async {
                let mut profiles_hdm = Vec::with_capacity(profiles_diesel.len());
                for profile_diesel in profiles_diesel.into_iter() {
                    profiles_hdm.push(
                        profile_diesel
                            .convert(
                                key_manager_state,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    );
                }
                Ok(profiles_hdm)
            })
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ProfileInterface for KVRouterStore<T> {
    type Error = errors::StorageError;

    
    
    

    #[instrument(skip_all)]
    async fn insert_business_profile(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        business_profile: hdm_business_profile::Profile,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?; 
        business_profile
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_merchant_id_profile_id(
            &conn,
            merchant_id,
            profile_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?
        .convert(
            key_manager_state,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        profile_name: &str,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::find_by_profile_name_merchant_id(
            &conn,
            profile_name,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?
        .convert(
            key_manager_state,
            merchant_key_store.key.get_inner(),
            merchant_key_store.merchant_id.clone().into(),
        )
        .await
        .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn update_profile_by_profile_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        current_state: hdm_business_profile::Profile,
        profile_update: hdm_business_profile::ProfileUpdate,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        Conversion::convert(current_state)
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .update_by_profile_id(&conn, DieselProfileUpdateInternal::from(profile_update))
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?
            .convert(
                key_manager_state,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        profile_id: &id_type::ProfileId,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<bool, Self::Error> {
        let conn = connection::pg_accounts_connection_write(self).await?;
        diesel_business_profile::Profile::delete_by_profile_id_merchant_id(
            &conn,
            profile_id,
            merchant_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_by_merchant_id(
        &self,
        key_manager_state: &KeyManagerState,
        merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Vec<hdm_business_profile::Profile>, Self::Error> {
        let conn = connection::pg_accounts_connection_read(self).await?;
        diesel_business_profile::Profile::list_profile_by_merchant_id(&conn, merchant_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
            .async_and_then(|profiles_diesel| async {
                let mut profiles_hdm = Vec::with_capacity(profiles_diesel.len());
                for profile_diesel in profiles_diesel.into_iter() {
                    profiles_hdm.push(
                        profile_diesel
                            .convert(
                                key_manager_state,
                                merchant_key_store.key.get_inner(),
                                merchant_key_store.merchant_id.clone().into(),
                            )
                            .await
                            .change_context(errors::StorageError::DecryptionError)?,
                    );
                }
                Ok(profiles_hdm)
            })
            .await
    }
}

#[async_trait::async_trait]
impl ProfileInterface for MockDb {
    type Error = errors::StorageError;

    async fn insert_business_profile(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _business_profile: hdm_business_profile::Profile,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("insert_business_profile not supported for MockDb in storage_impl"))
    }

    async fn find_business_profile_by_profile_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("find_business_profile_by_profile_id not supported for MockDb in storage_impl"))
    }

    async fn find_business_profile_by_merchant_id_profile_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _merchant_id: &id_type::MerchantId,
        _profile_id: &id_type::ProfileId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("find_business_profile_by_merchant_id_profile_id not supported for MockDb in storage_impl"))
    }

    async fn find_business_profile_by_profile_name_merchant_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _profile_name: &str,
        _merchant_id: &id_type::MerchantId,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("find_business_profile_by_profile_name_merchant_id not supported for MockDb in storage_impl"))
    }

    async fn update_profile_by_profile_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _current_state: hdm_business_profile::Profile,
        _profile_update: hdm_business_profile::ProfileUpdate,
    ) -> StorageResult<hdm_business_profile::Profile, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("update_profile_by_profile_id not supported for MockDb in storage_impl"))
    }

    async fn delete_profile_by_profile_id_merchant_id(
        &self,
        _profile_id: &id_type::ProfileId,
        _merchant_id: &id_type::MerchantId,
    ) -> StorageResult<bool, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("delete_profile_by_profile_id_merchant_id not supported for MockDb in storage_impl"))
    }

    async fn list_profile_by_merchant_id(
        &self,
        _key_manager_state: &KeyManagerState,
        _merchant_key_store: &hdm_merchant_key_store::MerchantKeyStore,
        _merchant_id: &id_type::MerchantId,
    ) -> StorageResult<Vec<hdm_business_profile::Profile>, Self::Error> {
        Err(report!(errors::StorageError::MockDbError)
            .attach_printable("list_profile_by_merchant_id not supported for MockDb in storage_impl"))
    }
}
