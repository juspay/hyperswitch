use diesel_models::gsm as storage;
use error_stack::IntoReport;
use router_env::{instrument, tracing};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

#[async_trait::async_trait]
pub trait GsmInterface {
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError>;
    async fn find_gsm_decision(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<String, errors::StorageError>;
    async fn find_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError>;
    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: storage::GatewayStatusMappingUpdate,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError>;

    async fn delete_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl GsmInterface for Store {
    #[instrument(skip_all)]
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        rule.insert(&conn).await.map_err(Into::into).into_report()
    }

    #[instrument(skip_all)]
    async fn find_gsm_decision(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<String, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GatewayStatusMap::retrieve_gsm_decision(
            &conn, connector, flow, sub_flow, code, message,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    #[instrument(skip_all)]
    async fn find_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GatewayStatusMap::find_gsm(&conn, connector, flow, sub_flow, code, message)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: storage::GatewayStatusMappingUpdate,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::GatewayStatusMap::update_gsm(&conn, connector, flow, sub_flow, code, message, data)
            .await
            .map_err(Into::into)
            .into_report()
    }

    #[instrument(skip_all)]
    async fn delete_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<bool, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::GatewayStatusMap::delete_gsm(&conn, connector, flow, sub_flow, code, message)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl GsmInterface for MockDb {
    async fn add_gsm_rule(
        &self,
        _rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_gsm_decision(
        &self,
        _connector: String,
        _flow: String,
        _sub_flow: String,
        _code: String,
        _message: String,
    ) -> CustomResult<String, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_gsm_rule(
        &self,
        _connector: String,
        _flow: String,
        _sub_flow: String,
        _code: String,
        _message: String,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_gsm_rule(
        &self,
        _connector: String,
        _flow: String,
        _sub_flow: String,
        _code: String,
        _message: String,
        _data: storage::GatewayStatusMappingUpdate,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_gsm_rule(
        &self,
        _connector: String,
        _flow: String,
        _sub_flow: String,
        _code: String,
        _message: String,
    ) -> CustomResult<bool, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }
}
