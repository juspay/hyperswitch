use diesel_models::gsm as storage;
use error_stack::IntoReport;

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
        /// Asynchronously adds a new GSM rule to the storage. 
    /// 
    /// # Arguments
    /// 
    /// * `rule` - A `GatewayStatusMappingNew` object representing the new rule to be added.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a `GatewayStatusMap` if the rule is added successfully, otherwise an `errors::StorageError`.
    /// 
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        rule.insert(&conn).await.map_err(Into::into).into_report()
    }

        /// Asynchronously finds the GSM decision for the given connector, flow, sub_flow, code, and message
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
        .map_err(Into::into)
        .into_report()
    }

        /// Asynchronously finds a gateway status map based on the provided parameters.
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
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously updates a GSM (Gateway Status Mapping) rule in the database with the provided information.
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
            .map_err(Into::into)
            .into_report()
    }

        /// Asynchronously deletes a GSM rule from the storage. The method takes the connector, flow, sub_flow, code, and message as input parameters and returns a custom result indicating if the rule was successfully deleted or if an error occurred.
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
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl GsmInterface for MockDb {
        /// Asynchronously adds a new GSM rule to the storage. Returns the updated GSM map on success,
    /// or a StorageError if an error occurs.
    async fn add_gsm_rule(
        &self,
        _rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
        Err(errors::StorageError::MockDbError)?
    }

        /// Asynchronously find the GSM decision in the database based on the provided parameters such as connector, flow, sub_flow, code, and message.
    /// If an error occurs during the database operation, it returns a `StorageError` wrapped in a `CustomResult`.
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

    /// Asynchronously finds a GSM rule based on the provided parameters such as connector, flow, sub flow, code, and message.
    ///
    /// # Arguments
    ///
    /// * `_connector` - A String representing the connector for the GSM rule.
    /// * `_flow` - A String representing the flow for the GSM rule.
    /// * `_sub_flow` - A String representing the sub flow for the GSM rule.
    /// * `_code` - A String representing the code for the GSM rule.
    /// * `_message` - A String representing the message for the GSM rule.
    ///
    /// # Returns
    ///
    /// * If successful, returns a `storage::GatewayStatusMap`.
    /// * If an error occurs, returns a `errors::StorageError`.
    ///
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

        /// This method updates a GSM rule in the storage. It takes in the connector, flow, sub_flow, code, message, and data of the rule to be updated, and returns a result indicating whether the update was successful along with the updated gateway status map or an error if the update failed.
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

        /// Asynchronously deletes a GSM rule from the database.
    ///
    /// # Arguments
    ///
    /// * `_connector` - The connector associated with the rule.
    /// * `_flow` - The flow associated with the rule.
    /// * `_sub_flow` - The sub-flow associated with the rule.
    /// * `_code` - The code associated with the rule.
    /// * `_message` - The message associated with the rule.
    ///
    /// # Returns
    ///
    /// A custom result containing a boolean value indicating the success of the deletion operation or a storage error.
    ///
    /// # Errors
    ///
    /// This method returns a `StorageError::MockDbError` if the deletion operation encounters an error.
    ///
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
