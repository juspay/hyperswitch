use error_stack::IntoReport;
use router_env::{instrument, tracing};

use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
    services::Store,
    types::storage::cards_info::CardInfo,
};

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError>;
}

#[async_trait::async_trait]
impl CardsInfoInterface for Store {
    #[instrument(skip_all)]
        /// Asynchronously retrieves card information based on the provided card issuer identification number (IIN).
    /// 
    /// # Arguments
    /// 
    /// * `card_iin` - A string reference representing the card issuer identification number (IIN) to search for.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing either `Some(CardInfo)` if the card information is found, or `None` if no matching card information is found. Returns a `StorageError` if an error occurs during the retrieval process.
    /// 
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        CardInfo::find_by_iin(&conn, card_iin)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl CardsInfoInterface for MockDb {
    #[instrument(skip_all)]
        /// Asynchronously retrieves the card information for the given card IIN (Issuer Identification Number).
    /// Returns a result containing either the CardInfo associated with the card IIN, or a StorageError if there was an issue with the storage.
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
        Ok(self
            .cards_info
            .lock()
            .await
            .iter()
            .find(|ci| ci.card_iin == card_iin)
            .cloned())
    }
}
