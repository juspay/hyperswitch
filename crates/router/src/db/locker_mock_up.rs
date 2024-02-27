use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage,
};

#[async_trait::async_trait]
pub trait LockerMockUpInterface {
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError>;

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError>;

    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError>;
}

#[async_trait::async_trait]
impl LockerMockUpInterface for Store {
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::LockerMockUp::find_locker_by_card_id(&conn, card_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert_locker_mock_up(&conn).await.map_err(Into::into).into_report()
    }

    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::LockerMockUp::delete_locker_by_card_id(&conn, card_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl LockerMockUpInterface for MockDb {
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
            id: locked_lockers
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
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

#[cfg(test)]
mod tests {
    #[allow(clippy::unwrap_used)]
    mod mockdb_locker_mock_up_interface {
        use crate::{
            db::{locker_mock_up::LockerMockUpInterface, MockDb},
            types::storage,
        };

        pub struct LockerMockUpIds {
            card_id: String,
            external_id: String,
            merchant_id: String,
            customer_id: String,
        }

        fn create_locker_mock_up_new(locker_ids: LockerMockUpIds) -> storage::LockerMockUpNew {
            storage::LockerMockUpNew {
                card_id: locker_ids.card_id,
                external_id: locker_ids.external_id,
                card_fingerprint: "card_fingerprint".into(),
                card_global_fingerprint: "card_global_fingerprint".into(),
                merchant_id: locker_ids.merchant_id,
                card_number: "1234123412341234".into(),
                card_exp_year: "2023".into(),
                card_exp_month: "06".into(),
                name_on_card: Some("name_on_card".into()),
                card_cvc: Some("123".into()),
                payment_method_id: Some("payment_method_id".into()),
                customer_id: Some(locker_ids.customer_id),
                nickname: Some("card_holder_nickname".into()),
                enc_card_data: Some("enc_card_data".into()),
            }
        }

        #[tokio::test]
        async fn find_locker_by_card_id() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_locker = mockdb
                .insert_locker_mock_up(create_locker_mock_up_new(LockerMockUpIds {
                    card_id: "card_1".into(),
                    external_id: "external_1".into(),
                    merchant_id: "merchant_1".into(),
                    customer_id: "customer_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_locker_mock_up(create_locker_mock_up_new(LockerMockUpIds {
                    card_id: "card_2".into(),
                    external_id: "external_1".into(),
                    merchant_id: "merchant_1".into(),
                    customer_id: "customer_1".into(),
                }))
                .await;

            let found_locker = mockdb.find_locker_by_card_id("card_1").await.unwrap();

            assert_eq!(created_locker, found_locker)
        }

        #[tokio::test]
        async fn insert_locker_mock_up() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_locker = mockdb
                .insert_locker_mock_up(create_locker_mock_up_new(LockerMockUpIds {
                    card_id: "card_1".into(),
                    external_id: "external_1".into(),
                    merchant_id: "merchant_1".into(),
                    customer_id: "customer_1".into(),
                }))
                .await
                .unwrap();

            let found_locker = mockdb
                .lockers
                .lock()
                .await
                .iter()
                .find(|l| l.card_id == "card_1")
                .cloned();

            assert!(found_locker.is_some());

            assert_eq!(created_locker, found_locker.unwrap())
        }

        #[tokio::test]
        async fn delete_locker_mock_up() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_locker = mockdb
                .insert_locker_mock_up(create_locker_mock_up_new(LockerMockUpIds {
                    card_id: "card_1".into(),
                    external_id: "external_1".into(),
                    merchant_id: "merchant_1".into(),
                    customer_id: "customer_1".into(),
                }))
                .await
                .unwrap();

            let deleted_locker = mockdb.delete_locker_mock_up("card_1").await.unwrap();

            assert_eq!(created_locker, deleted_locker);

            let exist = mockdb
                .lockers
                .lock()
                .await
                .iter()
                .any(|l| l.card_id == "card_1");

            assert!(!exist)
        }
    }
}
