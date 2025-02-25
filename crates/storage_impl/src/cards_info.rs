use common_utils::errors::CustomResult;
use diesel_models::cards_info as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::cards_info::CardsInfoInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> CardsInfoInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<storage::CardInfo>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}
