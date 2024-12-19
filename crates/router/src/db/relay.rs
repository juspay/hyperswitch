use diesel_models;

use crate::core::errors::{self, CustomResult};

#[async_trait::async_trait]
pub trait RelayInterface {
    async fn insert_relay(
        &self,
        new: diesel_models::relay::RelayNew,
    ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError>;

    async fn update_relay(
        &self,
        this: diesel_models::relay::Relay,
        relay: diesel_models::relay::RelayUpdate,
    ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError>;

    async fn find_relay_by_id(
        &self,
        id: &str,
    ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError>;
}

mod storage {
    use error_stack::report;

    use super::RelayInterface;
    use crate::{
        connection,
        core::errors::{self, CustomResult},
        services::Store,
    };

    #[async_trait::async_trait]
    impl RelayInterface for Store {
        async fn insert_relay(
            &self,
            new: diesel_models::relay::RelayNew,
        ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            new.insert(&conn)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        async fn update_relay(
            &self,
            this: diesel_models::relay::Relay,
            relay: diesel_models::relay::RelayUpdate,
        ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError> {
            let conn = connection::pg_connection_write(self).await?;
            this.update(&conn, relay)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }

        async fn find_relay_by_id(
            &self,
            id: &str,
        ) -> CustomResult<diesel_models::relay::Relay, errors::StorageError> {
            let conn = connection::pg_connection_read(self).await?;
            diesel_models::relay::Relay::find_by_id(&conn, id)
                .await
                .map_err(|error| report!(errors::StorageError::from(error)))
        }
    }
}
