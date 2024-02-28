use diesel_models::authentication::AuthenticationUpdateInternal;
use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait AuthenticationInterface {
    async fn insert_authentication(
        &self,
        authentication: storage::AuthenticationNew,
    ) -> CustomResult<storage::Authentication, errors::StorageError>;

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: String,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError>;

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError>;
}

#[async_trait::async_trait]
impl AuthenticationInterface for Store {
    async fn insert_authentication(
        &self,
        authentication: storage::AuthenticationNew,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        authentication
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: String,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authentication::find_by_merchant_id_authentication_id(
            &conn,
            &merchant_id,
            &authentication_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::Authentication::update_by_merchant_id_authentication_id(
            &conn,
            previous_state.merchant_id,
            previous_state.authentication_id,
            authentication_update,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }
}

#[async_trait::async_trait]
impl AuthenticationInterface for MockDb {
    async fn insert_authentication(
        &self,
        authentication: storage::AuthenticationNew,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let mut authentications = self.authentications.lock().await;
        if authentications.iter().any(|authentication_inner| {
            authentication_inner.authentication_id == authentication.authentication_id
        }) {
            Err(errors::StorageError::DuplicateValue {
                entity: "authentication_id",
                key: None,
            })?
        }
        let authentication = storage::Authentication {
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            authentication_id: authentication.authentication_id,
            merchant_id: authentication.merchant_id,
            authentication_status: authentication.authentication_status,
            authentication_connector: authentication.authentication_connector,
            connector_authentication_id: authentication.connector_authentication_id,
            authentication_data: authentication.authentication_data,
            payment_method_id: authentication.payment_method_id,
            authentication_type: authentication.authentication_type,
            authentication_lifecycle_status: authentication.authentication_lifecycle_status,
            error_code: authentication.error_code,
            error_message: authentication.error_message,
            connector_metadata: authentication.connector_metadata,
        };
        authentications.push(authentication.clone());
        Ok(authentication)
    }

    async fn find_authentication_by_merchant_id_authentication_id(
        &self,
        merchant_id: String,
        authentication_id: String,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let authentications = self.authentications.lock().await;
        authentications
            .iter()
            .find(|a| a.merchant_id == merchant_id && a.authentication_id == authentication_id)
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "cannot find authentication for authentication_id = {authentication_id} and merchant_id = {merchant_id}"
                )).into(),
            ).cloned()
    }

    async fn update_authentication_by_merchant_id_authentication_id(
        &self,
        previous_state: storage::Authentication,
        authentication_update: storage::AuthenticationUpdate,
    ) -> CustomResult<storage::Authentication, errors::StorageError> {
        let mut authentications = self.authentications.lock().await;
        let authentication_id = previous_state.authentication_id.clone();
        let merchant_id = previous_state.merchant_id.clone();
        authentications
            .iter_mut()
            .find(|authentication| authentication.authentication_id == authentication_id && authentication.merchant_id == merchant_id)
            .map(|authentication| {
                let authentication_update_internal =
                    AuthenticationUpdateInternal::from(authentication_update);
                let updated_authentication = authentication_update_internal.apply_changeset(previous_state);
                *authentication = updated_authentication.clone();
                updated_authentication
            })
            .ok_or(
                errors::StorageError::ValueNotFound(format!(
                    "cannot find authentication for authentication_id = {authentication_id} and merchant_id = {merchant_id}"
                ))
                .into(),
            )
    }
}
