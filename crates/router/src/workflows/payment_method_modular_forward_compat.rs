use std::{marker::PhantomData, str::FromStr};

use common_utils::ext_traits::{Encode, OptionExt, StringExt, ValueExt};
use error_stack::ResultExt;
use hyperswitch_masking::Secret;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

#[cfg(feature = "v1")]
use crate::types::payment_methods as pm_types;
use crate::{
    core::payment_methods::{cards, utils as payment_method_utils, vault},
    errors,
    logger::{self, error},
    routes::{app::StorageInterface, SessionState},
    types::{domain, storage, storage::PaymentMethodModularCompatTrackingData},
};

pub struct PaymentMethodModularForwardCompatWorkflow;

#[cfg(feature = "v1")]
#[derive(Default)]
struct ForwardCompatUpdates {
    connector_mandate_details: Option<diesel_models::payment_method::ConnectorMandateCompatDetails>,
    locker_fingerprint_id: Option<String>,
    auxiliary_fingerprint_id: Option<String>,
}

#[cfg(feature = "v1")]
struct ForwardCompatWorkflowBuilder<S: ForwardCompatState> {
    _state: PhantomData<S>,
    tracking_data: Option<PaymentMethodModularCompatTrackingData>,
    merchant_id: Option<common_utils::id_type::MerchantId>,
    key_store: Option<domain::MerchantKeyStore>,
    merchant_account: Option<domain::MerchantAccount>,
    payment_method: Option<domain::PaymentMethod>,
    updates: Option<ForwardCompatUpdates>,
}

#[cfg(feature = "v1")]
trait ForwardCompatState {}

#[cfg(feature = "v1")]
trait ForwardCompatTransitionTo<S: ForwardCompatState> {}

#[cfg(feature = "v1")]
struct ForwardCompatStarted;
#[cfg(feature = "v1")]
struct ForwardTrackingLoaded;
#[cfg(feature = "v1")]
struct ForwardContextLoaded;
#[cfg(feature = "v1")]
struct ForwardPaymentMethodLoaded;
#[cfg(feature = "v1")]
struct ForwardDbCompatPrepared;
#[cfg(feature = "v1")]
struct ForwardLockerCompatApplied;

#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardCompatStarted {}
#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardTrackingLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardContextLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardPaymentMethodLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardDbCompatPrepared {}
#[cfg(feature = "v1")]
impl ForwardCompatState for ForwardLockerCompatApplied {}

#[cfg(feature = "v1")]
impl ForwardCompatTransitionTo<ForwardTrackingLoaded> for ForwardCompatStarted {}
#[cfg(feature = "v1")]
impl ForwardCompatTransitionTo<ForwardContextLoaded> for ForwardTrackingLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatTransitionTo<ForwardPaymentMethodLoaded> for ForwardContextLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatTransitionTo<ForwardDbCompatPrepared> for ForwardPaymentMethodLoaded {}
#[cfg(feature = "v1")]
impl ForwardCompatTransitionTo<ForwardLockerCompatApplied> for ForwardDbCompatPrepared {}

#[cfg(feature = "v1")]
impl<S: ForwardCompatState> ForwardCompatWorkflowBuilder<S> {
    fn transition<T: ForwardCompatState>(self) -> ForwardCompatWorkflowBuilder<T>
    where
        S: ForwardCompatTransitionTo<T>,
    {
        ForwardCompatWorkflowBuilder {
            _state: PhantomData,
            tracking_data: self.tracking_data,
            merchant_id: self.merchant_id,
            key_store: self.key_store,
            merchant_account: self.merchant_account,
            payment_method: self.payment_method,
            updates: self.updates,
        }
    }

    fn tracking_data(&self) -> errors::RouterResult<&PaymentMethodModularCompatTrackingData> {
        self.tracking_data
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("tracking data must be loaded in forward compatibility workflow")
    }

    fn merchant_id(&self) -> errors::RouterResult<&common_utils::id_type::MerchantId> {
        self.merchant_id
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("merchant id must be loaded in forward compatibility workflow")
    }

    fn key_store(&self) -> errors::RouterResult<&domain::MerchantKeyStore> {
        self.key_store
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("merchant key store must be loaded in forward compatibility workflow")
    }

    fn merchant_account(&self) -> errors::RouterResult<&domain::MerchantAccount> {
        self.merchant_account
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("merchant account must be loaded in forward compatibility workflow")
    }

    fn payment_method(&self) -> errors::RouterResult<&domain::PaymentMethod> {
        self.payment_method
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("payment method must be loaded in forward compatibility workflow")
    }

    fn updates(&self) -> errors::RouterResult<&ForwardCompatUpdates> {
        self.updates
            .as_ref()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("updates must be prepared in forward compatibility workflow")
    }

    fn updates_mut(&mut self) -> errors::RouterResult<&mut ForwardCompatUpdates> {
        self.updates
            .as_mut()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("updates must be prepared in forward compatibility workflow")
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardCompatStarted> {
    fn new() -> Self {
        Self {
            _state: PhantomData,
            tracking_data: None,
            merchant_id: None,
            key_store: None,
            merchant_account: None,
            payment_method: None,
            updates: None,
        }
    }

    fn load_tracking_data(
        mut self,
        tracking_data: PaymentMethodModularCompatTrackingData,
    ) -> ForwardCompatWorkflowBuilder<ForwardTrackingLoaded> {
        self.merchant_id = Some(tracking_data.merchant_id.clone());
        self.tracking_data = Some(tracking_data);
        self.transition()
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardTrackingLoaded> {
    fn load_context(
        mut self,
        key_store: domain::MerchantKeyStore,
        merchant_account: domain::MerchantAccount,
    ) -> ForwardCompatWorkflowBuilder<ForwardContextLoaded> {
        self.key_store = Some(key_store);
        self.merchant_account = Some(merchant_account);
        self.transition()
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardContextLoaded> {
    fn load_payment_method(
        mut self,
        payment_method: domain::PaymentMethod,
    ) -> ForwardCompatWorkflowBuilder<ForwardPaymentMethodLoaded> {
        self.payment_method = Some(payment_method);
        self.transition()
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardPaymentMethodLoaded> {
    fn is_complete(&self) -> errors::RouterResult<bool> {
        let payment_method = self.payment_method()?;
        Ok(payment_method.version == common_enums::ApiVersion::V2
            && payment_method.compatibility_updated_at == Some(payment_method.last_modified))
    }

    fn prepare_db_compat(
        mut self,
    ) -> errors::RouterResult<ForwardCompatWorkflowBuilder<ForwardDbCompatPrepared>> {
        let connector_mandate_details =
            diesel_models::payment_method::CommonMandateReference::parse_connector_mandate_compat_details(
                self.payment_method()?.connector_mandate_details.clone(),
            )
            .map(diesel_models::payment_method::CommonMandateReference::add_v2_connector_mandate_fields);
        self.updates = Some(ForwardCompatUpdates {
            connector_mandate_details,
            ..Default::default()
        });
        Ok(self.transition())
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardDbCompatPrepared> {
    async fn apply_locker_compat(
        mut self,
        state: &SessionState,
        process_id: &str,
    ) -> Result<ForwardCompatWorkflowBuilder<ForwardLockerCompatApplied>, errors::ProcessTrackerError>
    {
        let merchant_id = self.merchant_id()?.clone();
        let payment_method = self.payment_method()?.clone();

        if payment_method.payment_method == Some(common_enums::PaymentMethod::Card) {
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
                let pmd = hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(
                    card_detail.clone(),
                );

                let customer_id_key = customer_id.get_string_repr().to_owned();
                let locker_fingerprint_id = Self::get_vault_fingerprint_id(
                    state,
                    customer_id_key.clone(),
                    pmd.to_fingerprint_data(),
                )
                .await?;
                let auxiliary_fingerprint_id = Self::get_vault_fingerprint_id(
                    state,
                    customer_id_key,
                    pmd.to_auxiliary_fingerprint_data(),
                )
                .await?;

                self.updates_mut()?.locker_fingerprint_id = Some(locker_fingerprint_id);
                self.updates_mut()?.auxiliary_fingerprint_id = Some(auxiliary_fingerprint_id);

                Self::upsert_payment_method_to_generic_vault(
                    state,
                    &merchant_id,
                    &customer_id,
                    domain::VaultId::generate(card_reference),
                    &pmd,
                )
                .await?;
                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Upserted card into generic locker in modular compatibility PT"
                );
            } else {
                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Skipping card migration in modular compatibility PT as locker_id is absent"
                );
            }

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
                        network_token_details,
                    );

                Self::upsert_payment_method_to_generic_vault(
                    state,
                    &merchant_id,
                    &customer_id,
                    domain::VaultId::generate(network_token_locker_id),
                    &network_token_pmd,
                )
                .await?;

                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Upserted network token into generic locker in modular compatibility PT"
                );
            } else {
                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%payment_method.payment_method_id,
                    "Skipping network token migration in modular compatibility PT as network_token_locker_id is absent"
                );
            }
        } else {
            logger::info!(
                process_id=%process_id,
                payment_method_id=%payment_method.payment_method_id,
                "Payment method is non-card; skipping locker migration in modular compatibility PT"
            );
        }

        Ok(self.transition())
    }

    async fn get_vault_fingerprint_id(
        state: &SessionState,
        key: String,
        data: impl serde::Serialize,
    ) -> Result<String, errors::ProcessTrackerError> {
        let data = serde_json::to_string(&data)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to serialize vault fingerprint data")?;

        let payload = ForwardCompatVaultFingerprintRequest { data, key }
            .encode_to_vec()
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode VaultFingerprintRequest")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let resp =
            vault::call_to_vault::<pm_types::GetVaultFingerprint>(state, payload, None, None)
                .await
                .change_context(errors::VaultError::VaultAPIError)
                .attach_printable("Call to vault failed")
                .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let fingerprint_resp: pm_types::VaultFingerprintResponse = resp
            .parse_struct("VaultFingerprintResponse")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Failed to parse data into VaultFingerprintResponse")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        Ok(fingerprint_resp.fingerprint_id)
    }

    async fn upsert_payment_method_to_generic_vault(
        state: &SessionState,
        merchant_id: &common_utils::id_type::MerchantId,
        customer_id: &common_utils::id_type::CustomerId,
        vault_id: domain::VaultId,
        data: &hyperswitch_domain_models::vault::PaymentMethodVaultingData,
    ) -> Result<(), errors::ProcessTrackerError> {
        let should_trigger_fingerprint_migration =
            payment_method_utils::get_should_trigger_fingerprint_migration(
                state,
                Some(customer_id),
                hyperswitch_domain_models::platform::ProviderMerchantId::new(
                    merchant_id.clone(),
                ),
            )
            .await;

        let payload = cards::encode_add_vault_request(
            should_trigger_fingerprint_migration,
            merchant_id.clone(),
            customer_id,
            data.clone(),
            state.conf.locker.ttl_for_storage_in_secs,
            Some(vault_id),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in generic locker in compatibility PT")?;

        let query_params = Some(pm_types::VaultQueryParam::from(pm_types::WriteMode::Upsert));

        let resp = vault::call_to_vault::<pm_types::AddVault>(state, payload, query_params, None)
            .await
            .change_context(errors::VaultError::VaultAPIError)
            .attach_printable("Call to vault failed")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to add payment method in generic locker in compatibility PT",
            )?;

        let _vault_id = cards::parse_add_vault_response(should_trigger_fingerprint_migration, resp)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to add payment method in generic locker in compatibility PT",
            )?;

        Ok(())
    }
}

#[cfg(feature = "v1")]
impl ForwardCompatWorkflowBuilder<ForwardLockerCompatApplied> {
    async fn mark_complete(
        self,
        db: &dyn StorageInterface,
    ) -> Result<(), errors::ProcessTrackerError> {
        let tracking_data = self.tracking_data()?;
        let payment_method = self.payment_method()?;
        let updates = self.updates()?;

        let pm_update = storage::PaymentMethodUpdate::PopulateModularCompatFields {
            id: tracking_data.payment_method_id.clone(),
            payment_method_type_v2: payment_method.payment_method,
            payment_method_subtype: payment_method.payment_method_type,
            connector_mandate_details: updates
                .connector_mandate_details
                .clone()
                .and_then(|connector_mandate_details| connector_mandate_details.into_value()),
            locker_fingerprint_id: updates.locker_fingerprint_id.clone(),
            auxiliary_fingerprint_id: updates.auxiliary_fingerprint_id.clone(),
            last_modified_by: tracking_data.last_modified_by.clone(),
        };

        db.update_payment_method(
            self.key_store()?,
            payment_method.clone(),
            pm_update,
            self.merchant_account()?.storage_scheme,
            // Forward compat completion update must not recursively enqueue another compat PT.
            None,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to populate modular fields for payment method in compatibility PT",
        )?;

        Ok(())
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, serde::Serialize)]
struct ForwardCompatVaultFingerprintRequest {
    data: String,
    key: String,
}

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodModularForwardCompatWorkflow {
    #[cfg(feature = "v1")]
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!(process_id=%process.id, "Starting payment method modular compatibility PT");
        let db = &*state.store;
        let tracking_data: PaymentMethodModularCompatTrackingData =
            process
                .tracking_data
                .clone()
                .parse_value("PaymentMethodModularCompatTrackingData")?;
        logger::info!(process_id=%process.id, ?tracking_data, "Parsed modular compatibility PT tracking data");
        let workflow = ForwardCompatWorkflowBuilder::new().load_tracking_data(tracking_data);

        let merchant_id = workflow.merchant_id()?.clone();
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
        let workflow = workflow.load_context(key_store, merchant_account);

        let payment_method = db
            .find_payment_method(
                workflow.key_store()?,
                &workflow.tracking_data()?.payment_method_id,
                workflow.merchant_account()?.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to fetch payment method for modular compatibility PT")?;
        let workflow = workflow.load_payment_method(payment_method);

        if workflow.is_complete()? {
            db.as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                .await?;
            crate::logger::info!(
                business_status = "COMPLETED_BY_PT",
                "Finished payment method modular compatibility PT; already forward compatible"
            );
        } else {
            workflow
                .prepare_db_compat()?
                .apply_locker_compat(state, &process.id)
                .await?
                .mark_complete(db)
                .await?;

            db.as_scheduler()
                .finish_process_with_business_status(process, "COMPLETED_BY_PT")
                .await?;
            crate::logger::info!(
                business_status = "COMPLETED_BY_PT",
                "Finished payment method modular compatibility PT"
            );
        }

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
