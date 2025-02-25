
// use error_stack::report;
// use router_env::{instrument, tracing};

// use super::MockDb;
// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     services::Store,
// };

// use hyperswitch_domain_models::errors;
use common_utils::errors::CustomResult;
use diesel_models::gsm as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait GsmInterface {
    type Error;
    async fn add_gsm_rule(
        &self,
        rule: storage::GatewayStatusMappingNew,
    ) -> CustomResult<storage::GatewayStatusMap, Self::Error>;
    async fn find_gsm_decision(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<String, Self::Error>;
    async fn find_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<storage::GatewayStatusMap, Self::Error>;
    async fn update_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
        data: storage::GatewayStatusMappingUpdate,
    ) -> CustomResult<storage::GatewayStatusMap, Self::Error>;

    async fn delete_gsm_rule(
        &self,
        connector: String,
        flow: String,
        sub_flow: String,
        code: String,
        message: String,
    ) -> CustomResult<bool, Self::Error>;
}

// #[async_trait::async_trait]
// impl GsmInterface for MockDb {
//     async fn add_gsm_rule(
//         &self,
//         _rule: storage::GatewayStatusMappingNew,
//     ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_gsm_decision(
//         &self,
//         _connector: String,
//         _flow: String,
//         _sub_flow: String,
//         _code: String,
//         _message: String,
//     ) -> CustomResult<String, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn find_gsm_rule(
//         &self,
//         _connector: String,
//         _flow: String,
//         _sub_flow: String,
//         _code: String,
//         _message: String,
//     ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn update_gsm_rule(
//         &self,
//         _connector: String,
//         _flow: String,
//         _sub_flow: String,
//         _code: String,
//         _message: String,
//         _data: storage::GatewayStatusMappingUpdate,
//     ) -> CustomResult<storage::GatewayStatusMap, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }

//     async fn delete_gsm_rule(
//         &self,
//         _connector: String,
//         _flow: String,
//         _sub_flow: String,
//         _code: String,
//         _message: String,
//     ) -> CustomResult<bool, errors::StorageError> {
//         Err(errors::StorageError::MockDbError)?
//     }
// }
