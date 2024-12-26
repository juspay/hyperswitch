use std::collections::HashMap;

use error_stack::report;
use hyperswitch_domain_models::disputes;
use router_env::{instrument, tracing};

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage::{self, DisputeDbExt},
};

#[async_trait::async_trait]
pub trait DisputeInterface {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError>;

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_disputes_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_constraints: &disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn get_dispute_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::enums::DisputeStatus, i64)>, errors::StorageError>;
}

#[async_trait::async_trait]
impl DisputeInterface for Store {
    #[instrument(skip_all)]
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        dispute
            .insert(&conn)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id_connector_dispute_id(
            &conn,
            merchant_id,
            payment_id,
            connector_dispute_id,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_dispute_id(&conn, merchant_id, dispute_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_disputes_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_constraints: &disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::filter_by_constraints(&conn, merchant_id, dispute_constraints)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, dispute)
            .await
            .map_err(|error| report!(errors::StorageError::from(error)))
    }

    #[instrument(skip_all)]
    async fn get_dispute_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::DisputeStatus, i64)>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::get_dispute_status_with_count(
            &conn,
            merchant_id,
            profile_id_list,
            time_range,
        )
        .await
        .map_err(|error| report!(errors::StorageError::from(error)))
    }
}

#[async_trait::async_trait]
impl DisputeInterface for MockDb {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let evidence = dispute.evidence.ok_or(errors::StorageError::MockDbError)?;

        let mut locked_disputes = self.disputes.lock().await;

        if locked_disputes
            .iter()
            .any(|d| d.dispute_id == dispute.dispute_id)
        {
            Err(errors::StorageError::MockDbError)?;
        }

        let now = common_utils::date_time::now();

        let new_dispute = storage::Dispute {
            dispute_id: dispute.dispute_id,
            amount: dispute.amount,
            currency: dispute.currency,
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
            payment_id: dispute.payment_id,
            attempt_id: dispute.attempt_id,
            merchant_id: dispute.merchant_id,
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: now,
            modified_at: now,
            connector: dispute.connector,
            profile_id: dispute.profile_id,
            evidence,
            merchant_connector_id: dispute.merchant_connector_id,
            dispute_amount: dispute.dispute_amount,
            organization_id: dispute.organization_id,
            dispute_currency: dispute.dispute_currency,
        };

        locked_disputes.push(new_dispute.clone());

        Ok(new_dispute)
    }
    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        Ok(self
            .disputes
            .lock()
            .await
            .iter()
            .find(|d| {
                d.merchant_id == *merchant_id
                    && d.payment_id == *payment_id
                    && d.connector_dispute_id == connector_dispute_id
            })
            .cloned())
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        locked_disputes
            .iter()
            .find(|d| d.merchant_id == *merchant_id && d.dispute_id == dispute_id)
            .cloned()
            .ok_or(errors::StorageError::ValueNotFound(format!("No dispute available for merchant_id = {merchant_id:?} and dispute_id = {dispute_id}"))
            .into())
    }

    async fn find_disputes_by_constraints(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        dispute_constraints: &disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;
        let limit_usize = dispute_constraints
            .limit
            .unwrap_or(u32::MAX)
            .try_into()
            .unwrap_or(usize::MAX);
        let offset_usize = dispute_constraints
            .offset
            .unwrap_or(0)
            .try_into()
            .unwrap_or(usize::MIN);
        let filtered_disputes: Vec<storage::Dispute> = locked_disputes
            .iter()
            .filter(|dispute| {
                dispute.merchant_id == *merchant_id
                    && dispute_constraints
                        .dispute_id
                        .as_ref()
                        .map_or(true, |id| &dispute.dispute_id == id)
                    && dispute_constraints
                        .payment_id
                        .as_ref()
                        .map_or(true, |id| &dispute.payment_id == id)
                    && dispute_constraints
                        .profile_id
                        .as_ref()
                        .map_or(true, |profile_ids| {
                            dispute
                                .profile_id
                                .as_ref()
                                .map_or(true, |id| profile_ids.contains(id))
                        })
                    && dispute_constraints
                        .dispute_status
                        .as_ref()
                        .map_or(true, |statuses| statuses.contains(&dispute.dispute_status))
                    && dispute_constraints
                        .dispute_stage
                        .as_ref()
                        .map_or(true, |stages| stages.contains(&dispute.dispute_stage))
                    && dispute_constraints.reason.as_ref().map_or(true, |reason| {
                        dispute
                            .connector_reason
                            .as_ref()
                            .map_or(true, |d_reason| d_reason == reason)
                    })
                    && dispute_constraints
                        .connector
                        .as_ref()
                        .map_or(true, |connectors| {
                            connectors
                                .iter()
                                .any(|connector| dispute.connector.as_str() == *connector)
                        })
                    && dispute_constraints
                        .merchant_connector_id
                        .as_ref()
                        .map_or(true, |id| {
                            dispute.merchant_connector_id.as_ref() == Some(id)
                        })
                    && dispute_constraints
                        .currency
                        .as_ref()
                        .map_or(true, |currencies| {
                            currencies.iter().any(|currency| {
                                dispute
                                    .dispute_currency
                                    .map(|dispute_currency| &dispute_currency == currency)
                                    .unwrap_or(dispute.currency.as_str() == currency.to_string())
                            })
                        })
                    && dispute_constraints
                        .time_range
                        .as_ref()
                        .map_or(true, |range| {
                            let dispute_time = dispute.created_at;
                            dispute_time >= range.start_time
                                && range
                                    .end_time
                                    .map_or(true, |end_time| dispute_time <= end_time)
                        })
            })
            .skip(offset_usize)
            .take(limit_usize)
            .cloned()
            .collect();

        Ok(filtered_disputes)
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        payment_id: &common_utils::id_type::PaymentId,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        Ok(locked_disputes
            .iter()
            .filter(|d| d.merchant_id == *merchant_id && d.payment_id == *payment_id)
            .cloned()
            .collect())
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let mut locked_disputes = self.disputes.lock().await;

        let dispute_to_update = locked_disputes
            .iter_mut()
            .find(|d| d.dispute_id == this.dispute_id)
            .ok_or(errors::StorageError::MockDbError)?;

        let now = common_utils::date_time::now();

        match dispute {
            storage::DisputeUpdate::Update {
                dispute_stage,
                dispute_status,
                connector_status,
                connector_reason,
                connector_reason_code,
                challenge_required_by,
                connector_updated_at,
            } => {
                if connector_reason.is_some() {
                    dispute_to_update.connector_reason = connector_reason;
                }

                if connector_reason_code.is_some() {
                    dispute_to_update.connector_reason_code = connector_reason_code;
                }

                if challenge_required_by.is_some() {
                    dispute_to_update.challenge_required_by = challenge_required_by;
                }

                if connector_updated_at.is_some() {
                    dispute_to_update.connector_updated_at = connector_updated_at;
                }

                dispute_to_update.dispute_stage = dispute_stage;
                dispute_to_update.dispute_status = dispute_status;
                dispute_to_update.connector_status = connector_status;
            }
            storage::DisputeUpdate::StatusUpdate {
                dispute_status,
                connector_status,
            } => {
                if let Some(status) = connector_status {
                    dispute_to_update.connector_status = status;
                }
                dispute_to_update.dispute_status = dispute_status;
            }
            storage::DisputeUpdate::EvidenceUpdate { evidence } => {
                dispute_to_update.evidence = evidence;
            }
        }

        dispute_to_update.modified_at = now;

        Ok(dispute_to_update.clone())
    }

    async fn get_dispute_status_with_count(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        profile_id_list: Option<Vec<common_utils::id_type::ProfileId>>,
        time_range: &common_utils::types::TimeRange,
    ) -> CustomResult<Vec<(common_enums::DisputeStatus, i64)>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        let filtered_disputes_data = locked_disputes
            .iter()
            .filter(|d| {
                d.merchant_id == *merchant_id
                    && d.created_at >= time_range.start_time
                    && time_range
                        .end_time
                        .as_ref()
                        .map(|received_end_time| received_end_time >= &d.created_at)
                        .unwrap_or(true)
                    && profile_id_list
                        .as_ref()
                        .zip(d.profile_id.as_ref())
                        .map(|(received_profile_list, received_profile_id)| {
                            received_profile_list.contains(received_profile_id)
                        })
                        .unwrap_or(true)
            })
            .cloned()
            .collect::<Vec<storage::Dispute>>();

        Ok(filtered_disputes_data
            .into_iter()
            .fold(
                HashMap::new(),
                |mut acc: HashMap<common_enums::DisputeStatus, i64>, value| {
                    acc.entry(value.dispute_status)
                        .and_modify(|value| *value += 1)
                        .or_insert(1);
                    acc
                },
            )
            .into_iter()
            .collect::<Vec<(common_enums::DisputeStatus, i64)>>())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    mod mockdb_dispute_interface {
        use std::borrow::Cow;

        use diesel_models::{
            dispute::DisputeNew,
            enums::{DisputeStage, DisputeStatus},
        };
        use hyperswitch_domain_models::disputes::DisputeListConstraints;
        use masking::Secret;
        use redis_interface::RedisSettings;
        use serde_json::Value;
        use time::macros::datetime;

        use crate::db::{dispute::DisputeInterface, MockDb};

        pub struct DisputeNewIds {
            dispute_id: String,
            payment_id: common_utils::id_type::PaymentId,
            attempt_id: String,
            merchant_id: common_utils::id_type::MerchantId,
            connector_dispute_id: String,
        }

        fn create_dispute_new(dispute_ids: DisputeNewIds) -> DisputeNew {
            DisputeNew {
                dispute_id: dispute_ids.dispute_id,
                amount: "amount".into(),
                currency: "currency".into(),
                dispute_stage: DisputeStage::Dispute,
                dispute_status: DisputeStatus::DisputeOpened,
                payment_id: dispute_ids.payment_id,
                attempt_id: dispute_ids.attempt_id,
                merchant_id: dispute_ids.merchant_id,
                connector_status: "connector_status".into(),
                connector_dispute_id: dispute_ids.connector_dispute_id,
                connector_reason: Some("connector_reason".into()),
                connector_reason_code: Some("connector_reason_code".into()),
                challenge_required_by: Some(datetime!(2019-01-01 0:00)),
                connector_created_at: Some(datetime!(2019-01-02 0:00)),
                connector_updated_at: Some(datetime!(2019-01-03 0:00)),
                connector: "connector".into(),
                evidence: Some(Secret::from(Value::String("evidence".into()))),
                profile_id: Some(common_utils::generate_profile_id_of_default_length()),
                merchant_connector_id: None,
                dispute_amount: 1040,
                organization_id: common_utils::id_type::OrganizationId::default(),
                dispute_currency: Some(common_enums::Currency::default()),
            }
        }

        #[tokio::test]
        async fn test_insert_dispute() {
            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create a mock DB");

            let merchant_id =
                common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: common_utils::id_type::PaymentId::try_from(Cow::Borrowed(
                        "payment_1",
                    ))
                    .unwrap(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_dispute = mockdb
                .disputes
                .lock()
                .await
                .iter()
                .find(|d| d.dispute_id == created_dispute.dispute_id)
                .cloned();

            assert!(found_dispute.is_some());

            assert_eq!(created_dispute, found_dispute.unwrap());
        }

        #[tokio::test]
        async fn test_find_by_merchant_id_payment_id_connector_dispute_id() {
            let merchant_id =
                common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: common_utils::id_type::PaymentId::try_from(Cow::Borrowed(
                        "payment_1",
                    ))
                    .unwrap(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: common_utils::id_type::PaymentId::try_from(Cow::Borrowed(
                        "payment_1",
                    ))
                    .unwrap(),
                    connector_dispute_id: "connector_dispute_2".into(),
                }))
                .await
                .unwrap();

            let found_dispute = mockdb
                .find_by_merchant_id_payment_id_connector_dispute_id(
                    &merchant_id,
                    &common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1"))
                        .unwrap(),
                    "connector_dispute_1",
                )
                .await
                .unwrap();

            assert!(found_dispute.is_some());

            assert_eq!(created_dispute, found_dispute.unwrap());
        }

        #[tokio::test]
        async fn test_find_dispute_by_merchant_id_dispute_id() {
            let merchant_id =
                common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

            let payment_id =
                common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_dispute = mockdb
                .find_dispute_by_merchant_id_dispute_id(&merchant_id, "dispute_1")
                .await
                .unwrap();

            assert_eq!(created_dispute, found_dispute);
        }

        #[tokio::test]
        async fn test_find_disputes_by_merchant_id() {
            let merchant_id =
                common_utils::id_type::MerchantId::try_from(Cow::from("merchant_2")).unwrap();

            let payment_id =
                common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_disputes = mockdb
                .find_disputes_by_constraints(
                    &merchant_id,
                    &DisputeListConstraints {
                        dispute_id: None,
                        payment_id: None,
                        profile_id: None,
                        connector: None,
                        merchant_connector_id: None,
                        currency: None,
                        limit: None,
                        offset: None,
                        dispute_status: None,
                        dispute_stage: None,
                        reason: None,
                        time_range: None,
                    },
                )
                .await
                .unwrap();

            assert_eq!(1, found_disputes.len());

            assert_eq!(created_dispute, found_disputes.first().unwrap().clone());
        }

        #[tokio::test]
        async fn test_find_disputes_by_merchant_id_payment_id() {
            let merchant_id =
                common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

            let payment_id =
                common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: merchant_id.clone(),
                    payment_id: payment_id.clone(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_disputes = mockdb
                .find_disputes_by_merchant_id_payment_id(&merchant_id, &payment_id)
                .await
                .unwrap();

            assert_eq!(1, found_disputes.len());

            assert_eq!(created_dispute, found_disputes.first().unwrap().clone());
        }

        mod update_dispute {
            use std::borrow::Cow;

            use diesel_models::{
                dispute::DisputeUpdate,
                enums::{DisputeStage, DisputeStatus},
            };
            use masking::Secret;
            use serde_json::Value;
            use time::macros::datetime;

            use crate::db::{
                dispute::{
                    tests::mockdb_dispute_interface::{create_dispute_new, DisputeNewIds},
                    DisputeInterface,
                },
                MockDb,
            };

            #[tokio::test]
            async fn test_update_dispute_update() {
                let merchant_id =
                    common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

                let payment_id =
                    common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: merchant_id.clone(),
                        payment_id: payment_id.clone(),
                        connector_dispute_id: "connector_dispute_1".into(),
                    }))
                    .await
                    .unwrap();

                let updated_dispute = mockdb
                    .update_dispute(
                        created_dispute.clone(),
                        DisputeUpdate::Update {
                            dispute_stage: DisputeStage::PreDispute,
                            dispute_status: DisputeStatus::DisputeAccepted,
                            connector_status: "updated_connector_status".into(),
                            connector_reason: Some("updated_connector_reason".into()),
                            connector_reason_code: Some("updated_connector_reason_code".into()),
                            challenge_required_by: Some(datetime!(2019-01-10 0:00)),
                            connector_updated_at: Some(datetime!(2019-01-11 0:00)),
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(created_dispute.dispute_id, updated_dispute.dispute_id);
                assert_eq!(created_dispute.amount, updated_dispute.amount);
                assert_eq!(created_dispute.currency, updated_dispute.currency);
                assert_ne!(created_dispute.dispute_stage, updated_dispute.dispute_stage);
                assert_ne!(
                    created_dispute.dispute_status,
                    updated_dispute.dispute_status
                );
                assert_eq!(created_dispute.payment_id, updated_dispute.payment_id);
                assert_eq!(created_dispute.attempt_id, updated_dispute.attempt_id);
                assert_eq!(created_dispute.merchant_id, updated_dispute.merchant_id);
                assert_ne!(
                    created_dispute.connector_status,
                    updated_dispute.connector_status
                );
                assert_eq!(
                    created_dispute.connector_dispute_id,
                    updated_dispute.connector_dispute_id
                );
                assert_ne!(
                    created_dispute.connector_reason,
                    updated_dispute.connector_reason
                );
                assert_ne!(
                    created_dispute.connector_reason_code,
                    updated_dispute.connector_reason_code
                );
                assert_ne!(
                    created_dispute.challenge_required_by,
                    updated_dispute.challenge_required_by
                );
                assert_eq!(
                    created_dispute.connector_created_at,
                    updated_dispute.connector_created_at
                );
                assert_ne!(
                    created_dispute.connector_updated_at,
                    updated_dispute.connector_updated_at
                );
                assert_eq!(created_dispute.created_at, updated_dispute.created_at);
                assert_ne!(created_dispute.modified_at, updated_dispute.modified_at);
                assert_eq!(created_dispute.connector, updated_dispute.connector);
                assert_eq!(created_dispute.evidence, updated_dispute.evidence);
            }

            #[tokio::test]
            async fn test_update_dispute_update_status() {
                let merchant_id =
                    common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

                let payment_id =
                    common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: merchant_id.clone(),
                        payment_id: payment_id.clone(),
                        connector_dispute_id: "connector_dispute_1".into(),
                    }))
                    .await
                    .unwrap();

                let updated_dispute = mockdb
                    .update_dispute(
                        created_dispute.clone(),
                        DisputeUpdate::StatusUpdate {
                            dispute_status: DisputeStatus::DisputeExpired,
                            connector_status: Some("updated_connector_status".into()),
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(created_dispute.dispute_id, updated_dispute.dispute_id);
                assert_eq!(created_dispute.amount, updated_dispute.amount);
                assert_eq!(created_dispute.currency, updated_dispute.currency);
                assert_eq!(created_dispute.dispute_stage, updated_dispute.dispute_stage);
                assert_ne!(
                    created_dispute.dispute_status,
                    updated_dispute.dispute_status
                );
                assert_eq!(created_dispute.payment_id, updated_dispute.payment_id);
                assert_eq!(created_dispute.attempt_id, updated_dispute.attempt_id);
                assert_eq!(created_dispute.merchant_id, updated_dispute.merchant_id);
                assert_ne!(
                    created_dispute.connector_status,
                    updated_dispute.connector_status
                );
                assert_eq!(
                    created_dispute.connector_dispute_id,
                    updated_dispute.connector_dispute_id
                );
                assert_eq!(
                    created_dispute.connector_reason,
                    updated_dispute.connector_reason
                );
                assert_eq!(
                    created_dispute.connector_reason_code,
                    updated_dispute.connector_reason_code
                );
                assert_eq!(
                    created_dispute.challenge_required_by,
                    updated_dispute.challenge_required_by
                );
                assert_eq!(
                    created_dispute.connector_created_at,
                    updated_dispute.connector_created_at
                );
                assert_eq!(
                    created_dispute.connector_updated_at,
                    updated_dispute.connector_updated_at
                );
                assert_eq!(created_dispute.created_at, updated_dispute.created_at);
                assert_ne!(created_dispute.modified_at, updated_dispute.modified_at);
                assert_eq!(created_dispute.connector, updated_dispute.connector);
                assert_eq!(created_dispute.evidence, updated_dispute.evidence);
            }

            #[tokio::test]
            async fn test_update_dispute_update_evidence() {
                let merchant_id =
                    common_utils::id_type::MerchantId::try_from(Cow::from("merchant_1")).unwrap();

                let payment_id =
                    common_utils::id_type::PaymentId::try_from(Cow::Borrowed("payment_1")).unwrap();

                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: merchant_id.clone(),
                        payment_id: payment_id.clone(),
                        connector_dispute_id: "connector_dispute_1".into(),
                    }))
                    .await
                    .unwrap();

                let updated_dispute = mockdb
                    .update_dispute(
                        created_dispute.clone(),
                        DisputeUpdate::EvidenceUpdate {
                            evidence: Secret::from(Value::String("updated_evidence".into())),
                        },
                    )
                    .await
                    .unwrap();

                assert_eq!(created_dispute.dispute_id, updated_dispute.dispute_id);
                assert_eq!(created_dispute.amount, updated_dispute.amount);
                assert_eq!(created_dispute.currency, updated_dispute.currency);
                assert_eq!(created_dispute.dispute_stage, updated_dispute.dispute_stage);
                assert_eq!(
                    created_dispute.dispute_status,
                    updated_dispute.dispute_status
                );
                assert_eq!(created_dispute.payment_id, updated_dispute.payment_id);
                assert_eq!(created_dispute.attempt_id, updated_dispute.attempt_id);
                assert_eq!(created_dispute.merchant_id, updated_dispute.merchant_id);
                assert_eq!(
                    created_dispute.connector_status,
                    updated_dispute.connector_status
                );
                assert_eq!(
                    created_dispute.connector_dispute_id,
                    updated_dispute.connector_dispute_id
                );
                assert_eq!(
                    created_dispute.connector_reason,
                    updated_dispute.connector_reason
                );
                assert_eq!(
                    created_dispute.connector_reason_code,
                    updated_dispute.connector_reason_code
                );
                assert_eq!(
                    created_dispute.challenge_required_by,
                    updated_dispute.challenge_required_by
                );
                assert_eq!(
                    created_dispute.connector_created_at,
                    updated_dispute.connector_created_at
                );
                assert_eq!(
                    created_dispute.connector_updated_at,
                    updated_dispute.connector_updated_at
                );
                assert_eq!(created_dispute.created_at, updated_dispute.created_at);
                assert_ne!(created_dispute.modified_at, updated_dispute.modified_at);
                assert_eq!(created_dispute.connector, updated_dispute.connector);
                assert_ne!(created_dispute.evidence, updated_dispute.evidence);
            }
        }
    }
}
