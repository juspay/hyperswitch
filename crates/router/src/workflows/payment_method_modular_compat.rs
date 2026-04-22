use std::str::FromStr;

use common_utils::ext_traits::{OptionExt, ValueExt};
use error_stack::ResultExt;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::payment_methods::{cards, vault},
    errors,
    logger::error,
    routes::SessionState,
    types::storage::{self, PaymentMethodModularCompatTrackingData},
};

pub struct PaymentMethodModularCompatWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodModularCompatWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let tracking_data: PaymentMethodModularCompatTrackingData = process
            .tracking_data
            .clone()
            .parse_value("PaymentMethodModularCompatTrackingData")?;

        let merchant_id = tracking_data.merchant_id;
        let key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                &merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await?;

        let merchant_account = db
            .find_merchant_account_by_merchant_id(&merchant_id, &key_store)
            .await?;

        let payment_method = db
            .find_payment_method(
                &key_store,
                &tracking_data.payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch payment method for modular compatibility PT")?;

        let pm_id_update = storage::PaymentMethodUpdate::PopulateId {
            id: tracking_data.payment_method_id.clone(),
            last_modified_by: tracking_data.last_modified_by.clone(),
        };

        let payment_method = db
            .update_payment_method(
                &key_store,
                payment_method,
                pm_id_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to populate id for payment method in modular compatibility PT")?;

        if payment_method.payment_method != Some(common_enums::PaymentMethod::Card) {
            return db
                .as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_NON_CARD")
                .await
                .map_err(Into::<errors::ProcessTrackerError>::into);
        }

        let customer_id = payment_method
            .customer_id
            .clone()
            .get_required_value("customer_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("customer_id not found for card payment method in compatibility PT")?;

        let card_reference = payment_method
            .locker_id
            .clone()
            .unwrap_or(payment_method.payment_method_id.clone());

        let locker_card = cards::get_card_from_vault(state, &customer_id, &merchant_id, &card_reference)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve card from legacy locker in compatibility PT")?;

        let card_network = payment_method
            .scheme
            .as_ref()
            .and_then(|scheme| common_enums::CardNetwork::from_str(scheme).ok());

        let card_detail = api_models::payment_methods::CardDetail::from((locker_card.get_card(), card_network));
        let pmd = hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card_detail);

        let entity_id = hyperswitch_domain_models::vault::V1VaultEntityId::new(
            key_store.merchant_id.clone(),
            customer_id,
        );

        let _resp = vault::add_payment_method_to_vault(
            state,
            &pmd,
            Some(crate::types::domain::VaultId::generate(card_reference)),
            entity_id,
            Some(crate::types::payment_methods::WriteMode::Upsert),
        )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in generic locker in compatibility PT")?;

        db.as_scheduler()
            .finish_process_with_business_status(process, "COMPLETED_BY_PT")
            .await?;

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        todo!()
    }

    async fn error_handler<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
        _error: errors::ProcessTrackerError,
    ) -> errors::CustomResult<(), errors::ProcessTrackerError> {
        let db = &*state.store;
        let retry_count = process.retry_count;
        let mapping = process_data::PaymentMethodsPTMapping::default();

        let time_delta = if retry_count == 0 {
            Some(mapping.default_mapping.start_after)
        } else {
            pt_utils::get_delay(retry_count + 1, &mapping.default_mapping.frequencies)
        };

        let schedule_time = pt_utils::get_time_from_delta(time_delta);

        match schedule_time {
            Some(s_time) => {
                db.as_scheduler()
                    .retry_process(process, s_time)
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?;
            }
            None => {
                db.as_scheduler()
                    .finish_process_with_business_status(process, "RETRIES_EXCEEDED")
                    .await
                    .map_err(Into::<errors::ProcessTrackerError>::into)?;
            }
        }

        error!("Failed while executing payment method modular compatibility workflow");
        Ok(())
    }
}
