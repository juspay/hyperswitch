use diesel_models::gsm as storage;
use error_stack::{report, ResultExt};
use router_env::{instrument, logger};
use storage_impl::redis::cache::{self, CacheId, CacheKind};

use super::MockDb;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    services::Store,
};

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
        let gsm_db_record = diesel_models::gsm::GatewayStatusMappingNew::try_from(rule)
            .change_context(errors::StorageError::SerializationFailed)
            .attach_printable("Failed to convert gsm domain models to diesel models")?
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))?;

        // The absence of this rule may be cached.
        clear_cached_gsm_rule(
            self,
            gsm_cache_key(
                &gsm_db_record.connector,
                &gsm_db_record.flow,
                &gsm_db_record.sub_flow,
                &gsm_db_record.code,
                &gsm_db_record.message,
            ),
        )
        .await;

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
        let find_rule_in_db = || async {
            let conn = connection::pg_connection_read(self).await?;
            match storage::GatewayStatusMap::find(
                &conn,
                connector.clone(),
                flow.clone(),
                sub_flow.clone(),
                code.clone(),
                message.clone(),
            )
            .await
            {
                Ok(gsm_db_record) => {
                    hyperswitch_domain_models::gsm::GatewayStatusMap::try_from(gsm_db_record)
                        .map(Some)
                        .change_context(errors::StorageError::DeserializationFailed)
                        .attach_printable("Failed to convert gsm diesel models to domain models")
                }
                Err(error)
                    if matches!(
                        error.current_context(),
                        diesel_models::errors::DatabaseError::NotFound
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(report!(errors::StorageError::from(error))),
            }
        };

        // Most lookups are for an error no rule is configured for, so the absence of a rule is
        // cached as well as a rule.
        let rule = Box::pin(cache::get_or_populate_in_memory_redis(
            self,
            &key,
            find_rule_in_db(),
            CacheId::Gsm,
        ))
        .await?;

        rule.ok_or_else(|| {
            report!(errors::StorageError::ValueNotFound(
                "No gsm rule is configured for the given key".to_string()
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
        let key = gsm_cache_key(&connector, &flow, &sub_flow, &code, &message);
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
        clear_cached_gsm_rule(self, key).await;

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
        let key = gsm_cache_key(&connector, &flow, &sub_flow, &code, &message);
        let conn = connection::pg_connection_write(self).await?;
        let is_deleted =
            storage::GatewayStatusMap::delete(&conn, connector, flow, sub_flow, code, message)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))?;
        clear_cached_gsm_rule(self, key).await;
        Ok(is_deleted)
    }
}

/// Clears a gsm rule from the caches of every instance after it was written. The rule is written
/// by then, so a failure to clear it is logged instead of failing the write, which did not depend
/// on redis before the rules were cached; the entry then lapses with its time to live.
async fn clear_cached_gsm_rule(store: &Store, key: String) {
    if let Err(error) =
        cache::redact_from_redis_and_publish(store, [CacheKind::Gsm(key.into())]).await
    {
        logger::error!(
            ?error,
            "Failed to clear a written gsm rule from the gsm cache"
        );
    }
}

/// The cache key of the gsm rule for a connector error. Its parts are length prefixed, so that
/// no two keys of a rule run into each other whatever the text of the error is.
fn gsm_cache_key(connector: &str, flow: &str, sub_flow: &str, code: &str, message: &str) -> String {
    [connector, flow, sub_flow, code, message]
        .iter()
        .fold(String::from("gsm"), |key, part| {
            format!("{key}_{}:{part}", part.len())
        })
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

#[cfg(test)]
mod tests {
    use super::gsm_cache_key;

    #[test]
    fn gsm_cache_keys_of_different_rules_differ() {
        assert_ne!(
            gsm_cache_key("stripe", "payment", "authorize", "card_declined", "a_b"),
            gsm_cache_key("stripe", "payment", "authorize", "card_declined_a", "b"),
        );
        assert_eq!(
            gsm_cache_key("stripe", "payment", "authorize", "51", "Insufficient funds"),
            "gsm_6:stripe_7:payment_9:authorize_2:51_18:Insufficient funds",
        );
    }
}
