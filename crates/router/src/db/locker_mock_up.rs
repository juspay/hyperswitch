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
        /// Asynchronously finds a locker by the given card ID.
    ///
    /// # Arguments
    ///
    /// * `card_id` - A string slice representing the card ID to search for.
    ///
    /// # Returns
    ///
    /// A `CustomResult` that contains a `LockerMockUp` if the locker with the specified card ID is found, or a `StorageError` if an error occurs.
    ///
    async fn find_locker_by_card_id(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::LockerMockUp::find_by_card_id(&conn, card_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

        /// Inserts a new LockerMockUp entry into the storage using an asynchronous connection to the database. 
    /// Returns a CustomResult containing the inserted LockerMockUp entry or a StorageError if an error occurs.
    async fn insert_locker_mock_up(
        &self,
        new: storage::LockerMockUpNew,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new.insert(&conn).await.map_err(Into::into).into_report()
    }

        /// Asynchronously deletes a locker mock-up by its associated card ID.
    ///
    /// # Arguments
    ///
    /// * `card_id` - A string slice representing the ID of the card associated with the locker mock-up to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing a `LockerMockUp` if the deletion is successful, otherwise an `errors::StorageError`.
    ///
    async fn delete_locker_mock_up(
        &self,
        card_id: &str,
    ) -> CustomResult<storage::LockerMockUp, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        storage::LockerMockUp::delete_by_card_id(&conn, card_id)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl LockerMockUpInterface for MockDb {
        /// Asynchronously finds a locker by card id in the storage. 
    /// 
    /// # Arguments
    /// 
    /// * `card_id` - A reference to a string representing the card id to search for.
    /// 
    /// # Returns
    /// 
    /// Returns a `CustomResult` with the found `LockerMockUp` if the card id is found,
    /// otherwise returns a `StorageError` if the card id is not found or if there is a mock database error.
    /// 
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

        /// Inserts a new locker mock-up into the storage, ensuring that the card_id is unique. If the card_id already exists in the storage, it returns a StorageError. Otherwise, it creates a new LockerMockUp instance and adds it to the storage, returning the newly created LockerMockUp.
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

        /// Deletes a locker mock-up with the specified card ID from the storage.
    ///
    /// # Arguments
    ///
    /// * `card_id` - A string slice that represents the card ID of the locker mock-up to be deleted.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the deleted `LockerMockUp` if the operation is successful, otherwise an `errors::StorageError`.
    ///
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

                /// Creates a new LockerMockUpNew struct using the provided LockerMockUpIds.
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
                /// Asynchronously finds a locker by card ID using the mock database.
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
                /// Inserts a mock locker into the mock database, retrieves it, and asserts that it has been successfully inserted.
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
        /// Asynchronously deletes a mock locker from the mock database. This method creates a new mock database and inserts a new mock locker with specified details. It then deletes the mock locker with the specified card ID and asserts that it has been successfully deleted from the database.
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
