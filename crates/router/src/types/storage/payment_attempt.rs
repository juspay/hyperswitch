use common_utils::types::MinorUnit;
use diesel_models::{capture::CaptureNew, enums};
use error_stack::ResultExt;
pub use hyperswitch_domain_models::payments::payment_attempt::{
    PaymentAttempt, PaymentAttemptUpdate,
};

use crate::{
    core::errors, errors::RouterResult, types::transformers::ForeignFrom, utils::OptionExt,
};
pub trait PaymentAttemptExt {
    fn make_new_capture(
        &self,
        capture_amount: MinorUnit,
        capture_status: enums::CaptureStatus,
    ) -> RouterResult<CaptureNew>;

    fn get_next_capture_id(&self) -> String;
    fn get_total_amount(&self) -> MinorUnit;
    fn get_surcharge_details(&self) -> Option<api_models::payments::RequestSurchargeDetails>;
}

impl PaymentAttemptExt for PaymentAttempt {
    #[cfg(feature = "v2")]
    fn make_new_capture(
        &self,
        capture_amount: MinorUnit,
        capture_status: enums::CaptureStatus,
    ) -> RouterResult<CaptureNew> {
        todo!()
    }

    #[cfg(feature = "v1")]
    fn make_new_capture(
        &self,
        capture_amount: MinorUnit,
        capture_status: enums::CaptureStatus,
    ) -> RouterResult<CaptureNew> {
        let capture_sequence = self.multiple_capture_count.unwrap_or_default() + 1;
        let now = common_utils::date_time::now();
        Ok(CaptureNew {
            payment_id: self.payment_id.clone(),
            merchant_id: self.merchant_id.clone(),
            capture_id: self.get_next_capture_id(),
            status: capture_status,
            amount: capture_amount,
            currency: self.currency,
            connector: self
                .connector
                .clone()
                .get_required_value("connector")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "connector field is required in payment_attempt to create a capture",
                )?,
            error_message: None,
            tax_amount: None,
            created_at: now,
            modified_at: now,
            error_code: None,
            error_reason: None,
            authorized_attempt_id: self.attempt_id.clone(),
            capture_sequence,
            connector_capture_id: None,
            connector_response_reference_id: None,
            connector_capture_data: None,
        })
    }

    #[cfg(feature = "v1")]
    fn get_next_capture_id(&self) -> String {
        let next_sequence_number = self.multiple_capture_count.unwrap_or_default() + 1;
        format!("{}_{}", self.attempt_id.clone(), next_sequence_number)
    }

    #[cfg(feature = "v2")]
    fn get_next_capture_id(&self) -> String {
        todo!()
    }

    #[cfg(feature = "v1")]
    fn get_surcharge_details(&self) -> Option<api_models::payments::RequestSurchargeDetails> {
        self.net_amount
            .get_surcharge_amount()
            .map(
                |surcharge_amount| api_models::payments::RequestSurchargeDetails {
                    surcharge_amount,
                    tax_amount: self.net_amount.get_tax_on_surcharge(),
                },
            )
    }

    #[cfg(feature = "v2")]
    fn get_surcharge_details(&self) -> Option<api_models::payments::RequestSurchargeDetails> {
        todo!()
    }

    #[cfg(feature = "v1")]
    fn get_total_amount(&self) -> MinorUnit {
        self.net_amount.get_total_amount()
    }

    #[cfg(feature = "v2")]
    fn get_total_amount(&self) -> MinorUnit {
        todo!()
    }
}

pub trait AttemptStatusExt {
    fn maps_to_intent_status(self, intent_status: enums::IntentStatus) -> bool;
}

impl AttemptStatusExt for enums::AttemptStatus {
    fn maps_to_intent_status(self, intent_status: enums::IntentStatus) -> bool {
        enums::IntentStatus::foreign_from(self) == intent_status
    }
}

#[cfg(test)]
#[cfg(all(
    feature = "v1", // Ignoring tests for v2 since they aren't actively running
    feature = "dummy_connector"
))]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::print_stderr)]
    use hyperswitch_domain_models::payments::payment_attempt::PaymentAttemptNew;
    use tokio::sync::oneshot;
    use uuid::Uuid;

    use crate::{
        configs::settings::Settings,
        db::StorageImpl,
        routes, services,
        types::{self, storage::enums},
    };

    async fn create_single_connection_test_transaction_pool() -> routes::AppState {
        // Set pool size to 1 and minimum idle connection size to 0
        std::env::set_var("ROUTER__MASTER_DATABASE__POOL_SIZE", "1");
        std::env::set_var("ROUTER__MASTER_DATABASE__MIN_IDLE", "0");
        std::env::set_var("ROUTER__REPLICA_DATABASE__POOL_SIZE", "1");
        std::env::set_var("ROUTER__REPLICA_DATABASE__MIN_IDLE", "0");

        let conf = Settings::new().expect("invalid settings");
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let api_client = Box::new(services::MockApiClient);
        Box::pin(routes::AppState::with_storage(
            conf,
            StorageImpl::PostgresqlTest,
            tx,
            api_client,
        ))
        .await
    }

    #[tokio::test]
    async fn test_payment_attempt_insert() {
        let state = create_single_connection_test_transaction_pool().await;
        let payment_id =
            common_utils::id_type::PaymentId::generate_test_payment_id_for_sample_data();
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::DummyConnector1.to_string();
        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            connector: Some(connector),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            merchant_id: Default::default(),
            attempt_id: Default::default(),
            status: Default::default(),
            net_amount: Default::default(),
            currency: Default::default(),
            save_to_locker: Default::default(),
            error_message: Default::default(),
            offer_amount: Default::default(),
            payment_method_id: Default::default(),
            payment_method: Default::default(),
            capture_method: Default::default(),
            capture_on: Default::default(),
            confirm: Default::default(),
            authentication_type: Default::default(),
            last_synced: Default::default(),
            cancellation_reason: Default::default(),
            amount_to_capture: Default::default(),
            mandate_id: Default::default(),
            browser_info: Default::default(),
            payment_token: Default::default(),
            error_code: Default::default(),
            connector_metadata: Default::default(),
            payment_experience: Default::default(),
            payment_method_type: Default::default(),
            payment_method_data: Default::default(),
            business_sub_label: Default::default(),
            straight_through_algorithm: Default::default(),
            preprocessing_step_id: Default::default(),
            mandate_details: Default::default(),
            error_reason: Default::default(),
            connector_response_reference_id: Default::default(),
            multiple_capture_count: Default::default(),
            amount_capturable: Default::default(),
            updated_by: Default::default(),
            authentication_data: Default::default(),
            encoded_data: Default::default(),
            merchant_connector_id: Default::default(),
            unified_code: Default::default(),
            unified_message: Default::default(),
            external_three_ds_authentication_attempted: Default::default(),
            authentication_connector: Default::default(),
            authentication_id: Default::default(),
            mandate_data: Default::default(),
            payment_method_billing_address_id: Default::default(),
            fingerprint_id: Default::default(),
            client_source: Default::default(),
            client_version: Default::default(),
            customer_acceptance: Default::default(),
            profile_id: common_utils::generate_profile_id_of_default_length(),
            organization_id: Default::default(),
            connector_mandate_detail: Default::default(),
            card_discovery: Default::default(),
        };

        let store = state
            .stores
            .get(state.conf.multitenancy.get_tenant_ids().first().unwrap())
            .unwrap();
        let response = store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();
        eprintln!("{response:?}");

        assert_eq!(response.payment_id, payment_id.clone());
    }

    #[tokio::test]
    /// Example of unit test
    /// Kind of test: state-based testing
    async fn test_find_payment_attempt() {
        let state = create_single_connection_test_transaction_pool().await;
        let current_time = common_utils::date_time::now();
        let payment_id =
            common_utils::id_type::PaymentId::generate_test_payment_id_for_sample_data();
        let attempt_id = Uuid::new_v4().to_string();
        let merchant_id = common_utils::id_type::MerchantId::new_from_unix_timestamp();
        let connector = types::Connector::DummyConnector1.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            connector: Some(connector),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            attempt_id: attempt_id.clone(),
            status: Default::default(),
            net_amount: Default::default(),
            currency: Default::default(),
            save_to_locker: Default::default(),
            error_message: Default::default(),
            offer_amount: Default::default(),
            payment_method_id: Default::default(),
            payment_method: Default::default(),
            capture_method: Default::default(),
            capture_on: Default::default(),
            confirm: Default::default(),
            authentication_type: Default::default(),
            last_synced: Default::default(),
            cancellation_reason: Default::default(),
            amount_to_capture: Default::default(),
            mandate_id: Default::default(),
            browser_info: Default::default(),
            payment_token: Default::default(),
            error_code: Default::default(),
            connector_metadata: Default::default(),
            payment_experience: Default::default(),
            payment_method_type: Default::default(),
            payment_method_data: Default::default(),
            business_sub_label: Default::default(),
            straight_through_algorithm: Default::default(),
            preprocessing_step_id: Default::default(),
            mandate_details: Default::default(),
            error_reason: Default::default(),
            connector_response_reference_id: Default::default(),
            multiple_capture_count: Default::default(),
            amount_capturable: Default::default(),
            updated_by: Default::default(),
            authentication_data: Default::default(),
            encoded_data: Default::default(),
            merchant_connector_id: Default::default(),
            unified_code: Default::default(),
            unified_message: Default::default(),
            external_three_ds_authentication_attempted: Default::default(),
            authentication_connector: Default::default(),
            authentication_id: Default::default(),
            mandate_data: Default::default(),
            payment_method_billing_address_id: Default::default(),
            fingerprint_id: Default::default(),
            client_source: Default::default(),
            client_version: Default::default(),
            customer_acceptance: Default::default(),
            profile_id: common_utils::generate_profile_id_of_default_length(),
            organization_id: Default::default(),
            connector_mandate_detail: Default::default(),
            card_discovery: Default::default(),
        };
        let store = state
            .stores
            .get(state.conf.multitenancy.get_tenant_ids().first().unwrap())
            .unwrap();
        store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        let response = store
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

    #[tokio::test]
    /// Example of unit test
    /// Kind of test: state-based testing
    async fn test_payment_attempt_mandate_field() {
        let state = create_single_connection_test_transaction_pool().await;
        let uuid = Uuid::new_v4().to_string();
        let merchant_id =
            common_utils::id_type::MerchantId::try_from(std::borrow::Cow::from("merchant1"))
                .unwrap();

        let payment_id =
            common_utils::id_type::PaymentId::generate_test_payment_id_for_sample_data();
        let current_time = common_utils::date_time::now();
        let connector = types::Connector::DummyConnector1.to_string();

        let payment_attempt = PaymentAttemptNew {
            payment_id: payment_id.clone(),
            merchant_id: merchant_id.clone(),
            connector: Some(connector),
            created_at: current_time.into(),
            modified_at: current_time.into(),
            mandate_id: Some("man_121212".to_string()),
            attempt_id: uuid.clone(),
            status: Default::default(),
            net_amount: Default::default(),
            currency: Default::default(),
            save_to_locker: Default::default(),
            error_message: Default::default(),
            offer_amount: Default::default(),
            payment_method_id: Default::default(),
            payment_method: Default::default(),
            capture_method: Default::default(),
            capture_on: Default::default(),
            confirm: Default::default(),
            authentication_type: Default::default(),
            last_synced: Default::default(),
            cancellation_reason: Default::default(),
            amount_to_capture: Default::default(),
            browser_info: Default::default(),
            payment_token: Default::default(),
            error_code: Default::default(),
            connector_metadata: Default::default(),
            payment_experience: Default::default(),
            payment_method_type: Default::default(),
            payment_method_data: Default::default(),
            business_sub_label: Default::default(),
            straight_through_algorithm: Default::default(),
            preprocessing_step_id: Default::default(),
            mandate_details: Default::default(),
            error_reason: Default::default(),
            connector_response_reference_id: Default::default(),
            multiple_capture_count: Default::default(),
            amount_capturable: Default::default(),
            updated_by: Default::default(),
            authentication_data: Default::default(),
            encoded_data: Default::default(),
            merchant_connector_id: Default::default(),
            unified_code: Default::default(),
            unified_message: Default::default(),
            external_three_ds_authentication_attempted: Default::default(),
            authentication_connector: Default::default(),
            authentication_id: Default::default(),
            mandate_data: Default::default(),
            payment_method_billing_address_id: Default::default(),
            fingerprint_id: Default::default(),
            client_source: Default::default(),
            client_version: Default::default(),
            customer_acceptance: Default::default(),
            profile_id: common_utils::generate_profile_id_of_default_length(),
            organization_id: Default::default(),
            connector_mandate_detail: Default::default(),
            card_discovery: Default::default(),
        };
        let store = state
            .stores
            .get(state.conf.multitenancy.get_tenant_ids().first().unwrap())
            .unwrap();
        store
            .insert_payment_attempt(payment_attempt, enums::MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        let response = store
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_id,
                &merchant_id,
                &uuid,
                enums::MerchantStorageScheme::PostgresOnly,
            )
            .await
            .unwrap();
        // checking it after fetch
        assert_eq!(response.mandate_id, Some("man_121212".to_string()));
    }
}
