use api_models::payment_methods::Card;
use common_utils::ext_traits::{Encode, OptionExt, StringExt, ValueExt};
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::payment_methods::{cards, transformers, vault},
    errors,
    logger::{self, error},
    routes::SessionState,
    types::{
        payment_methods as pm_types,
        storage::{self, PaymentMethodModularCompatTrackingData},
    },
};

pub struct PaymentMethodModularBackwardCompatWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodModularBackwardCompatWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!(process_id=%process.id, "Starting payment method modular backward compatibility PT");
        let db = &*state.store;
        let tracking_data: PaymentMethodModularCompatTrackingData =
            process
                .tracking_data
                .clone()
                .parse_value("PaymentMethodModularCompatTrackingData")?;
        logger::info!(process_id=%process.id, ?tracking_data, "Parsed modular backward compatibility PT tracking data");

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
            .attach_printable(
                "Failed to fetch payment method for modular backward compatibility PT",
            )?;

        let payment_method_type = payment_method.get_payment_method_type();
        let payment_method_subtype = payment_method.get_payment_method_subtype();
        let payment_method = if payment_method_type.is_some() || payment_method_subtype.is_some() {
            let pm_update = storage::PaymentMethodUpdate::AdditionalDataUpdate {
                payment_method_data: None,
                status: None,
                locker_id: None,
                payment_method: payment_method_type,
                payment_method_type: payment_method_subtype,
                payment_method_issuer: None,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                last_modified_by: tracking_data.last_modified_by.clone(),
                metadata: None,
                last_used_at: None,
                connector_mandate_details: None,
                network_tokenization_data: None,
            };

            db.update_payment_method(
                &key_store,
                payment_method,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to populate legacy payment method fields in backward compatibility PT",
            )?
        } else {
            payment_method
        };

        let business_status = if payment_method.get_payment_method_type()
            != Some(common_enums::PaymentMethod::Card)
        {
            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Skipping legacy locker write for non-card payment method"
            );
            "COMPLETED_BY_PT"
        } else if payment_method.locker_id.is_none() {
            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Skipping legacy locker write for card payment method as locker reference is missing"
            );
            "COMPLETED_BY_PT"
        } else if payment_method.customer_id.is_none() {
            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Skipping legacy locker write for modular backward compatibility PT because customer_id is missing"
            );
            "COMPLETED_BY_PT"
        } else {
            let customer_id = payment_method
                .customer_id
                .clone()
                .get_required_value("customer_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "customer_id not found for card payment method in backward compatibility PT",
                )?;

            let card_reference = payment_method
                .locker_id
                .clone()
                .get_required_value("locker_id")?
                .to_string();
            #[derive(Debug, serde::Serialize)]
            struct ModularBackwardCompatVaultRetrieveRequest {
                entity_id: common_utils::id_type::CustomerId,
                vault_id: hyperswitch_domain_models::payment_methods::VaultId,
            }

            let vault_request = ModularBackwardCompatVaultRetrieveRequest {
                entity_id: customer_id.clone(),
                vault_id: hyperswitch_domain_models::payment_methods::VaultId::generate(
                    card_reference.clone(),
                ),
            };
            let payload = vault_request
                .encode_to_vec()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to encode generic locker retrieve request in backward compatibility PT",
                )?;
            let vault_response =
                vault::call_to_vault::<pm_types::VaultRetrieve>(state, payload, None)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to retrieve card from generic locker in backward compatibility PT",
                    )?;
            let stored_pm_resp: pm_types::VaultRetrieveResponse = vault_response
                .parse_struct("VaultRetrieveResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to parse generic locker retrieve response in backward compatibility PT",
                )?;
            let locker_card_detail = match stored_pm_resp.data {
                hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card) => card,
                hyperswitch_domain_models::vault::PaymentMethodVaultingData::NetworkToken(_)
                | hyperswitch_domain_models::vault::PaymentMethodVaultingData::CardNumber(_)
                | hyperswitch_domain_models::vault::PaymentMethodVaultingData::BankDebit(_) => {
                    return Err(error_stack::report!(
                        errors::ApiErrorResponse::InternalServerError
                    )
                    .attach_printable(
                        "Generic locker returned non-card data in backward compatibility PT",
                    )
                    .into());
                }
            };

            let card_isin = locker_card_detail.card_number.get_card_isin();
            let locker_req = transformers::StoreLockerReq::LockerCard(transformers::StoreCardReq {
                merchant_id,
                merchant_customer_id: customer_id.clone(),
                requestor_card_reference: Some(card_reference.to_owned()),
                card: Card {
                    card_number: locker_card_detail.card_number,
                    name_on_card: locker_card_detail.card_holder_name,
                    card_exp_month: locker_card_detail.card_exp_month,
                    card_exp_year: locker_card_detail.card_exp_year,
                    card_brand: locker_card_detail
                        .card_network
                        .map(|network| network.to_string()),
                    card_isin: Some(card_isin),
                    nick_name: locker_card_detail
                        .nick_name
                        .as_ref()
                        .map(|nick_name| nick_name.peek().to_owned()),
                },
                ttl: state.conf.locker.ttl_for_storage_in_secs,
            });

            let _ = cards::add_card_to_vault(state, &locker_req, &customer_id)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add card to legacy locker in backward compatibility PT",
                )?;

            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Upserted card into legacy locker in modular backward compatibility PT"
            );
            "COMPLETED_BY_PT"
        };

        db.as_scheduler()
            .finish_process_with_business_status(process, business_status)
            .await?;
        crate::logger::info!(
            business_status,
            "Finished payment method modular backward compatibility PT"
        );

        Ok(())
    }

    #[cfg(feature = "v2")]
    async fn execute_workflow<'a>(
        &'a self,
        _state: &'a SessionState,
        _process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        Err(errors::ProcessTrackerError::NotImplemented)
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

        error!("Failed while executing payment method modular backward compatibility workflow");
        Ok(())
    }
}
