use diesel_models::profile_acquirer::{ProfileAcquirer, ProfileAcquirerNew};
use error_stack::report;
use router_env::{instrument, tracing};

use super::Store;
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MockDb,
};

#[async_trait::async_trait]
pub trait ProfileAcquirerInterface {
    async fn insert_profile_acquirer(
        &self,
        new_acquirer: ProfileAcquirerNew,
    ) -> CustomResult<ProfileAcquirer, errors::StorageError>;

    async fn list_profile_acquirer_based_on_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Vec<ProfileAcquirer>, errors::StorageError>;
}

#[async_trait::async_trait]
impl ProfileAcquirerInterface for Store {
    #[instrument(skip_all)]
    async fn insert_profile_acquirer(
        &self,
        new_acquirer: ProfileAcquirerNew,
    ) -> CustomResult<ProfileAcquirer, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        new_acquirer
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn list_profile_acquirer_based_on_profile_id(
        &self,
        profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Vec<ProfileAcquirer>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        ProfileAcquirer::list_by_profile_id(&conn, profile_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl ProfileAcquirerInterface for MockDb {
    #[instrument(skip_all)]
    async fn insert_profile_acquirer(
        &self,
        new_acquirer: ProfileAcquirerNew,
    ) -> CustomResult<ProfileAcquirer, errors::StorageError> {
        let now = common_utils::date_time::now();
        let acquirer = ProfileAcquirer {
            profile_acquirer_id: new_acquirer.profile_acquirer_id,
            acquirer_assigned_merchant_id: new_acquirer.acquirer_assigned_merchant_id,
            merchant_name: new_acquirer.merchant_name,
            mcc: new_acquirer.mcc,
            merchant_country_code: new_acquirer.merchant_country_code,
            network: new_acquirer.network,
            acquirer_bin: new_acquirer.acquirer_bin,
            acquirer_ica: new_acquirer.acquirer_ica,
            acquirer_fraud_rate: new_acquirer.acquirer_fraud_rate,
            created_at: new_acquirer.created_at.unwrap_or(now),
            last_modified_at: new_acquirer.last_modified_at.unwrap_or(now),
            profile_id: new_acquirer.profile_id,
        };

        let mut profile_acquirers = self.profile_acquirers.lock().await;
        if profile_acquirers
            .iter()
            .any(|ma| ma.profile_acquirer_id == acquirer.profile_acquirer_id)
        {
            Err(errors::StorageError::DuplicateValue {
                entity: "profile_acquirer",
                key: Some(acquirer.profile_acquirer_id.get_string_repr().to_string()),
            }
            .into())
        } else {
            profile_acquirers.push(acquirer.clone());
            Ok(acquirer)
        }
    }

    async fn list_profile_acquirer_based_on_profile_id(
        &self,
        _profile_id: &common_utils::id_type::ProfileId,
    ) -> CustomResult<Vec<ProfileAcquirer>, errors::StorageError> {
        // TODO: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
