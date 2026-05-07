use api_models::payment_methods::Card;
use common_utils::ext_traits::{OptionExt, ValueExt};
#[cfg(feature = "v2")]
use common_utils::id_type;
use error_stack::ResultExt;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

use crate::{
    core::payment_methods::{cards, transformers},
    errors,
    logger::{self, error},
    routes::SessionState,
    types::storage::{self, PaymentMethodModularCompatTrackingData},
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
                .get_required_value("locker_id")?;
            let locker_card =
                cards::get_card_from_vault(state, &customer_id, &merchant_id, &card_reference)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to retrieve card from generic locker in backward compatibility PT",
                    )?;

            let locker_card_detail = locker_card.get_card();
            let locker_req = transformers::StoreLockerReq::LockerCard(transformers::StoreCardReq {
                merchant_id,
                merchant_customer_id: customer_id.clone(),
                requestor_card_reference: Some(card_reference.to_owned()),
                card: Card {
                    card_number: locker_card_detail.card_number,
                    name_on_card: locker_card_detail.name_on_card,
                    card_exp_month: locker_card_detail.card_exp_month,
                    card_exp_year: locker_card_detail.card_exp_year,
                    card_brand: locker_card_detail.card_brand,
                    card_isin: locker_card_detail.card_isin,
                    nick_name: locker_card_detail.nick_name,
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

        let payment_method_id = id_type::GlobalPaymentMethodId::new_unchecked(
            tracking_data.payment_method_id.to_owned(),
        );

        let payment_method = db
            .find_payment_method(
                &key_store,
                &payment_method_id,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to fetch payment method for modular backward compatibility PT",
            )?;

        let business_status = if payment_method.get_payment_method_type()
            != Some(common_enums::PaymentMethod::Card)
        {
            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Skipping legacy locker write for non-card payment method"
            );
            "COMPLETED_BY_PT"
        } else if let Some(card_reference) = payment_method
            .locker_id
            .as_ref()
            .map(|locker_id| locker_id.get_string_repr().to_owned())
        {
            match payment_method.customer_id.clone() {
                Some(customer_id) => {
                    let legacy_customer_id = id_type::CustomerId::try_from(customer_id)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to convert global customer_id to legacy customer_id in backward compatibility PT",
                        )?;

                    let locker_card = cards::get_card_from_vault(
                        state,
                        &legacy_customer_id,
                        &merchant_id,
                        card_reference.as_str(),
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to retrieve card from generic locker in backward compatibility PT",
                    )?;

                    let locker_card_detail = locker_card.get_card();
                    let locker_req =
                        transformers::StoreLockerReq::LockerCard(transformers::StoreCardReq {
                            merchant_id,
                            merchant_customer_id: legacy_customer_id.to_owned(),
                            requestor_card_reference: Some(card_reference.to_owned()),
                            card: Card {
                                card_number: locker_card_detail.card_number,
                                name_on_card: locker_card_detail.name_on_card,
                                card_exp_month: locker_card_detail.card_exp_month,
                                card_exp_year: locker_card_detail.card_exp_year,
                                card_brand: locker_card_detail.card_brand,
                                card_isin: locker_card_detail.card_isin,
                                nick_name: locker_card_detail.nick_name,
                            },
                            ttl: state.conf.locker.ttl_for_storage_in_secs,
                        });

                    let _ = cards::add_card_to_vault(state, &locker_req, &legacy_customer_id)
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
                }
                None => {
                    logger::info!(
                        process_id=%process.id,
                        payment_method_id=%tracking_data.payment_method_id,
                        "Skipping legacy locker write for modular backward compatibility PT because customer_id is missing"
                    );
                    "COMPLETED_BY_PT"
                }
            }
        } else {
            logger::info!(
                process_id=%process.id,
                payment_method_id=%tracking_data.payment_method_id,
                "Skipping legacy locker write for modular backward compatibility PT because locker_id is missing"
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
