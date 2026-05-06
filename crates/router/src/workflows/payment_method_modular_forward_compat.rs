use std::str::FromStr;

use common_utils::ext_traits::{Encode, OptionExt, StringExt, ValueExt};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

#[cfg(feature = "v1")]
use crate::types::payment_methods as pm_types;
use crate::{
    core::payment_methods::{cards, vault},
    errors,
    logger::{self, error},
    routes::SessionState,
    types::storage::{self, PaymentMethodModularCompatTrackingData},
};

pub struct PaymentMethodModularForwardCompatWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodModularForwardCompatWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        // Step 1: Parse PT tracking payload and load merchant context (key store + account).
        logger::info!(process_id=%process.id, "Starting payment method modular compatibility PT");
        let db = &*state.store;
        let tracking_data: PaymentMethodModularCompatTrackingData =
            process
                .tracking_data
                .clone()
                .parse_value("PaymentMethodModularCompatTrackingData")?;
        logger::info!(process_id=%process.id, ?tracking_data, "Parsed modular compatibility PT tracking data");

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

        // Step 2: Populate v1 compat fields used by PM modular service.
        let pm_id_update = storage::PaymentMethodUpdate::PopulateModularCompatFields {
            id: tracking_data.payment_method_id.clone(),
            payment_method_type_v2: payment_method.payment_method,
            payment_method_subtype: payment_method.payment_method_type,
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
            .attach_printable(
                "Failed to populate id for payment method in modular compatibility PT",
            )?;

        let business_status = if payment_method.payment_method
            != Some(common_enums::PaymentMethod::Card)
        {
            // Step 3: Non-card payment methods do not require locker migration.
            crate::logger::info!(
                process_id=%process.id,
                payment_method_id=%payment_method.payment_method_id,
                "Payment method is non-card; skipping locker migration in modular compatibility PT"
            );
            "COMPLETED_BY_PT"
        } else {
            let customer_id = payment_method
                .customer_id
                .clone()
                .get_required_value("customer_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "customer_id not found for card payment method in compatibility PT",
                )?;

            let card_network = payment_method
                .scheme
                .as_ref()
                .and_then(|scheme| common_enums::CardNetwork::from_str(scheme).ok());

            // Step 4: Fetch card payload from legacy locker only when locker_id is present.
            if let Some(card_reference) = payment_method.locker_id.clone() {
                let locker_card =
                    cards::get_card_from_vault(state, &customer_id, &merchant_id, &card_reference)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to retrieve card from legacy locker in compatibility PT",
                        )?;

                let card_detail = api_models::payment_methods::CardDetail::from((
                    locker_card.get_card(),
                    card_network.clone(),
                ));
                let pmd =
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card_detail);

                // Step 5: Upsert the card into generic locker via direct AddVault call.
                let entity_id = hyperswitch_domain_models::vault::V1VaultEntityId::new(
                    key_store.merchant_id.clone(),
                    customer_id.clone(),
                );

                let payload = pm_types::AddVaultRequest {
                    entity_id,
                    vault_id: crate::types::domain::VaultId::generate(card_reference),
                    data: &pmd,
                    ttl: state.conf.locker.ttl_for_storage_in_secs,
                }
                .encode_to_vec()
                .change_context(errors::VaultError::RequestEncodingFailed)
                .attach_printable("Failed to encode AddVaultRequest")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add payment method in generic locker in compatibility PT",
                )?;

                let query_params =
                    Some(pm_types::VaultQueryParam::from(pm_types::WriteMode::Upsert));

                let resp = vault::call_to_vault::<pm_types::AddVault>(state, payload, query_params)
                    .await
                    .change_context(errors::VaultError::VaultAPIError)
                    .attach_printable("Call to vault failed")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to add payment method in generic locker in compatibility PT",
                    )?;

                let _stored_pm_resp: pm_types::InternalAddVaultResponse = resp
                    .parse_struct("InternalAddVaultResponse")
                    .change_context(errors::VaultError::ResponseDeserializationFailed)
                    .attach_printable("Failed to parse data into InternalAddVaultResponse")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to add payment method in generic locker in compatibility PT",
                    )?;
                logger::info!(
                    process_id=%process.id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Upserted card into generic locker in modular compatibility PT"
                );
            } else {
                logger::info!(
                    process_id=%process.id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Skipping card migration in modular compatibility PT as locker_id is absent"
                );
            }

            // Step 6: Migrate network token to generic locker when an NT locker id is present.
            if let Some(network_token_locker_id) = payment_method.network_token_locker_id.clone() {
                let locker_network_token = cards::get_card_from_vault(
                    state,
                    &customer_id,
                    &merchant_id,
                    &network_token_locker_id,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to retrieve network token from legacy locker in compatibility PT",
                )?;

                let locker_network_token_card = locker_network_token.get_card();
                let network_token_details =
                    hyperswitch_domain_models::payment_method_data::NetworkTokenDetails {
                        network_token: locker_network_token_card.card_number.into(),
                        network_token_exp_month: locker_network_token_card.card_exp_month,
                        network_token_exp_year: locker_network_token_card.card_exp_year,
                        cryptogram: None,
                        card_issuer: payment_method.issuer_name.clone(),
                        card_network,
                        card_type: None,
                        card_issuing_country: None,
                        card_holder_name: locker_network_token_card.name_on_card,
                        nick_name: locker_network_token_card.nick_name.map(Secret::new),
                        par: None,
                    };

                let network_token_pmd =
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::NetworkToken(
                        network_token_details.clone(),
                    );

                let network_token_entity_id =
                    hyperswitch_domain_models::vault::V1VaultEntityId::new(
                        key_store.merchant_id.clone(),
                        customer_id,
                    );

                let network_token_payload = pm_types::AddVaultRequest {
                    entity_id: network_token_entity_id,
                    vault_id: crate::types::domain::VaultId::generate(
                        network_token_locker_id.clone(),
                    ),
                    data: &network_token_pmd,
                    ttl: state.conf.locker.ttl_for_storage_in_secs,
                }
                .encode_to_vec()
                .change_context(errors::VaultError::RequestEncodingFailed)
                .attach_printable("Failed to encode AddVaultRequest for network token")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add network token in generic locker in compatibility PT",
                )?;

                let network_token_query_params =
                    Some(pm_types::VaultQueryParam::from(pm_types::WriteMode::Upsert));

                let network_token_resp = vault::call_to_vault::<pm_types::AddVault>(
                    state,
                    network_token_payload,
                    network_token_query_params,
                )
                .await
                .change_context(errors::VaultError::VaultAPIError)
                .attach_printable("Call to vault failed for network token")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add network token in generic locker in compatibility PT",
                )?;

                let _stored_network_token_resp: pm_types::InternalAddVaultResponse =
                    network_token_resp
                        .parse_struct("InternalAddVaultResponse")
                        .change_context(errors::VaultError::ResponseDeserializationFailed)
                        .attach_printable(
                            "Failed to parse network token response into InternalAddVaultResponse",
                        )
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to add network token in generic locker in compatibility PT",
                        )?;

                crate::logger::info!(
                    process_id=%process.id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Upserted network token into generic locker in modular compatibility PT"
                );
            } else {
                crate::logger::info!(
                    process_id=%process.id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Skipping network token migration in modular compatibility PT as network_token_locker_id is absent"
                );
            }

            "COMPLETED_BY_PT"
        };

        // Step 7: Mark process as completed once the required branch is done.
        db.as_scheduler()
            .finish_process_with_business_status(process, business_status)
            .await?;
        crate::logger::info!(
            business_status,
            "Finished payment method modular compatibility PT"
        );

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
