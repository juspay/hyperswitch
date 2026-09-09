use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    resource as domain,
    resource::ResourceInterface,
};
use hyperswitch_masking::Secret;
use router_env::{instrument, tracing};

use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> ResourceInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_linked_resource(
        &self,
        resource: domain::Resource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        self.router_store.insert_linked_resource(resource, key).await
    }

    #[instrument(skip_all)]
    async fn find_linked_resource_by_id(
        &self,
        id: common_utils::id_type::ResourceId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        self.router_store.find_linked_resource_by_id(id, key).await
    }

    #[instrument(skip_all)]
    async fn find_resource_scope_id(
        &self,
        id: common_utils::id_type::ResourceId,
    ) -> CustomResult<String, Self::Error> {
        self.router_store.find_resource_scope_id(id).await
    }

    #[instrument(skip_all)]
    async fn list_linked_resources_by_scope_id_and_resource_type(
        &self,
        scope_id: String,
        resource_type: String,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::Resource>, Self::Error> {
        self.router_store
            .list_linked_resources_by_scope_id_and_resource_type(scope_id, resource_type, key)
            .await
    }

    #[instrument(skip_all)]
    async fn update_linked_resource_data(
        &self,
        id: common_utils::id_type::ResourceId,
        update: domain::ResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        self.router_store
            .update_linked_resource_data(id, update, key)
            .await
    }

    #[instrument(skip_all)]
    async fn find_requestor_organization_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<String>, Self::Error> {
        self.router_store
            .find_requestor_organization_id(requestor_type, requestor_id)
            .await
    }

    #[instrument(skip_all)]
    async fn resolve_effective_resource_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<common_utils::id_type::ResourceId>, Self::Error> {
        self.router_store
            .resolve_effective_resource_id(requestor_type, requestor_id)
            .await
    }

    #[instrument(skip_all)]
    async fn resolve_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<domain::ApplePayCertificateCache>, Self::Error> {
        self.router_store
            .resolve_apple_pay_certificate_cache(requestor_type, requestor_id)
            .await
    }

    #[instrument(skip_all)]
    async fn set_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
        data: serde_json::Value,
        encrypted_data: common_utils::encryption::Encryption,
    ) -> CustomResult<(), Self::Error> {
        self.router_store
            .set_apple_pay_certificate_cache(requestor_type, requestor_id, data, encrypted_data)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> ResourceInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_linked_resource(
        &self,
        resource: domain::Resource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let identifier = resource
            .key_identifier()
            .change_context(Self::Error::EncryptionError)?;
        resource
            .construct_new()
            .await
            .change_context(Self::Error::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(|error| report!(Self::Error::from(error)))?
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(Self::Error::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_linked_resource_by_id(
        &self,
        id: common_utils::id_type::ResourceId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let resource = diesel_models::resource::Resource::find_by_id(&conn, id)
            .await
            .map_err(|error| report!(Self::Error::from(error)))?;
        let identifier = domain::Resource::identifier_for_diesel(&resource)
            .change_context(Self::Error::DecryptionError)?;

        resource
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(Self::Error::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_resource_scope_id(
        &self,
        id: common_utils::id_type::ResourceId,
    ) -> CustomResult<String, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        diesel_models::resource::Resource::find_by_id(&conn, id)
            .await
            .map(|resource| resource.scope_id)
            .map_err(|error| report!(Self::Error::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_linked_resources_by_scope_id_and_resource_type(
        &self,
        scope_id: String,
        resource_type: String,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::Resource>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let resources = diesel_models::resource::Resource::list_by_scope_id_and_resource_type(
            &conn,
            scope_id,
            resource_type,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))?;

        futures::future::try_join_all(resources.into_iter().map(|resource| async {
            let identifier = domain::Resource::identifier_for_diesel(&resource)
                .change_context(Self::Error::DecryptionError)?;
            resource
                .convert(
                    self.get_keymanager_state()
                        .attach_printable("Missing KeyManagerState")?,
                    key,
                    identifier,
                )
                .await
                .change_context(Self::Error::DecryptionError)
        }))
        .await
    }

    #[instrument(skip_all)]
    async fn update_linked_resource_data(
        &self,
        id: common_utils::id_type::ResourceId,
        update: domain::ResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let resource = diesel_models::resource::Resource::update_by_id(&conn, id, update.into())
            .await
            .map_err(|error| report!(Self::Error::from(error)))?;
        let identifier = domain::Resource::identifier_for_diesel(&resource)
            .change_context(Self::Error::DecryptionError)?;

        resource
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(Self::Error::DecryptionError)
    }

    #[instrument(skip_all)]
    async fn find_requestor_organization_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<String>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        diesel_models::query::resource_link::find_requestor_organization_id(
            &conn,
            requestor_type,
            &requestor_id,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))
    }

    #[instrument(skip_all)]
    async fn resolve_effective_resource_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<common_utils::id_type::ResourceId>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        diesel_models::query::resource_link::resolve_effective_resource_id(
            &conn,
            requestor_type,
            &requestor_id,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))
    }

    #[instrument(skip_all)]
    async fn resolve_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<domain::ApplePayCertificateCache>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        diesel_models::query::resource_link::resolve_apple_pay_certificate_cache(
            &conn,
            requestor_type,
            &requestor_id,
        )
        .await
        .map(|cache| {
            cache.map(|cache| domain::ApplePayCertificateCache {
                data: cache.data,
                encrypted_data: cache.encrypted_data,
            })
        })
        .map_err(|error| report!(Self::Error::from(error)))
    }

    #[instrument(skip_all)]
    async fn set_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
        data: serde_json::Value,
        encrypted_data: common_utils::encryption::Encryption,
    ) -> CustomResult<(), Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        diesel_models::query::resource_link::set_apple_pay_certificate_cache(
            &conn,
            requestor_type,
            &requestor_id,
            data,
            encrypted_data,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))
    }
}

#[async_trait::async_trait]
impl ResourceInterface for MockDb {
    type Error = StorageError;

    async fn insert_linked_resource(
        &self,
        resource: domain::Resource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let mut locked_resources = self.resources.lock().await;

        if locked_resources.iter().any(|stored| stored.id == resource.id) {
            Err(StorageError::DuplicateValue {
                entity: "resources",
                key: Some(resource.id.get_string_repr().to_owned()),
            })?;
        }

        let identifier = resource
            .key_identifier()
            .change_context(StorageError::EncryptionError)?;
        let stored = Conversion::convert(resource)
            .await
            .change_context(StorageError::MockDbError)?;
        locked_resources.push(stored.clone());
        stored
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_linked_resource_by_id(
        &self,
        id: common_utils::id_type::ResourceId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let resource = self
            .resources
            .lock()
            .await
            .iter()
            .find(|stored| stored.id == id)
            .cloned()
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        let identifier = domain::Resource::identifier_for_diesel(&resource)
            .change_context(StorageError::DecryptionError)?;

        resource
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_resource_scope_id(
        &self,
        id: common_utils::id_type::ResourceId,
    ) -> CustomResult<String, Self::Error> {
        let scope_id = self
            .resources
            .lock()
            .await
            .iter()
            .find(|stored| stored.id == id)
            .map(|stored| stored.scope_id.clone())
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        Ok(scope_id)
    }

    async fn list_linked_resources_by_scope_id_and_resource_type(
        &self,
        scope_id: String,
        resource_type: String,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<domain::Resource>, Self::Error> {
        let resources = self.resources.lock().await;
        futures::future::try_join_all(
            resources
                .iter()
                .filter(|stored| stored.scope_id == scope_id && stored.resource_type == resource_type)
                .map(|stored| async {
                    let identifier = domain::Resource::identifier_for_diesel(stored)
                        .change_context(StorageError::DecryptionError)?;
                    stored
                        .to_owned()
                        .convert(
                            self.get_keymanager_state()
                                .attach_printable("Missing KeyManagerState")?,
                            key,
                            identifier,
                        )
                        .await
                        .change_context(StorageError::DecryptionError)
                }),
        )
        .await
    }

    async fn update_linked_resource_data(
        &self,
        id: common_utils::id_type::ResourceId,
        update: domain::ResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::Resource, Self::Error> {
        let mut locked_resources = self.resources.lock().await;
        let index = locked_resources
            .iter()
            .position(|stored| stored.id == id)
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        let update_internal: diesel_models::resource::ResourceUpdateInternal = update.into();
        let entry = locked_resources
            .get_mut(index)
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        if let Some(data) = update_internal.data {
            entry.data = data;
        }
        entry.modified_at = update_internal.modified_at;
        let resource = entry.clone();
        let identifier = domain::Resource::identifier_for_diesel(&resource)
            .change_context(StorageError::DecryptionError)?;

        resource
            .convert(
                self.get_keymanager_state()
                    .attach_printable("Missing KeyManagerState")?,
                key,
                identifier,
            )
            .await
            .change_context(StorageError::DecryptionError)
    }

    async fn find_requestor_organization_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<String>, Self::Error> {
        let merchant_id = match self.resolve_owning_merchant_id(requestor_type, &requestor_id).await? {
            Some(merchant_id) => merchant_id,
            None => return Ok(None),
        };
        let accounts = self.merchant_accounts.lock().await;
        Ok(accounts
            .iter()
            .find(|account| account.get_id() == &merchant_id)
            .map(|account| account.organization_id.get_string_repr().to_string()))
    }

    async fn resolve_effective_resource_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<common_utils::id_type::ResourceId>, Self::Error> {
        let Some(cache) = self
            .resolve_apple_pay_certificate_cache(requestor_type, requestor_id)
            .await?
        else {
            return Ok(None);
        };
        let Some(resource_id) = cache.data.get("resource_id").and_then(|value| value.as_str())
        else {
            return Ok(None);
        };
        parse_id(resource_id).map(Some)
    }

    async fn resolve_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
    ) -> CustomResult<Option<domain::ApplePayCertificateCache>, Self::Error> {
        let (merchant_id, profile_id) = match requestor_type {
            common_enums::ResourceRequestorType::MerchantConnectorAccount => {
                let mca_id = parse_id(&requestor_id)?;
                let accounts = self.merchant_connector_accounts.lock().await;
                let mca = accounts
                    .iter()
                    .find(|mca| mca.get_id() == mca_id)
                    .ok_or(StorageError::ValueNotFound(String::from(
                        "merchant_connector_account",
                    )))?;
                if let Some(data) = mca.apple_pay_certificates.clone() {
                    return Ok(Some(domain::ApplePayCertificateCache {
                        data,
                        encrypted_data: mca.apple_pay_certificates_encrypted.clone(),
                    }));
                }
                (mca.merchant_id.clone(), mca_profile_id(mca))
            }
            common_enums::ResourceRequestorType::Profile => {
                let profile_id = parse_id::<common_utils::id_type::ProfileId>(&requestor_id)?;
                let profiles = self.business_profiles.lock().await;
                let profile = profiles
                    .iter()
                    .find(|profile| profile.get_id() == &profile_id)
                    .ok_or(StorageError::ValueNotFound(String::from("business_profile")))?;
                if let Some(data) = profile.apple_pay_certificates.clone() {
                    return Ok(Some(domain::ApplePayCertificateCache {
                        data,
                        encrypted_data: profile.apple_pay_certificates_encrypted.clone(),
                    }));
                }
                (profile.merchant_id.clone(), None)
            }
            common_enums::ResourceRequestorType::MerchantAccount => {
                (parse_id(&requestor_id)?, None)
            }
        };

        if let Some(profile_id) = profile_id {
            let profiles = self.business_profiles.lock().await;
            if let Some(data) = profiles
                .iter()
                .find(|profile| {
                    profile.merchant_id == merchant_id && profile.get_id() == &profile_id
                })
                .and_then(|profile| profile.apple_pay_certificates.clone())
            {
                return Ok(Some(domain::ApplePayCertificateCache {
                    data,
                    encrypted_data: None,
                }));
            }
        }

        let accounts = self.merchant_accounts.lock().await;
        let account = accounts
            .iter()
            .find(|account| account.get_id() == &merchant_id)
            .ok_or(StorageError::ValueNotFound(String::from("merchant_account")))?;
        Ok(account.apple_pay_certificates.clone().map(|data| {
            domain::ApplePayCertificateCache {
                data,
                encrypted_data: account.apple_pay_certificates_encrypted.clone(),
            }
        }))
    }

    async fn set_apple_pay_certificate_cache(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: String,
        data: serde_json::Value,
        encrypted_data: common_utils::encryption::Encryption,
    ) -> CustomResult<(), Self::Error> {
        match requestor_type {
            common_enums::ResourceRequestorType::MerchantConnectorAccount => {
                let mca_id = parse_id(&requestor_id)?;
                let mut accounts = self.merchant_connector_accounts.lock().await;
                let mca = accounts
                    .iter_mut()
                    .find(|mca| mca.get_id() == mca_id)
                    .ok_or(StorageError::ValueNotFound(String::from(
                        "merchant_connector_account",
                    )))?;
                mca.apple_pay_certificates = Some(data);
                mca.apple_pay_certificates_encrypted = Some(encrypted_data);
            }
            common_enums::ResourceRequestorType::Profile => {
                let profile_id = parse_id(&requestor_id)?;
                let mut profiles = self.business_profiles.lock().await;
                let profile = profiles
                    .iter_mut()
                    .find(|profile| profile.get_id() == &profile_id)
                    .ok_or(StorageError::ValueNotFound(String::from("business_profile")))?;
                profile.apple_pay_certificates = Some(data);
                profile.apple_pay_certificates_encrypted = Some(encrypted_data);
            }
            common_enums::ResourceRequestorType::MerchantAccount => {
                let merchant_id = parse_id(&requestor_id)?;
                let mut accounts = self.merchant_accounts.lock().await;
                let account = accounts
                    .iter_mut()
                    .find(|account| account.get_id() == &merchant_id)
                    .ok_or(StorageError::ValueNotFound(String::from("merchant_account")))?;
                account.apple_pay_certificates = Some(data);
                account.apple_pay_certificates_encrypted = Some(encrypted_data);
            }
        }
        Ok(())
    }
}

impl MockDb {
    async fn resolve_owning_merchant_id(
        &self,
        requestor_type: common_enums::ResourceRequestorType,
        requestor_id: &str,
    ) -> CustomResult<Option<common_utils::id_type::MerchantId>, StorageError> {
        match requestor_type {
            common_enums::ResourceRequestorType::MerchantConnectorAccount => {
                let mca_id = parse_id(requestor_id)?;
                Ok(self
                    .merchant_connector_accounts
                    .lock()
                    .await
                    .iter()
                    .find(|mca| mca.get_id() == mca_id)
                    .map(|mca| mca.merchant_id.clone()))
            }
            common_enums::ResourceRequestorType::Profile => {
                let profile_id = parse_id(requestor_id)?;
                Ok(self
                    .business_profiles
                    .lock()
                    .await
                    .iter()
                    .find(|profile| profile.get_id() == &profile_id)
                    .map(|profile| profile.merchant_id.clone()))
            }
            common_enums::ResourceRequestorType::MerchantAccount => {
                Ok(Some(parse_id(requestor_id)?))
            }
        }
    }
}

#[cfg(feature = "v1")]
fn mca_profile_id(
    mca: &diesel_models::merchant_connector_account::MerchantConnectorAccount,
) -> Option<common_utils::id_type::ProfileId> {
    mca.profile_id.clone()
}

#[cfg(feature = "v2")]
fn mca_profile_id(
    mca: &diesel_models::merchant_connector_account::MerchantConnectorAccount,
) -> Option<common_utils::id_type::ProfileId> {
    Some(mca.profile_id.clone())
}

fn parse_id<T>(value: &str) -> CustomResult<T, StorageError>
where
    T: TryFrom<
        std::borrow::Cow<'static, str>,
        Error = error_stack::Report<common_utils::errors::ValidationError>,
    >,
{
    T::try_from(std::borrow::Cow::Owned(value.to_owned()))
        .change_context(StorageError::MockDbError)
}
