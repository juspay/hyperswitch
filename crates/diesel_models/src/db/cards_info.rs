use common_utils::errors::{CustomResult};
use crate::services::{Store, MockDb};
use crate::{self as storage, CardInfo};
use crate::{domain::behaviour::Conversion, connection};
use crate::AddressNew;
use crate::address::AddressUpdateInternal;
use error_stack::{IntoReport, ResultExt};
use crate::{domain, errors};

#[async_trait::async_trait]
pub trait CardsInfoInterface {
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> CustomResult<Option<CardInfo>, errors::StorageError>;
}

#[async_trait::async_trait]
impl CardsInfoInterface for Store {
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
