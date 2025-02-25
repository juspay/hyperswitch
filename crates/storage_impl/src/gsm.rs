use common_utils::errors::CustomResult;
use diesel_models::gsm as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::gsm::GsmInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> GsmInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        rule.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::GatewayStatusMap::find(&conn, connector, flow, sub_flow, code, message)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
        storage::GatewayStatusMap::update(&conn, connector, flow, sub_flow, code, message, data)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
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
