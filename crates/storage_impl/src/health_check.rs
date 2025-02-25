use async_bb8_diesel::{AsyncConnection, AsyncRunQueryDsl};
use common_utils::errors::CustomResult;
use diesel_models::{self as storage, ConfigNew};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};
use sample::health_check::HealthCheckDbInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> HealthCheckDbInterface for RouterStore<T> {
    type Error = errors::HealthCheckDBError;

    #[instrument(skip_all)]
    async fn health_check_db(&self) -> CustomResult<(), errors::HealthCheckDBError> {
        let conn = connection::pg_connection_write(self)
            .await
            .change_context(errors::HealthCheckDBError::DBError)?;

        conn.transaction_async(|conn| async move {
            let query = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1 + 1"));
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

            storage::Config::delete_by_key(&conn, "test_key")
                .await
                .map_err(|err| {
                    logger::error!(delete_err=?err,"Error while deleting element in the database");
                    errors::HealthCheckDBError::DBDeleteError
                })?;

            logger::debug!("Database delete was successful");

            Ok::<_, errors::HealthCheckDBError>(())
        })
        .await?;

        Ok(())
    }
}
