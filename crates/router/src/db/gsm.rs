use diesel_models::gsm as storage;
use error_stack::{report, ResultExt};
use router_env::instrument;
use storage_impl::redis::cache::{self, CacheKind};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

/// Cache key of a gateway status map row: the columns that identify it.
fn gsm_cache_key(connector: &str, flow: &str, sub_flow: &str, code: &str, message: &str) -> String {
    format!("gsm_{connector}_{flow}_{sub_flow}_{code}_{message}")
}

#[router_derive::track_async_interface]
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
        // Lookups cache a missing row too, so the new row's key is cleared once it is added.
        let key = gsm_cache_key(
            &rule.connector,
            &rule.flow,
            &rule.sub_flow,
            &rule.code,
            &rule.message,
        );
        let gsm_db_record = cache::publish_and_redact(self, CacheKind::Gsm(key.into()), || async {
            diesel_models::gsm::GatewayStatusMappingNew::try_from(rule)
                .change_context(errors::StorageError::SerializationFailed)
                .attach_printable("Failed to convert gsm domain models to diesel models")?
                .insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        })
        .await?;

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
        let key = gsm_cache_key(&connector, &flow, &sub_flow, &code, &message);
        // A missing row is cached as `None` too: callers look up the issuer code key first and
        // fall back to the error code, so misses are frequent and would otherwise reach the
        // database on every attempt.
        let find_call = || async {
            let conn = connection::pg_connection_read(self).await?;
            match storage::GatewayStatusMap::find(&conn, connector, flow, sub_flow, code, message)
                .await
            {
                Ok(gsm_db_record) => {
                    hyperswitch_domain_models::gsm::GatewayStatusMap::try_from(gsm_db_record)
                        .map(Some)
                        .change_context(errors::StorageError::DeserializationFailed)
                        .attach_printable("Failed to convert gsm diesel models to domain models")
                }
                Err(error)
                    if matches!(error.current_context(), errors::DatabaseError::NotFound) =>
                {
                    Ok(None)
                }
                Err(error) => Err(report!(errors::StorageError::from(error))),
            }
        };
        let gsm_record: Option<hyperswitch_domain_models::gsm::GatewayStatusMap> = Box::pin(
            cache::get_or_populate_in_memory_redis(self, &key, find_call(), cache::CacheId::Gsm),
        )
        .await?;
        gsm_record.ok_or_else(|| {
            report!(errors::StorageError::ValueNotFound(
                "gateway status map rule".to_string()
            ))
        })
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
        let key = gsm_cache_key(&connector, &flow, &sub_flow, &code, &message);
        let gsm_db_record = cache::publish_and_redact(self, CacheKind::Gsm(key.into()), || async {
            storage::GatewayStatusMap::update(
                &conn,
                connector,
                flow,
                sub_flow,
                code,
                message,
                gsm_update_data,
            )
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
        })
        .await?;

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
        let key = gsm_cache_key(&connector, &flow, &sub_flow, &code, &message);
        cache::publish_and_redact(self, CacheKind::Gsm(key.into()), || async {
            storage::GatewayStatusMap::delete(&conn, connector, flow, sub_flow, code, message)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        })
        .await
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
