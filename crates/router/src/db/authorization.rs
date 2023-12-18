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
