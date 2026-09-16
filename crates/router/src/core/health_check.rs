#[cfg(feature = "olap")]
use analytics::health_check::HealthCheck;
#[cfg(feature = "dynamic_routing")]
use api_models::health_check::HealthCheckMap;
use api_models::health_check::HealthState;
use common_utils::{
    crypto::{GenerateDigest, Sha256},
    fp_utils,
};
use error_stack::ResultExt;
use external_services::managers::secrets_management::SecretsManagementConfig;
use hyperswitch_masking::PeekInterface;
use router_env::logger;
use subtle::ConstantTimeEq;

use crate::{
    consts,
    core::errors::{self, CustomResult},
    routes::app,
    services::api as services,
};

#[async_trait::async_trait]
pub trait HealthCheckInterface {
    async fn health_check_db(&self) -> CustomResult<HealthState, errors::HealthCheckDBError>;
    async fn health_check_redis(&self) -> CustomResult<HealthState, errors::HealthCheckRedisError>;
    async fn health_check_locker(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckLockerError>;
    async fn health_check_kms(&self) -> CustomResult<HealthState, errors::HealthCheckKmsError>;
    async fn health_check_encryption_service(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckEncryptionServiceError>;
    async fn health_check_outgoing(&self)
        -> CustomResult<HealthState, errors::HealthCheckOutGoing>;
    #[cfg(feature = "olap")]
    async fn health_check_analytics(&self)
        -> CustomResult<HealthState, errors::HealthCheckDBError>;

    #[cfg(feature = "olap")]
    async fn health_check_opensearch(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckDBError>;

    #[cfg(feature = "dynamic_routing")]
    async fn health_check_grpc(
        &self,
    ) -> CustomResult<HealthCheckMap, errors::HealthCheckGRPCServiceError>;

    #[cfg(feature = "dynamic_routing")]
    async fn health_check_decision_engine(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckDecisionEngineError>;

    async fn health_check_unified_connector_service(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckUnifiedConnectorServiceError>;
}

#[async_trait::async_trait]
impl HealthCheckInterface for app::SessionState {
    async fn health_check_db(&self) -> CustomResult<HealthState, errors::HealthCheckDBError> {
        let db = &*self.store;
        db.health_check_db().await?;
        Ok(HealthState::Running)
    }

    async fn health_check_redis(&self) -> CustomResult<HealthState, errors::HealthCheckRedisError> {
        let db = &*self.store;
        let redis_conn = db
            .get_redis_conn()
            .change_context(errors::HealthCheckRedisError::RedisConnectionError)?;

        redis_conn
            .serialize_and_set_key_with_expiry(&"test_key".into(), "test_value", 30)
            .await
            .change_context(errors::HealthCheckRedisError::SetFailed)?;

        logger::debug!("Redis set_key was successful");

        redis_conn
            .get_key::<()>(&"test_key".into())
            .await
            .change_context(errors::HealthCheckRedisError::GetFailed)?;

        logger::debug!("Redis get_key was successful");

        redis_conn
            .delete_key(&"test_key".into())
            .await
            .change_context(errors::HealthCheckRedisError::DeleteFailed)?;

        logger::debug!("Redis delete_key was successful");

        Ok(HealthState::Running)
    }

    async fn health_check_locker(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckLockerError> {
        let locker = &self.conf.locker;
        if !locker.mock_locker {
            let mut url = locker.host.to_owned();
            url.push_str(consts::LOCKER_DEEP_HEALTH_CALL_PATH);
            let request = services::Request::new(services::Method::Get, &url);
            services::call_connector_api(self, request, "health_check_for_locker", None)
                .await
                .change_context(errors::HealthCheckLockerError::FailedToCallLocker)?
                .map_err(|_| {
                    error_stack::report!(errors::HealthCheckLockerError::FailedToCallLocker)
                })?;
            Ok(HealthState::Running)
        } else {
            Ok(HealthState::NotApplicable)
        }
    }

    async fn health_check_kms(&self) -> CustomResult<HealthState, errors::HealthCheckKmsError> {
        if matches!(
            self.conf.secrets_management,
            SecretsManagementConfig::NoEncryption
        ) {
            logger::debug!("KMS health check not applicable; secrets manager is no_encryption");
            Ok(HealthState::NotApplicable)
        } else {
            let decrypted = self
                .secret_management_client
                .get_secret(self.kms_health_check_probe.clone())
                .await
                .change_context(errors::HealthCheckKmsError::FailedToDecrypt)?;

            logger::debug!("KMS decrypt call succeeded, verifying the decrypted value");

            fp_utils::when(decrypted.peek().trim().is_empty(), || {
                Err(error_stack::report!(
                    errors::HealthCheckKmsError::EmptySecret
                ))
            })?;

            let expected = self.conf.secrets.get_inner().admin_api_key.peek();
            let expected_digest = Sha256
                .generate_digest(expected.as_bytes())
                .change_context(errors::HealthCheckKmsError::DigestFailed)?;
            let actual_digest = Sha256
                .generate_digest(decrypted.peek().as_bytes())
                .change_context(errors::HealthCheckKmsError::DigestFailed)?;

            let secrets_match = bool::from(expected_digest.ct_eq(&actual_digest));
            fp_utils::when(!secrets_match, || {
                Err(error_stack::report!(
                    errors::HealthCheckKmsError::SecretMismatch
                ))
            })?;

            logger::debug!("KMS decrypt health check successful");
            Ok(HealthState::Running)
        }
    }

    async fn health_check_encryption_service(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckEncryptionServiceError> {
        let key_manager = self.conf.key_manager.get_inner();
        if cfg!(feature = "encryption_service") && key_manager.enabled {
            let mut url = key_manager.url.clone();
            url.push_str(consts::ENCRYPTION_SERVICE_HEALTH_CALL_PATH);
            let request = services::Request::new(services::Method::Get, &url);
            services::call_connector_api(
                self,
                request,
                "health_check_for_encryption_service",
                None,
            )
            .await
            .change_context(
                errors::HealthCheckEncryptionServiceError::FailedToCallEncryptionService,
            )?
            .map_err(|_| {
                error_stack::report!(
                    errors::HealthCheckEncryptionServiceError::FailedToCallEncryptionService
                )
            })?;
            Ok(HealthState::Running)
        } else {
            logger::debug!("Encryption service is disabled, skipping its health check");
            Ok(HealthState::NotApplicable)
        }
    }

    #[cfg(feature = "olap")]
    async fn health_check_analytics(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckDBError> {
        let analytics = &self.pool;
        match analytics {
            analytics::AnalyticsProvider::Sqlx(client) => client
                .deep_health_check()
                .await
                .change_context(errors::HealthCheckDBError::SqlxAnalyticsError),
            analytics::AnalyticsProvider::Clickhouse(client) => client
                .deep_health_check()
                .await
                .change_context(errors::HealthCheckDBError::ClickhouseAnalyticsError),
            analytics::AnalyticsProvider::CombinedCkh(sqlx_client, ckh_client) => {
                sqlx_client
                    .deep_health_check()
                    .await
                    .change_context(errors::HealthCheckDBError::SqlxAnalyticsError)?;
                ckh_client
                    .deep_health_check()
                    .await
                    .change_context(errors::HealthCheckDBError::ClickhouseAnalyticsError)
            }
            analytics::AnalyticsProvider::CombinedSqlx(sqlx_client, ckh_client) => {
                sqlx_client
                    .deep_health_check()
                    .await
                    .change_context(errors::HealthCheckDBError::SqlxAnalyticsError)?;
                ckh_client
                    .deep_health_check()
                    .await
                    .change_context(errors::HealthCheckDBError::ClickhouseAnalyticsError)
            }
        }?;

        Ok(HealthState::Running)
    }

    #[cfg(feature = "olap")]
    async fn health_check_opensearch(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckDBError> {
        if let Some(client) = self.opensearch_client.as_ref() {
            client
                .deep_health_check()
                .await
                .change_context(errors::HealthCheckDBError::OpensearchError)?;
            Ok(HealthState::Running)
        } else {
            Ok(HealthState::NotApplicable)
        }
    }

    async fn health_check_outgoing(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckOutGoing> {
        let request = services::Request::new(services::Method::Get, consts::OUTGOING_CALL_URL);
        services::call_connector_api(self, request, "outgoing_health_check", None)
            .await
            .map_err(|err| errors::HealthCheckOutGoing::OutGoingFailed {
                message: err.to_string(),
            })?
            .map_err(|err| errors::HealthCheckOutGoing::OutGoingFailed {
                message: format!(
                    "Got a non 200 status while making outgoing request. Error {:?}",
                    err.response
                ),
            })?;

        logger::debug!("Outgoing request successful");
        Ok(HealthState::Running)
    }

    #[cfg(feature = "dynamic_routing")]
    async fn health_check_grpc(
        &self,
    ) -> CustomResult<HealthCheckMap, errors::HealthCheckGRPCServiceError> {
        let health_client = &self.grpc_client.health_client;
        let grpc_config = &self.conf.grpc_client;

        let health_check_map = health_client
            .perform_health_check(grpc_config)
            .await
            .change_context(errors::HealthCheckGRPCServiceError::FailedToCallService)?;

        logger::debug!("Health check successful");
        Ok(health_check_map)
    }

    #[cfg(feature = "dynamic_routing")]
    async fn health_check_decision_engine(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckDecisionEngineError> {
        if self.conf.open_router.dynamic_routing_enabled {
            let url = format!("{}/{}", self.conf.open_router.url, "health");
            let request = services::Request::new(services::Method::Get, &url);
            let _ = services::call_connector_api(
                self,
                request,
                "health_check_for_decision_engine",
                None,
            )
            .await
            .change_context(
                errors::HealthCheckDecisionEngineError::FailedToCallDecisionEngineService,
            )?;

            logger::debug!("Decision engine health check successful");
            Ok(HealthState::Running)
        } else {
            logger::debug!("Decision engine health check not applicable");
            Ok(HealthState::NotApplicable)
        }
    }

    async fn health_check_unified_connector_service(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckUnifiedConnectorServiceError> {
        if let Some(_ucs_client) = &self.grpc_client.unified_connector_service_client {
            // For now, we'll just check if the client exists and is configured
            // In the future, this could be enhanced to make an actual health check call
            // to the unified connector service if it supports health check endpoints
            logger::debug!("Unified Connector Service client is configured and available");
            Ok(HealthState::Running)
        } else {
            logger::debug!("Unified Connector Service client not configured");
            Ok(HealthState::NotApplicable)
        }
    }
}
