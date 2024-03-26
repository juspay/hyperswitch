#[cfg(feature = "olap")]
use analytics::health_check::HealthCheck;
use api_models::health_check::HealthState;
use error_stack::ResultExt;
use router_env::logger;

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
    async fn health_check_outgoing(&self)
        -> CustomResult<HealthState, errors::HealthCheckOutGoing>;
    #[cfg(feature = "olap")]
    async fn health_check_analytics(&self)
        -> CustomResult<HealthState, errors::HealthCheckDBError>;
}

#[async_trait::async_trait]
impl HealthCheckInterface for app::AppState {
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
            .serialize_and_set_key_with_expiry("test_key", "test_value", 30)
            .await
            .change_context(errors::HealthCheckRedisError::SetFailed)?;

        logger::debug!("Redis set_key was successful");

        redis_conn
            .get_key("test_key")
            .await
            .change_context(errors::HealthCheckRedisError::GetFailed)?;

        logger::debug!("Redis get_key was successful");

        redis_conn
            .delete_key("test_key")
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
            let mut url = locker.host_rs.to_owned();
            url.push_str(consts::LOCKER_HEALTH_CALL_PATH);
            let request = services::Request::new(services::Method::Get, &url);
            services::call_connector_api(self, request, "health_check_for_locker")
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

    async fn health_check_outgoing(
        &self,
    ) -> CustomResult<HealthState, errors::HealthCheckOutGoing> {
        let request = services::Request::new(services::Method::Get, consts::OUTGOING_CALL_URL);
        services::call_connector_api(self, request, "outgoing_health_check")
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
}
