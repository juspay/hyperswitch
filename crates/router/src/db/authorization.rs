use diesel_models::authorization::AuthorizationUpdateInternal;
use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait AuthorizationInterface {
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError>;

    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError>;

    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
        authorization_id: String,
        authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError>;
}

#[async_trait::async_trait]
impl AuthorizationInterface for Store {
        /// Asynchronously inserts a new authorization record into the storage. 
    /// 
    /// # Arguments
    /// 
    /// * `authorization` - A new authorization record to be inserted into the storage.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the inserted `storage::Authorization` if successful, or a `errors::StorageError` if an error occurred during insertion.
    /// 
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        authorization
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously finds all authorizations by a given merchant ID and payment ID in the storage.
    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Authorization::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates an authorization by merchant ID and authorization ID
    /// 
    /// # Arguments
    /// 
    /// * `merchant_id` - The ID of the merchant
    /// * `authorization_id` - The ID of the authorization
    /// * `authorization` - The updated authorization data
    /// 
    /// # Returns
    /// 
    /// The updated authorization if successful, or a `StorageError` if an error occurs
    /// 
    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
        authorization_id: String,
        authorization: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authorization::update_by_merchant_id_authorization_id(
            &conn,
            merchant_id,
            authorization_id,
            authorization,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl AuthorizationInterface for MockDb {
        /// Asynchronously inserts a new authorization into the storage, checking for duplicate authorization IDs.
    /// If a duplicate is found, returns a `StorageError` with the details. Otherwise, a new `Authorization` is created
    /// and added to the store, and the new `Authorization` is returned.
    async fn insert_authorization(
        &self,
        authorization: storage::AuthorizationNew,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let mut authorizations = self.authorizations.lock().await;
        if authorizations.iter().any(|authorization_inner| {
            authorization_inner.authorization_id == authorization.authorization_id
        }) {
            Err(errors::StorageError::DuplicateValue {
                entity: "authorization_id",
                key: None,
            })?
        }
        let authorization = storage::Authorization {
            authorization_id: authorization.authorization_id,
            merchant_id: authorization.merchant_id,
            payment_id: authorization.payment_id,
            amount: authorization.amount,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            status: authorization.status,
            error_code: authorization.error_code,
            error_message: authorization.error_message,
            connector_authorization_id: authorization.connector_authorization_id,
            previously_authorized_amount: authorization.previously_authorized_amount,
        };
        authorizations.push(authorization.clone());
        Ok(authorization)
    }

        /// Asynchronously finds all authorizations by a given merchant ID and payment ID.
    async fn find_all_authorizations_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Authorization>, errors::StorageError> {
        let authorizations = self.authorizations.lock().await;
        let authorizations_found: Vec<storage::Authorization> = authorizations
            .iter()
            .filter(|a| a.merchant_id == merchant_id && a.payment_id == payment_id)
            .cloned()
            .collect();

        Ok(authorizations_found)
    }

        /// Asynchronously updates an authorization by merchant ID and authorization ID. It searches for the authorization with the given merchant ID and authorization ID, updates it with the provided authorization update, and returns the updated authorization if found. If no matching authorization is found, it returns a `StorageError` indicating that the authorization could not be found.
    async fn update_authorization_by_merchant_id_authorization_id(
        &self,
        merchant_id: String,
        authorization_id: String,
        authorization_update: storage::AuthorizationUpdate,
    ) -> CustomResult<storage::Authorization, errors::StorageError> {
        let mut authorizations = self.authorizations.lock().await;
        authorizations
            .iter_mut()
            .find(|authorization| authorization.authorization_id == authorization_id && authorization.merchant_id == merchant_id)
            .map(|authorization| {
                let authorization_updated =
                    AuthorizationUpdateInternal::from(authorization_update)
                        .create_authorization(authorization.clone());
                *authorization = authorization_updated.clone();
                authorization_updated
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "cannot find authorization for authorization_id = {authorization_id} and merchant_id = {merchant_id}"
                ))
                .into(),
            )
    }
}
