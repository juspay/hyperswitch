pub use diesel_models::payment_attempt::{
    PaymentAttempt, PaymentAttemptNew, PaymentAttemptUpdate, PaymentAttemptUpdateInternal,
};
use diesel_models::{capture::CaptureNew, enums};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    pub routed_through: Option<String>,
    pub algorithm: Option<api_models::admin::StraightThroughAlgorithm>,
}

pub trait PaymentAttemptExt {
    fn make_new_capture(
        &self,
        capture_amount: i64,
        capture_status: enums::CaptureStatus,
    ) -> CaptureNew;

    fn get_next_capture_id(&self) -> String;
}

impl PaymentAttemptExt for PaymentAttempt {
    fn make_new_capture(
        &self,
        capture_amount: i64,
        capture_status: enums::CaptureStatus,
    ) -> CaptureNew {
        let capture_sequence = self.multiple_capture_count.unwrap_or_default() + 1;
        let now = common_utils::date_time::now();
        CaptureNew {
            payment_id: self.payment_id.clone(),
            merchant_id: self.merchant_id.clone(),
            capture_id: self.get_next_capture_id(),
            status: capture_status,
            amount: capture_amount,
            currency: self.currency,
            connector: self.connector.clone(),
            error_message: None,
            tax_amount: None,
            created_at: now,
            modified_at: now,
            error_code: None,
            error_reason: None,
            authorized_attempt_id: self.attempt_id.clone(),
            capture_sequence,
            connector_transaction_id: None,
        }
    }
    fn get_next_capture_id(&self) -> String {
        let next_sequence_number = self.multiple_capture_count.unwrap_or_default() + 1;
        format!("{}_{}", self.attempt_id.clone(), next_sequence_number)
    }
}

#[cfg(test)]
#[cfg(feature = "dummy_connector")]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use tokio::sync::oneshot;
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
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest, tx).await;

        let payment_id = Uuid::new_v4().to_string();
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::DummyConnector1.to_string();
        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            connector: Some(connector),
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
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest, tx).await;

        let current_time = common_utils::date_time::now();
        let payment_id = Uuid::new_v4().to_string();
        let attempt_id = Uuid::new_v4().to_string();
        let merchant_id = Uuid::new_v4().to_string();
        let connector = types::Connector::DummyConnector1.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            connector: Some(connector),
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
        let uuid = Uuid::new_v4().to_string();
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest, tx).await;
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::DummyConnector1.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: uuid.clone(),
            merchant_id: "1".to_string(),
            connector: Some(connector),
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
