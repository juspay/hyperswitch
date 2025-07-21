use diesel_models::gsm as storage;
use error_stack::{report, ResultExt};
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
        rule: hyperswitch_domain_models::gsm::GatewayStatusMap,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError>;
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
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError>;
    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: hyperswitch_domain_models::gsm::GatewayStatusMappingUpdate,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError>;

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
        rule: hyperswitch_domain_models::gsm::GatewayStatusMap,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let gsm_db_record = diesel_models::gsm::GatewayStatusMappingNew::try_from(rule)
            .change_context(errors::StorageError::SerializationFailed)
            .attach_printable("Failed to convert gsm domain models to diesel models")?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        hyperswitch_domain_models::gsm::GatewayStatusMap::try_from(gsm_db_record)
            .change_context(errors::StorageError::DeserializationFailed)
            .attach_printable("Failed to convert gsm diesel models to domain models")
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
        storage::GatewayStatusMap::retrieve_decision(
            &conn, connector, flow, sub_flow, code, message,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        let gsm_db_record =
            storage::GatewayStatusMap::find(&conn, connector, flow, sub_flow, code, message)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;

        hyperswitch_domain_models::gsm::GatewayStatusMap::try_from(gsm_db_record)
            .change_context(errors::StorageError::DeserializationFailed)
            .attach_printable("Failed to convert gsm diesel models to domain models")
    }

    #[instrument(skip_all)]
    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: hyperswitch_domain_models::gsm::GatewayStatusMappingUpdate,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let gsm_update_data = diesel_models::gsm::GatewayStatusMappingUpdate::try_from(data)
            .change_context(errors::StorageError::SerializationFailed)?;
        let gsm_db_record = storage::GatewayStatusMap::update(
            &conn,
            connector,
            flow,
            sub_flow,
            code,
            message,
            gsm_update_data,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))?;

        hyperswitch_domain_models::gsm::GatewayStatusMap::try_from(gsm_db_record)
            .change_context(errors::StorageError::DeserializationFailed)
            .attach_printable("Failed to convert gsm diesel models to domain models")
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
        storage::GatewayStatusMap::delete(&conn, connector, flow, sub_flow, code, message)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl GsmInterface for MockDb {
    async fn add_gsm_rule(
        &self,
        _rule: hyperswitch_domain_models::gsm::GatewayStatusMap,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
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
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_gsm_rule(
        &self,
        _connector: String,
        _flow: String,
        _sub_flow: String,
        _code: String,
        _message: String,
        _data: hyperswitch_domain_models::gsm::GatewayStatusMappingUpdate,
    ) -> CustomResult<hyperswitch_domain_models::gsm::GatewayStatusMap, errors::StorageError> {
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
