use error_stack::{report, ResultExt};
use hyperswitch_domain_models::{
    behaviour::{Conversion, ReverseConversion},
    hierarchical_resource as domain,
    hierarchical_resource::HierarchicalResourceInterface,
};
use hyperswitch_masking::Secret;
use router_env::{instrument, tracing};

use crate::{
    kv_router_store,
    utils::{pg_accounts_connection_read, pg_accounts_connection_write},
    CustomResult, DatabaseStore, MockDb, RouterStore, StorageError,
};

#[async_trait::async_trait]
impl<T: DatabaseStore> HierarchicalResourceInterface for kv_router_store::KVRouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_linked_resource(
        &self,
        resource: domain::HierarchicalResource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        self.router_store
            .insert_linked_resource(resource, key)
            .await
    }

    #[instrument(skip_all)]
    async fn find_linked_resource_by_id(
        &self,
        id: common_utils::id_type::ResourceId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
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
    ) -> CustomResult<Vec<domain::HierarchicalResource>, Self::Error> {
        self.router_store
            .list_linked_resources_by_scope_id_and_resource_type(scope_id, resource_type, key)
            .await
    }

    #[instrument(skip_all)]
    async fn update_linked_resource_data(
        &self,
        id: common_utils::id_type::ResourceId,
        update: domain::HierarchicalResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        self.router_store
            .update_linked_resource_data(id, update, key)
            .await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> HierarchicalResourceInterface for RouterStore<T> {
    type Error = StorageError;

    #[instrument(skip_all)]
    async fn insert_linked_resource(
        &self,
        resource: domain::HierarchicalResource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
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
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let resource =
            diesel_models::hierarchical_resource::HierarchicalResource::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(Self::Error::from(error)))?;
        let identifier = domain::HierarchicalResource::identifier_for_diesel(&resource)
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
        diesel_models::hierarchical_resource::HierarchicalResource::find_by_id(&conn, id)
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
    ) -> CustomResult<Vec<domain::HierarchicalResource>, Self::Error> {
        let conn = pg_accounts_connection_read(self).await?;
        let resources = diesel_models::hierarchical_resource::HierarchicalResource::list_by_scope_id_and_resource_type(
            &conn,
            scope_id,
            resource_type,
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))?;

        futures::future::try_join_all(resources.into_iter().map(|resource| async {
            let identifier = domain::HierarchicalResource::identifier_for_diesel(&resource)
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
        update: domain::HierarchicalResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        let conn = pg_accounts_connection_write(self).await?;
        let resource = diesel_models::hierarchical_resource::HierarchicalResource::update_by_id(
            &conn,
            id,
            update.into(),
        )
        .await
        .map_err(|error| report!(Self::Error::from(error)))?;
        let identifier = domain::HierarchicalResource::identifier_for_diesel(&resource)
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
}

#[async_trait::async_trait]
impl HierarchicalResourceInterface for MockDb {
    type Error = StorageError;

    async fn insert_linked_resource(
        &self,
        resource: domain::HierarchicalResource,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        let mut locked_resources = self.hierarchical_resources.lock().await;

        if locked_resources
            .iter()
            .any(|stored| stored.id == resource.id)
        {
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
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        let resource = self
            .hierarchical_resources
            .lock()
            .await
            .iter()
            .find(|stored| stored.id == id)
            .cloned()
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        let identifier = domain::HierarchicalResource::identifier_for_diesel(&resource)
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
            .hierarchical_resources
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
    ) -> CustomResult<Vec<domain::HierarchicalResource>, Self::Error> {
        let resources = self.hierarchical_resources.lock().await;
        futures::future::try_join_all(
            resources
                .iter()
                .filter(|stored| {
                    stored.scope_id == scope_id && stored.resource_type == resource_type
                })
                .map(|stored| async {
                    let identifier = domain::HierarchicalResource::identifier_for_diesel(stored)
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
        update: domain::HierarchicalResourceDataUpdate,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<domain::HierarchicalResource, Self::Error> {
        let mut locked_resources = self.hierarchical_resources.lock().await;
        let index = locked_resources
            .iter()
            .position(|stored| stored.id == id)
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        let update_internal: diesel_models::hierarchical_resource::HierarchicalResourceUpdateInternal = update.into();
        let entry = locked_resources
            .get_mut(index)
            .ok_or(StorageError::ValueNotFound(String::from("resources")))?;
        if let Some(data) = update_internal.data {
            entry.data = data;
        }
        entry.modified_at = update_internal.modified_at;
        let resource = entry.clone();
        let identifier = domain::HierarchicalResource::identifier_for_diesel(&resource)
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
}
