// use error_stack::report;
// use router_env::{instrument, tracing};

// use crate::{
//     connection,
//     core::errors::{self, CustomResult},
//     db::MockDb,
//     services::Store,
//     types::storage::cards_info::CardInfo,
// };

use common_utils::errors::CustomResult;
// use hyperswitch_domain_models::errors;
use diesel_models::cards_info as storage;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait CardsInfoInterface {
    type Error;
    async fn get_card_info(
        &self,
        _card_iin: &str,
    ) -> CustomResult<Option<storage::CardInfo>, Self::Error>;
}

// #[async_trait::async_trait]
// impl CardsInfoInterface for MockDb {
//     #[instrument(skip_all)]
//     async fn get_card_info(
//         &self,
//         card_iin: &str,
//     ) -> CustomResult<Option<CardInfo>, errors::StorageError> {
//         Ok(self
//             .cards_info
//             .lock()
//             .await
//             .iter()
//             .find(|ci| ci.card_iin == card_iin)
//             .cloned())
//     }
// }
