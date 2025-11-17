use crate::{connection, errors, kv_router_store, DatabaseStore, MockDb, RouterStore};
use common_utils::errors::CustomResult;
use diesel_models::locker_mock_up as storage;
use error_stack::report;
use hyperswitch_domain_models::locker_mock_up::LockerMockUpInterface;
use router_env::{instrument, tracing};

#[async_trait::async_trait]
impl<T: DatabaseStore> LockerMockUpInterface for kv_router_store::KVRouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.router_store.find_locker_by_card_id(card_id).await
    }

    #[instrument(skip_all)]
    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.router_store.insert_locker_mock_up(new).await
    }

    #[instrument(skip_all)]
    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.router_store.delete_locker_mock_up(card_id).await
    }
}

#[async_trait::async_trait]
impl<T: DatabaseStore> LockerMockUpInterface for RouterStore<T> {
    type Error = errors::StorageError;
    #[instrument(skip_all)]
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::LockerMockUp::find_by_card_id(&conn, card_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::LockerMockUp::delete_by_card_id(&conn, card_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl LockerMockUpInterface for MockDb {
    type Error = errors::StorageError;
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        self.lockers
            .lock()
            .await
            .iter()
            .find(|l| l.card_id == card_id)
            .cloned()
            .ok_or(errors::StorageError::MockDbError.into())
    }

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let mut locked_lockers = self.lockers.lock().await;

        if locked_lockers.iter().any(|l| l.card_id == new.card_id) {
            Err(errors::StorageError::MockDbError)?;
        }

        let created_locker = storage::LockerMockUp {
            card_id: new.card_id,
            external_id: new.external_id,
            card_fingerprint: new.card_fingerprint,
            card_global_fingerprint: new.card_global_fingerprint,
            merchant_id: new.merchant_id,
            card_number: new.card_number,
            card_exp_year: new.card_exp_year,
            card_exp_month: new.card_exp_month,
            name_on_card: new.name_on_card,
            nickname: None,
            customer_id: new.customer_id,
            duplicate: None,
            card_cvc: new.card_cvc,
            payment_method_id: new.payment_method_id,
            enc_card_data: new.enc_card_data,
        };

        locked_lockers.push(created_locker.clone());

        Ok(created_locker)
    }

    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let mut locked_lockers = self.lockers.lock().await;

        let position = locked_lockers
            .iter()
            .position(|l| l.card_id == card_id)
            .ok_or(errors::StorageError::MockDbError)?;

        Ok(locked_lockers.remove(position))
    }
}
