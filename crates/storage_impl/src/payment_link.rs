use common_utils::errors::CustomResult;
use diesel_models::payment_link as storage;
use error_stack::report;
use router_env::{instrument, tracing};
use sample::payment_link::PaymentLinkInterface;

use crate::{connection, errors, DatabaseStore, RouterStore};

#[async_trait::async_trait]
impl<T: DatabaseStore> PaymentLinkInterface for RouterStore<T> {
    type Error = errors::StorageError;

    #[instrument(skip_all)]
    async fn find_payment_link_by_payment_link_id(
        &self,
        payment_link_id: &str,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::PaymentLink::find_link_by_payment_link_id(&conn, payment_link_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_payment_link(
        &self,
        payment_link_config: storage::PaymentLinkNew,
    ) -> CustomResult<storage::PaymentLink, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        payment_link_config
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    //     #[instrument(skip_all)]
    //     async fn list_payment_link_by_merchant_id(
    //         &self,
    //         merchant_id: &common_utils::id_type::MerchantId,
    //         payment_link_constraints: api_models::payments::PaymentLinkListConstraints,
    //     ) -> CustomResult<Vec<storage::PaymentLink>, errors::StorageError> {
    //         let conn = connection::pg_connection_read(self).await?;
    //         storage::PaymentLink::filter_by_constraints(&conn, merchant_id, payment_link_constraints)
    //             .await
    //             .map_err(|error| report!(errors::StorageError::from(error)))
    //     }
}
