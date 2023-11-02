use error_stack::{IntoReport, ResultExt};

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
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError>;

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;
}

#[async_trait::async_trait]
impl DisputeInterface for Store {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        dispute
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
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
        .map_err(Into::into)
        .into_report()
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_dispute_id(&conn, merchant_id, dispute_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::filter_by_constraints(&conn, merchant_id, dispute_constraints)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, dispute)
            .await
            .map_err(Into::into)
            .into_report()
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
            id: locked_disputes
                .len()
                .try_into()
                .into_report()
                .change_context(errors::StorageError::MockDbError)?,
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
        };

        locked_disputes.push(new_dispute.clone());

        Ok(new_dispute)
    }
    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        Ok(self
            .disputes
            .lock()
            .await
            .iter()
            .find(|d| {
                d.merchant_id == merchant_id
                    && d.payment_id == payment_id
                    && d.connector_dispute_id == connector_dispute_id
            })
            .cloned())
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        locked_disputes
            .iter()
            .find(|d| d.merchant_id == merchant_id && d.dispute_id == dispute_id)
            .cloned()
            .ok_or(errors::StorageError::ValueNotFound(format!("No dispute available for merchant_id = {merchant_id} and dispute_id = {dispute_id}"))
            .into())
    }

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        Ok(locked_disputes
            .iter()
            .filter(|d| {
                d.merchant_id == merchant_id
                    && dispute_constraints
                        .dispute_status
                        .as_ref()
                        .map(|status| status == &d.dispute_status)
                        .unwrap_or(true)
                    && dispute_constraints
                        .dispute_stage
                        .as_ref()
                        .map(|stage| stage == &d.dispute_stage)
                        .unwrap_or(true)
                    && dispute_constraints
                        .reason
                        .as_ref()
                        .and_then(|reason| {
                            d.connector_reason
                                .as_ref()
                                .map(|connector_reason| connector_reason == reason)
                        })
                        .unwrap_or(true)
                    && dispute_constraints
                        .connector
                        .as_ref()
                        .map(|connector| connector == &d.connector)
                        .unwrap_or(true)
                    && dispute_constraints
                        .received_time
                        .as_ref()
                        .map(|received_time| received_time == &d.created_at)
                        .unwrap_or(true)
                    && dispute_constraints
                        .received_time_lt
                        .as_ref()
                        .map(|received_time_lt| received_time_lt > &d.created_at)
                        .unwrap_or(true)
                    && dispute_constraints
                        .received_time_gt
                        .as_ref()
                        .map(|received_time_gt| received_time_gt < &d.created_at)
                        .unwrap_or(true)
                    && dispute_constraints
                        .received_time_lte
                        .as_ref()
                        .map(|received_time_lte| received_time_lte >= &d.created_at)
                        .unwrap_or(true)
                    && dispute_constraints
                        .received_time_gte
                        .as_ref()
                        .map(|received_time_gte| received_time_gte <= &d.created_at)
                        .unwrap_or(true)
            })
            .take(
                dispute_constraints
                    .limit
                    .and_then(|limit| usize::try_from(limit).ok())
                    .unwrap_or(usize::MAX),
            )
            .cloned()
            .collect())
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        Ok(locked_disputes
            .iter()
            .filter(|d| d.merchant_id == merchant_id && d.payment_id == payment_id)
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
}

#[cfg(test)]
mod tests {
    #[allow(clippy::unwrap_used)]
    mod mockdb_dispute_interface {
        use api_models::disputes::DisputeListConstraints;
        use diesel_models::{
            dispute::DisputeNew,
            enums::{DisputeStage, DisputeStatus},
        };
        use masking::Secret;
        use redis_interface::RedisSettings;
        use serde_json::Value;
        use time::macros::datetime;

        use crate::db::{dispute::DisputeInterface, MockDb};

        pub struct DisputeNewIds {
            dispute_id: String,
            payment_id: String,
            attempt_id: String,
            merchant_id: String,
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
                profile_id: None,
                merchant_connector_id: None,
            }
        }

        #[tokio::test]
        async fn test_insert_dispute() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&RedisSettings::default())
                .await
                .expect("Failed to create a mock DB");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
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
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_2".into(),
                }))
                .await
                .unwrap();

            let found_dispute = mockdb
                .find_by_merchant_id_payment_id_connector_dispute_id(
                    "merchant_1",
                    "payment_1",
                    "connector_dispute_1",
                )
                .await
                .unwrap();

            assert!(found_dispute.is_some());

            assert_eq!(created_dispute, found_dispute.unwrap());
        }

        #[tokio::test]
        async fn test_find_dispute_by_merchant_id_dispute_id() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_dispute = mockdb
                .find_dispute_by_merchant_id_dispute_id("merchant_1", "dispute_1")
                .await
                .unwrap();

            assert_eq!(created_dispute, found_dispute);
        }

        #[tokio::test]
        async fn test_find_disputes_by_merchant_id() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_2".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_disputes = mockdb
                .find_disputes_by_merchant_id(
                    "merchant_1",
                    DisputeListConstraints {
                        limit: None,
                        dispute_status: None,
                        dispute_stage: None,
                        reason: None,
                        connector: None,
                        received_time: None,
                        received_time_lt: None,
                        received_time_gt: None,
                        received_time_lte: None,
                        received_time_gte: None,
                        profile_id: None,
                    },
                )
                .await
                .unwrap();

            assert_eq!(1, found_disputes.len());

            assert_eq!(created_dispute, found_disputes.get(0).unwrap().clone());
        }

        #[tokio::test]
        async fn test_find_disputes_by_merchant_id_payment_id() {
            #[allow(clippy::expect_used)]
            let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                .await
                .expect("Failed to create Mock store");

            let created_dispute = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_1".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_1".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let _ = mockdb
                .insert_dispute(create_dispute_new(DisputeNewIds {
                    dispute_id: "dispute_2".into(),
                    attempt_id: "attempt_1".into(),
                    merchant_id: "merchant_2".into(),
                    payment_id: "payment_1".into(),
                    connector_dispute_id: "connector_dispute_1".into(),
                }))
                .await
                .unwrap();

            let found_disputes = mockdb
                .find_disputes_by_merchant_id_payment_id("merchant_1", "payment_1")
                .await
                .unwrap();

            assert_eq!(1, found_disputes.len());

            assert_eq!(created_dispute, found_disputes.get(0).unwrap().clone());
        }

        mod update_dispute {
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
                #[allow(clippy::expect_used)]
                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: "merchant_1".into(),
                        payment_id: "payment_1".into(),
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

                assert_eq!(created_dispute.id, updated_dispute.id);
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
                #[allow(clippy::expect_used)]
                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: "merchant_1".into(),
                        payment_id: "payment_1".into(),
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

                assert_eq!(created_dispute.id, updated_dispute.id);
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
                #[allow(clippy::expect_used)]
                let mockdb = MockDb::new(&redis_interface::RedisSettings::default())
                    .await
                    .expect("Failed to create Mock store");

                let created_dispute = mockdb
                    .insert_dispute(create_dispute_new(DisputeNewIds {
                        dispute_id: "dispute_1".into(),
                        attempt_id: "attempt_1".into(),
                        merchant_id: "merchant_1".into(),
                        payment_id: "payment_1".into(),
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

                assert_eq!(created_dispute.id, updated_dispute.id);
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
