use error_stack::ResultExt;
pub use storage_models::payment_attempt::{
    PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
};

use crate::{
    core::errors::{self, CustomResult},
    utils::ValueExt,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    pub routed_through: Option<String>,
    pub algorithm: Option<api_models::admin::RoutingAlgorithm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutedThroughData {
    pub routed_through: Option<String>,
}

pub trait PaymentAttemptExt {
    fn get_routed_through_connector(&self) -> CustomResult<Option<String>, errors::ParsingError>;
}

impl PaymentAttemptExt for PaymentAttempt {
    fn get_routed_through_connector(&self) -> CustomResult<Option<String>, errors::ParsingError> {
        if let Some(ref val) = self.connector {
            let data: RoutedThroughData = val
                .clone()
                .parse_value("RoutedThroughData")
                .attach_printable("Failed to read routed_through connector from payment attempt")?;

            Ok(data.routed_through)
        } else {
            Ok(None)
        }
    }
}

#[cfg(feature = "kv_store")]
impl crate::utils::storage_partitioning::KvStorePartition for PaymentAttempt {}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use uuid::Uuid;

    use super::*;
    use crate::{
        configs::settings::Settings,
        db::StorageImpl,
        routes,
        types::{self, storage::enums},
    };

    #[actix_rt::test]
    #[ignore]
    async fn test_payment_attempt_insert() {
        let conf = Settings::new().expect("invalid settings");

        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;

        let payment_id = Uuid::new_v4().to_string();
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::Dummy.to_string();
        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            connector: Some(serde_json::json!({
                "routed_through": connector,
            })),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            ..PaymentAttemptNew::default()
        };

        let response = state
            .store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();
        eprintln!("{response:?}");

        assert_eq!(response.payment_id, payment_id.clone());
    }

    #[actix_rt::test]
    /// Example of unit test
    /// Kind of test: state-based testing
    async fn test_find_payment_attempt() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");
        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;

        let current_time = common_utils::date_time::now();
        let payment_id = Uuid::new_v4().to_string();
        let attempt_id = Uuid::new_v4().to_string();
        let merchant_id = Uuid::new_v4().to_string();
        let connector = types::Connector::Dummy.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            connector: Some(serde_json::json!({
                "routed_through": connector,
            })),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            attempt_id: attempt_id.clone(),
            ..PaymentAttemptNew::default()
        };
        state
            .store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        let response = state
            .store
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_id,
                &merchant_id,
                &attempt_id,
                enums::MerchantStorageScheme::PostgresOnly,
            )
            .await
            .unwrap();

        eprintln!("{response:?}");

        assert_eq!(response.payment_id, payment_id);
    }

    #[actix_rt::test]
    /// Example of unit test
    /// Kind of test: state-based testing
    async fn test_payment_attempt_mandate_field() {
        use crate::configs::settings::Settings;
        let conf = Settings::new().expect("invalid settings");
        let uuid = uuid::Uuid::new_v4().to_string();
        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::Dummy.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: uuid.clone(),
            merchant_id: "1".to_string(),
            connector: Some(serde_json::json!({
                "routed_through": connector,
            })),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            // Adding a mandate_id
            mandate_id: Some("man_121212".to_string()),
            attempt_id: uuid.clone(),
            ..PaymentAttemptNew::default()
        };
        state
            .store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        let response = state
            .store
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &uuid,
                "1",
                &uuid,
                enums::MerchantStorageScheme::PostgresOnly,
            )
            .await
            .unwrap();
        // checking it after fetch
        assert_eq!(response.mandate_id, Some("man_121212".to_string()));
    }
}
