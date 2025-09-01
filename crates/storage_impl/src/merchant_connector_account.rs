use common_utils::{id_type, types::keymanager::KeyManagerState};
use diesel_models::merchant_connector_account::{
    MerchantConnectorAccount, MerchantConnectorAccountUpdateInternal,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    merchant_connector_account::{self as domain},
    merchant_key_store::MerchantKeyStore,
};

use crate::{
    errors::StorageError,
    kv_router_store::KVRouterStore,
    utils::{pg_connection_read, pg_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> domain::MerchantConnectorAccountInterface for RouterStore<T> {
    type Error = StorageError;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            MerchantConnectorAccount::find_by_merchant_id_connector(
                &conn,
                merchant_id,
                connector_label,
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            MerchantConnectorAccount::find_by_profile_id_connector_name(
                &conn,
                profile_id,
                connector_name,
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            MerchantConnectorAccount::find_by_merchant_id_connector_name(
                &conn,
                merchant_id,
                connector_name,
            ),
        )
        .await
    }

    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_account_new = t
            .construct_new()
            .await
            .change_context(StorageError::EncryptionError)?;

        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            merchant_connector_account_new.insert(&conn),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                merchant_id,
                merchant_connector_id,
            ),
        )
        .await
    }

    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.call_database(
            state,
            key_store,
            #[cfg(feature = "v1")]
            MerchantConnectorAccount::find_by_merchant_id_merchant_connector_id(
                &conn,
                &key_store.merchant_id,
                id,
            ),
            #[cfg(feature = "v2")]
            MerchantConnectorAccount::find_by_id(&conn, id),
        )
        .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        let conn = pg_connection_read(self).await?;
        let filtered_accounts = self
            .find_resources(
                state,
                key_store,
                MerchantConnectorAccount::find_by_merchant_id(&conn, merchant_id, get_disabled),
            )
            .await?;
        Ok(domain::MerchantConnectorAccounts::new(filtered_accounts))
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            MerchantConnectorAccount::list_by_profile_id(&conn, profile_id),
        )
        .await
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let conn = pg_connection_read(self).await?;
        self.find_resources(
            state,
            key_store,
            MerchantConnectorAccount::list_enabled_by_profile_id(&conn, profile_id, connector_type),
        )
        .await
    }

    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_account_storage = Conversion::convert(this)
            .await
            .change_context(StorageError::EncryptionError)?;

        let conn = pg_connection_write(self).await?;
        self.call_database(
            state,
            key_store,
            merchant_connector_account_storage.update(&conn, merchant_connector_account),
        )
        .await
    }

    async fn update_multiple_merchant_connector_accounts(
        &self,
        this: Vec<(
            domain::MerchantConnectorAccount,
            MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), Self::Error> {
        let conn = pg_connection_write(self).await?;
        // Process each update individually
        // In a real implementation, this would be wrapped in a database transaction
        for (domain_mca, update) in this {
            let storage_mca = Conversion::convert(domain_mca)
                .await
                .change_context(StorageError::EncryptionError)?;

            storage_mca.update(&conn, update).await.map_err(|error| {
                let new_err = crate::diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })?;
        }

        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let conn = pg_connection_write(self).await?;
        MerchantConnectorAccount::delete_by_merchant_id_merchant_connector_id(
            &conn,
            merchant_id,
            merchant_connector_id,
        )
        .await
        .map_err(|error| {
            let new_err = crate::diesel_error_to_data_error(*error.current_context());
            error.change_context(new_err)
        })
    }

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let conn = pg_connection_write(self).await?;
        MerchantConnectorAccount::delete_by_id(&conn, id)
            .await
            .map_err(|error| {
                let new_err = crate::diesel_error_to_data_error(*error.current_context());
                error.change_context(new_err)
            })
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> domain::MerchantConnectorAccountInterface for KVRouterStore<T> {
    type Error = StorageError;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_connector_label(
                state,
                merchant_id,
                connector_label,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_profile_id_connector_name(
                state,
                profile_id,
                connector_name,
                key_store,
            )
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_connector_name(
                state,
                merchant_id,
                connector_name,
                key_store,
            )
            .await
    }

    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .insert_merchant_connector_account(state, t, key_store)
            .await
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                state,
                merchant_id,
                merchant_connector_id,
                key_store,
            )
            .await
    }

    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_id(state, id, key_store)
            .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        self.router_store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                state,
                merchant_id,
                get_disabled,
                key_store,
            )
            .await
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .list_connector_account_by_profile_id(state, profile_id, key_store)
            .await
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        self.router_store
            .list_enabled_connector_accounts_by_profile_id(
                state,
                profile_id,
                key_store,
                connector_type,
            )
            .await
    }

    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        self.router_store
            .update_merchant_connector_account(state, this, merchant_connector_account, key_store)
            .await
    }

    async fn update_multiple_merchant_connector_accounts(
        &self,
        this: Vec<(
            domain::MerchantConnectorAccount,
            MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), Self::Error> {
        self.router_store
            .update_multiple_merchant_connector_accounts(this)
            .await
    }

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        self.router_store
            .delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
                merchant_id,
                merchant_connector_id,
            )
            .await
    }

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        self.router_store
            .delete_merchant_connector_account_by_id(id)
            .await
    }
}

#[async_trait::async_trait]
impl domain::MerchantConnectorAccountInterface for MockDb {
    type Error = StorageError;

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_label(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_label: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resource(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| {
                mca.merchant_id == *merchant_id
                    && mca.connector_label.as_deref() == Some(connector_label)
            },
            format!(
                "MerchantConnectorAccount with merchant_id {} and connector_label {} not found",
                merchant_id.get_string_repr(),
                connector_label
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_profile_id_connector_name(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resource(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| {
                mca.profile_id.as_ref() == Some(profile_id) && mca.connector_name == connector_name
            },
            format!(
                "MerchantConnectorAccount with profile_id {} and connector_name {} not found",
                profile_id.get_string_repr(),
                connector_name
            ),
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn find_merchant_connector_account_by_merchant_id_connector_name(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        connector_name: &str,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resources(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| mca.merchant_id == *merchant_id && mca.connector_name == connector_name,
            format!(
                "MerchantConnectorAccount with merchant_id {} and connector_name {} not found",
                merchant_id.get_string_repr(),
                connector_name
            ),
        )
        .await
    }

    async fn insert_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        t: domain::MerchantConnectorAccount,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let mut merchant_connector_accounts = self.merchant_connector_accounts.lock().await;

        let merchant_connector_account = Conversion::convert(t)
            .await
            .change_context(StorageError::EncryptionError)?;

        merchant_connector_accounts.push(merchant_connector_account.clone());

        merchant_connector_account
            .convert(
                state,
                key_store.key.get_inner(),
                key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    #[cfg(feature = "v1")]
    async fn find_by_merchant_connector_account_merchant_id_merchant_connector_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resource(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| {
                mca.merchant_id == *merchant_id
                    && mca.merchant_connector_id == *merchant_connector_id
            },
            format!(
                "MerchantConnectorAccount with merchant_id {} and merchant_connector_id {} not found",
                merchant_id.get_string_repr(),
                merchant_connector_id.get_string_repr()
            ),
        )
        .await
    }

    async fn find_merchant_connector_account_by_id(
        &self,
        state: &KeyManagerState,
        id: &id_type::MerchantConnectorAccountId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resource(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| {
                #[cfg(feature = "v1")]
                {
                    mca.merchant_connector_id == *id
                }
                #[cfg(feature = "v2")]
                {
                    mca.id == *id
                }
            },
            format!(
                "MerchantConnectorAccount with id {} not found",
                id.get_string_repr()
            ),
        )
        .await
    }

    async fn find_merchant_connector_account_by_merchant_id_and_disabled_list(
        &self,
        state: &KeyManagerState,
        merchant_id: &id_type::MerchantId,
        get_disabled: bool,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccounts, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        let filtered_accounts = self
            .get_resources(
                state,
                key_store,
                merchant_connector_accounts,
                |mca| {
                    mca.merchant_id == *merchant_id && mca.disabled.unwrap_or(false) == get_disabled
                },
                format!(
                    "MerchantConnectorAccount with merchant_id {} not found",
                    merchant_id.get_string_repr()
                ),
            )
            .await?;
        Ok(domain::MerchantConnectorAccounts::new(filtered_accounts))
    }

    #[cfg(all(feature = "olap", feature = "v2"))]
    async fn list_connector_account_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resources(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| mca.profile_id == *profile_id,
            format!(
                "MerchantConnectorAccount with profile_id {} not found",
                profile_id.get_string_repr()
            ),
        )
        .await
    }

    async fn list_enabled_connector_accounts_by_profile_id(
        &self,
        state: &KeyManagerState,
        profile_id: &id_type::ProfileId,
        key_store: &MerchantKeyStore,
        connector_type: common_enums::ConnectorType,
    ) -> CustomResult<Vec<domain::MerchantConnectorAccount>, Self::Error> {
        let merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        self.get_resources(
            state,
            key_store,
            merchant_connector_accounts,
            |mca| {
                #[cfg(feature = "v1")]
                let profile_matches = mca.profile_id == Some(profile_id.clone());
                #[cfg(feature = "v2")]
                let profile_matches = mca.profile_id == profile_id.clone();
                profile_matches
                    && mca.connector_type == connector_type
                    && !mca.disabled.unwrap_or(false)
            },
            format!(
                "Enabled MerchantConnectorAccount with profile_id {} and connector_type {:?} not found",
                profile_id.get_string_repr(),
                connector_type
            ),
        )
        .await
    }

    async fn update_merchant_connector_account(
        &self,
        state: &KeyManagerState,
        this: domain::MerchantConnectorAccount,
        merchant_connector_account: MerchantConnectorAccountUpdateInternal,
        key_store: &MerchantKeyStore,
    ) -> CustomResult<domain::MerchantConnectorAccount, Self::Error> {
        let merchant_connector_account_updated = merchant_connector_account
            .create_merchant_connector_account(
                Conversion::convert(this.clone())
                    .await
                    .change_context(StorageError::EncryptionError)?,
            );

        self.update_resource::<MerchantConnectorAccount, _>(
            state,
            key_store,
            self.merchant_connector_accounts.lock().await,
            merchant_connector_account_updated,
            |mca| {
                #[cfg(feature = "v1")]
                {
                    mca.merchant_connector_id == this.get_id()
                }
                #[cfg(feature = "v2")]
                {
                    mca.id == this.get_id()
                }
            },
            "cannot update merchant connector account".to_string(),
        )
        .await
    }

    async fn update_multiple_merchant_connector_accounts(
        &self,
        this: Vec<(
            domain::MerchantConnectorAccount,
            MerchantConnectorAccountUpdateInternal,
        )>,
    ) -> CustomResult<(), Self::Error> {
        let mut merchant_connector_accounts = self.merchant_connector_accounts.lock().await;

        for (domain_mca, update) in this {
            let storage_mca = Conversion::convert(domain_mca.clone())
                .await
                .change_context(StorageError::EncryptionError)?;

            let updated_mca = update.create_merchant_connector_account(storage_mca);

            if let Some(pos) = merchant_connector_accounts.iter().position(|mca| {
                #[cfg(feature = "v1")]
                {
                    mca.merchant_connector_id == domain_mca.get_id()
                }
                #[cfg(feature = "v2")]
                {
                    mca.id == domain_mca.get_id()
                }
            }) {
                if let Some(mca) = merchant_connector_accounts.get_mut(pos) {
                    *mca = updated_mca;
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "v1")]
    async fn delete_merchant_connector_account_by_merchant_id_merchant_connector_id(
        &self,
        merchant_id: &id_type::MerchantId,
        merchant_connector_id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let mut merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        let initial_len = merchant_connector_accounts.len();
        merchant_connector_accounts.retain(|mca| {
            !(mca.merchant_id == *merchant_id
                && mca.merchant_connector_id == *merchant_connector_id)
        });
        Ok(merchant_connector_accounts.len() < initial_len)
    }

    #[cfg(feature = "v2")]
    async fn delete_merchant_connector_account_by_id(
        &self,
        id: &id_type::MerchantConnectorAccountId,
    ) -> CustomResult<bool, Self::Error> {
        let mut merchant_connector_accounts = self.merchant_connector_accounts.lock().await;
        let initial_len = merchant_connector_accounts.len();
        merchant_connector_accounts.retain(|mca| mca.id != *id);
        Ok(merchant_connector_accounts.len() < initial_len)
    }
}
