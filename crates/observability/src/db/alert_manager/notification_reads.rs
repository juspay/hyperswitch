use diesel_models::{observability::alert_manager::notification_reads as storage, StorageResult};

use crate::{db::Store, domain_models::alert_manager::notification_reads as domain_models};

#[async_trait::async_trait]
pub trait NotificationReadsInterface {
    async fn upsert_notification_read(
        &self,
        new: domain_models::NotificationReadsNew,
    ) -> StorageResult<domain_models::NotificationReads>;

    async fn find_notification_read_by_user_name(
        &self,
        user_name: &str,
    ) -> StorageResult<domain_models::NotificationReads>;
}

#[async_trait::async_trait]
impl NotificationReadsInterface for Store {
    async fn upsert_notification_read(
        &self,
        new: domain_models::NotificationReadsNew,
    ) -> StorageResult<domain_models::NotificationReads> {
        let connection = self.connection().await?;

        storage::NotificationReadsNew::from(new)
            .upsert(&connection)
            .await
            .map(domain_models::NotificationReads::from)
    }

    async fn find_notification_read_by_user_name(
        &self,
        user_name: &str,
    ) -> StorageResult<domain_models::NotificationReads> {
        let connection = self.connection().await?;

        storage::NotificationReads::find_by_user_name(&connection, user_name)
            .await
            .map(domain_models::NotificationReads::from)
    }
}
