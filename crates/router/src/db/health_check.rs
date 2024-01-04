use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use diesel_models::ConfigNew;
use error_stack::ResultExt;
use router_env::logger;

use super::{MockDb, StorageInterface, Store};
use crate::{
    connection,
    consts::LOCKER_HEALTH_CALL_PATH,
    core::errors::{self, CustomResult},
    routes,
    services::api as services,
    types::storage,
};

#[async_trait::async_trait]
pub trait HealthCheckInterface {
    async fn health_check_db(&self) -> CustomResult<(), errors::HealthCheckDBError>;
    async fn health_check_redis(
        &self,
        db: &dyn StorageInterface,
    ) -> CustomResult<(), errors::HealthCheckRedisError>;
    async fn health_check_locker(
        &self,
        state: &routes::AppState,
    ) -> CustomResult<(), errors::HealthCheckLockerError>;
}

#[async_trait::async_trait]
impl HealthCheckInterface for Store {
    async fn health_check_db(&self) -> CustomResult<(), errors::HealthCheckDBError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::HealthCheckDBError::DBError)?;

        let _data = conn
            .transaction_async(|conn| {
                Box::pin(async move {
                    let query =
                        diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
                    let _x: i32 = query.get_result_async(&conn).await.map_err(|err| {
                        logger::error!(read_err=?err,"Error while reading element in the database");
                        errors::HealthCheckDBError::DBReadError
                    })?;

                    logger::debug!("Database read was successful");

                    let config = ConfigNew {
                        key: "test_key".to_string(),
                        config: "test_value".to_string(),
                    };

                    config.insert(&conn).await.map_err(|err| {
                        logger::error!(write_err=?err,"Error while writing to database");
                        errors::HealthCheckDBError::DBWriteError
                    })?;

                    logger::debug!("Database write was successful");

                    storage::Config::delete_by_key(&conn, "test_key").await.map_err(|err| {
                        logger::error!(delete_err=?err,"Error while deleting element in the database");
                        errors::HealthCheckDBError::DBDeleteError
                    })?;

                    logger::debug!("Database delete was successful");

                    Ok::<_, errors::HealthCheckDBError>(())
                })
            })
            .await?;

        Ok(())
    }

    async fn health_check_redis(
        &self,
        db: &dyn StorageInterface,
    ) -> CustomResult<(), errors::HealthCheckRedisError> {
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

        Ok(())
    }

    async fn health_check_locker(
        &self,
        state: &routes::AppState,
    ) -> CustomResult<(), errors::HealthCheckLockerError> {
        let locker = &state.conf.locker;
        if !locker.mock_locker {
            let mut url = locker.host_rs.to_owned();
            url.push_str(LOCKER_HEALTH_CALL_PATH);
            let request = services::Request::new(services::Method::Get, &url);
            services::call_connector_api(state, request)
                .await
                .change_context(errors::HealthCheckLockerError::FailedToCallLocker)?
                .ok();
        }

        logger::debug!("Locker call was successful");

        Ok(())
    }
}

#[async_trait::async_trait]
impl HealthCheckInterface for MockDb {
    async fn health_check_db(&self) -> CustomResult<(), errors::HealthCheckDBError> {
        Ok(())
    }

    async fn health_check_redis(
        &self,
        _: &dyn StorageInterface,
    ) -> CustomResult<(), errors::HealthCheckRedisError> {
        Ok(())
    }

    async fn health_check_locker(
        &self,
        _: &routes::AppState,
    ) -> CustomResult<(), errors::HealthCheckLockerError> {
        Ok(())
    }
}
