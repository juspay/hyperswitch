use std::marker::PhantomData;

#[cfg(feature = "v1")]
use api_models::payment_methods::Card;
use common_utils::{
    ext_traits::{OptionExt, StringExt, ValueExt},
    id_type,
};
use error_stack::ResultExt;
use hyperswitch_masking::PeekInterface;
use scheduler::{
    consumer::types::process_data, utils as pt_utils, workflows::ProcessTrackerWorkflow,
};

#[cfg(feature = "v1")]
use crate::core::payment_methods::transformers;
#[cfg(feature = "v2")]
use crate::core::payment_methods::{
    add_payment_method_to_legacy_locker, update_metadata_changed_payment_method_in_legacy_locker,
};
use crate::{
    core::payment_methods::{cards, utils as payment_method_utils, vault},
    errors,
    logger::{self, error},
    routes::{app::StorageInterface, SessionState},
    types::{
        domain, payment_methods as pm_types,
        storage::{self, PaymentMethodModularCompatTrackingData},
    },
};

#[cfg(feature = "v1")]
struct BackwardCompatUpdates {
    payment_method: Option<common_enums::PaymentMethod>,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    connector_mandate_details: Option<serde_json::Value>,
}

#[cfg(feature = "v2")]
struct BackwardCompatUpdates {
    payment_method: Option<common_enums::PaymentMethod>,
    payment_method_type: Option<common_enums::PaymentMethodType>,
    connector_mandate_details: Option<diesel_models::payment_method::CommonMandateReference>,
}

struct BackwardCompatWorkflowBuilder<S: BackwardCompatState> {
    _state: PhantomData<S>,
    tracking_data: Option<PaymentMethodModularCompatTrackingData>,
    merchant_id: Option<id_type::MerchantId>,
    key_store: Option<domain::MerchantKeyStore>,
    merchant_account: Option<domain::MerchantAccount>,
    payment_method: Option<domain::PaymentMethod>,
    updates: Option<BackwardCompatUpdates>,
}

trait BackwardCompatState {}

trait BackwardCompatTransitionTo<S: BackwardCompatState> {}

struct BackwardCompatStarted;
struct BackwardTrackingLoaded;
struct BackwardContextLoaded;
struct BackwardPaymentMethodLoaded;
struct BackwardDbCompatPrepared;
struct BackwardLockerCompatApplied;

impl BackwardCompatState for BackwardCompatStarted {}
impl BackwardCompatState for BackwardTrackingLoaded {}
impl BackwardCompatState for BackwardContextLoaded {}
impl BackwardCompatState for BackwardPaymentMethodLoaded {}
impl BackwardCompatState for BackwardDbCompatPrepared {}
impl BackwardCompatState for BackwardLockerCompatApplied {}

impl BackwardCompatTransitionTo<BackwardTrackingLoaded> for BackwardCompatStarted {}
impl BackwardCompatTransitionTo<BackwardContextLoaded> for BackwardTrackingLoaded {}
impl BackwardCompatTransitionTo<BackwardPaymentMethodLoaded> for BackwardContextLoaded {}
impl BackwardCompatTransitionTo<BackwardDbCompatPrepared> for BackwardPaymentMethodLoaded {}
impl BackwardCompatTransitionTo<BackwardLockerCompatApplied> for BackwardDbCompatPrepared {}

#[allow(clippy::expect_used)]
impl<S: BackwardCompatState> BackwardCompatWorkflowBuilder<S> {
    fn transition<T: BackwardCompatState>(self) -> BackwardCompatWorkflowBuilder<T>
    where
        S: BackwardCompatTransitionTo<T>,
    {
        BackwardCompatWorkflowBuilder {
            _state: PhantomData,
            tracking_data: self.tracking_data,
            merchant_id: self.merchant_id,
            key_store: self.key_store,
            merchant_account: self.merchant_account,
            payment_method: self.payment_method,
            updates: self.updates,
        }
    }

    fn tracking_data(&self) -> &PaymentMethodModularCompatTrackingData {
        self.tracking_data
            .as_ref()
            .expect("tracking data must be loaded in backward compatibility workflow")
    }

    fn merchant_id(&self) -> &id_type::MerchantId {
        self.merchant_id
            .as_ref()
            .expect("merchant id must be loaded in backward compatibility workflow")
    }

    fn key_store(&self) -> &domain::MerchantKeyStore {
        self.key_store
            .as_ref()
            .expect("merchant key store must be loaded in backward compatibility workflow")
    }

    fn merchant_account(&self) -> &domain::MerchantAccount {
        self.merchant_account
            .as_ref()
            .expect("merchant account must be loaded in backward compatibility workflow")
    }

    fn payment_method(&self) -> &domain::PaymentMethod {
        self.payment_method
            .as_ref()
            .expect("payment method must be loaded in backward compatibility workflow")
    }
}

impl BackwardCompatWorkflowBuilder<BackwardCompatStarted> {
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
    ) -> BackwardCompatWorkflowBuilder<BackwardTrackingLoaded> {
        self.merchant_id = Some(tracking_data.merchant_id.clone());
        self.tracking_data = Some(tracking_data);
        self.transition()
    }
}

impl BackwardCompatWorkflowBuilder<BackwardTrackingLoaded> {
    fn load_context(
        mut self,
        key_store: domain::MerchantKeyStore,
        merchant_account: domain::MerchantAccount,
    ) -> BackwardCompatWorkflowBuilder<BackwardContextLoaded> {
        self.key_store = Some(key_store);
        self.merchant_account = Some(merchant_account);
        self.transition()
    }
}

impl BackwardCompatWorkflowBuilder<BackwardContextLoaded> {
    fn load_payment_method(
        mut self,
        payment_method: domain::PaymentMethod,
    ) -> BackwardCompatWorkflowBuilder<BackwardPaymentMethodLoaded> {
        self.payment_method = Some(payment_method);
        self.transition()
    }
}

impl BackwardCompatWorkflowBuilder<BackwardPaymentMethodLoaded> {
    fn should_skip(&self) -> bool {
        self.payment_method().version == common_enums::ApiVersion::V1
    }

    fn prepare_db_compat(mut self) -> BackwardCompatWorkflowBuilder<BackwardDbCompatPrepared> {
        let updates = BackwardCompatUpdates {
            payment_method: self.payment_method().get_payment_method_type(),
            payment_method_type: self.payment_method().get_payment_method_subtype(),
            connector_mandate_details: Self::connector_mandate_details(self.payment_method()),
        };
        self.updates = Some(updates);
        self.transition()
    }

    #[cfg(feature = "v1")]
    fn connector_mandate_details(
        payment_method: &domain::PaymentMethod,
    ) -> Option<serde_json::Value> {
        diesel_models::payment_method::CommonMandateReference::add_v1_connector_mandate_fields(
            payment_method.connector_mandate_details.clone(),
        )
    }

    #[cfg(feature = "v2")]
    fn connector_mandate_details(
        payment_method: &domain::PaymentMethod,
    ) -> Option<diesel_models::payment_method::CommonMandateReference> {
        payment_method
            .connector_mandate_details
            .clone()
            .map(Into::into)
    }
}

impl BackwardCompatWorkflowBuilder<BackwardDbCompatPrepared> {
    #[cfg(feature = "v1")]
    async fn apply_locker_compat(
        self,
        state: &SessionState,
        db: &dyn StorageInterface,
        process_id: &str,
    ) -> Result<
        BackwardCompatWorkflowBuilder<BackwardLockerCompatApplied>,
        errors::ProcessTrackerError,
    > {
        Self::backfill_legacy_locker_card(
            state,
            db,
            self.key_store(),
            self.merchant_id(),
            self.payment_method(),
            self.merchant_account().storage_scheme,
            self.tracking_data(),
            process_id,
        )
        .await?;

        Ok(self.transition())
    }

    #[cfg(feature = "v2")]
    async fn apply_locker_compat(
        self,
        state: &SessionState,
        _db: &dyn StorageInterface,
        process_id: &str,
    ) -> Result<
        BackwardCompatWorkflowBuilder<BackwardLockerCompatApplied>,
        errors::ProcessTrackerError,
    > {
        let platform = domain::Platform::new(
            self.merchant_account().clone(),
            self.key_store().clone(),
            self.merchant_account().clone(),
            self.key_store().clone(),
            None,
        );

        Box::pin(Self::backfill_legacy_locker_card(
            state,
            &platform,
            self.payment_method(),
            self.tracking_data(),
            process_id,
        ))
        .await?;

        Ok(self.transition())
    }
}

#[allow(clippy::expect_used)]
impl BackwardCompatWorkflowBuilder<BackwardLockerCompatApplied> {
    async fn mark_complete(
        self,
        db: &dyn StorageInterface,
    ) -> Result<domain::PaymentMethod, errors::ProcessTrackerError> {
        let Self {
            tracking_data,
            key_store,
            merchant_account,
            payment_method,
            updates,
            ..
        } = self;
        let tracking_data =
            tracking_data.expect("tracking data must be loaded in backward compatibility workflow");
        let key_store = key_store
            .expect("merchant key store must be loaded in backward compatibility workflow");
        let merchant_account = merchant_account
            .expect("merchant account must be loaded in backward compatibility workflow");
        let payment_method = payment_method
            .expect("payment method must be loaded in backward compatibility workflow");
        let updates = updates.expect("updates must be prepared in backward compatibility workflow");

        let pm_update = storage::PaymentMethodUpdate::PopulateLegacyCompatFields {
            payment_method: updates.payment_method,
            payment_method_type: updates.payment_method_type,
            connector_mandate_details: updates.connector_mandate_details,
            last_modified_by: tracking_data.last_modified_by,
        };

        Ok(db
            .update_payment_method(
                &key_store,
                payment_method,
                pm_update,
                merchant_account.storage_scheme,
                // Backward compat completion update must not recursively enqueue compat again.
                None,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to populate legacy payment method fields in backward compatibility flow",
            )?)
    }
}

#[cfg(feature = "v1")]
impl BackwardCompatWorkflowBuilder<BackwardDbCompatPrepared> {
    #[allow(clippy::too_many_arguments)]
    async fn backfill_legacy_locker_card(
        state: &SessionState,
        db: &dyn StorageInterface,
        key_store: &domain::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        payment_method: &domain::PaymentMethod,
        storage_scheme: common_enums::MerchantStorageScheme,
        tracking_data: &PaymentMethodModularCompatTrackingData,
        process_id: &str,
    ) -> Result<(), errors::ProcessTrackerError> {
        let legacy_locker_skip_reason = match (
            payment_method.get_payment_method_type(),
            payment_method.locker_id.as_ref(),
            payment_method.customer_id.as_ref(),
        ) {
            (Some(common_enums::PaymentMethod::Card), Some(_), Some(_)) => None,
            (Some(common_enums::PaymentMethod::Card), None, _) => {
                Some("locker reference is missing")
            }
            (Some(common_enums::PaymentMethod::Card), Some(_), None) => {
                Some("customer_id is missing")
            }
            _ => Some("payment method is not card"),
        };

        if let Some(skip_reason) = legacy_locker_skip_reason {
            logger::info!(
                process_id=%process_id,
                payment_method_id=%tracking_data.payment_method_id,
                skip_reason,
                "Skipping legacy locker card backfill in modular backward compatibility PT"
            );
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

            let legacy_card_exists = match cards::get_card_from_vault(
                state,
                &customer_id,
                merchant_id,
                &card_reference,
            )
            .await
            {
                Ok(_) => {
                    logger::info!(
                        process_id=%process_id,
                        payment_method_id=%tracking_data.payment_method_id,
                        card_reference=%card_reference,
                        "Skipping legacy locker write in modular backward compatibility PT because card already exists"
                    );
                    true
                }
                Err(err) => {
                    logger::info!(
                        ?err,
                        process_id=%process_id,
                        payment_method_id=%tracking_data.payment_method_id,
                        card_reference=%card_reference,
                        "Legacy locker card not found or not readable in modular backward compatibility PT; proceeding with legacy locker upsert"
                    );
                    false
                }
            };

            if !legacy_card_exists {
                let should_trigger_fingerprint_migration =
                    payment_method_utils::get_should_trigger_fingerprint_migration(
                        state,
                        Some(&customer_id),
                        hyperswitch_domain_models::platform::ProviderMerchantId::from_merchant_id(
                            merchant_id.clone(),
                        ),
                    )
                    .await;

                let payload = cards::encode_vault_retrieve_request(
                    should_trigger_fingerprint_migration,
                    merchant_id.clone(),
                    &customer_id,
                    &card_reference,
                )
                .attach_printable(
                    "Failed to encode generic locker retrieve request in backward compatibility PT",
                )?;
                let vault_response = vault::call_to_vault::<pm_types::VaultRetrieve>(
                    state, payload, None, None,
                )
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
                let locker_card_detail =
                    if let hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card) =
                        stored_pm_resp.data
                    {
                        card
                    } else {
                        Err(
                        error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable(
                            "Generic locker returned non-card data in backward compatibility PT",
                        ),
                    )?
                    };

                let card_isin = locker_card_detail.card_number.get_card_isin();
                let locker_req =
                    transformers::StoreLockerReq::LockerCard(transformers::StoreCardReq {
                        merchant_id: merchant_id.clone(),
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

                let add_card_resp = cards::add_card_to_vault(state, &locker_req, &customer_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to add card to legacy locker in backward compatibility PT",
                    )?;

                if matches!(
                    add_card_resp.duplication_check,
                    Some(transformers::DataDuplicationCheck::MetaDataChanged)
                ) {
                    let older_payment_method = db
                        .find_payment_method_by_locker_id(
                            key_store,
                            &add_card_resp.card_reference,
                            storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Failed to fetch legacy PM referencing metadata-changed locker id",
                        )?;

                    let updated_card_resp = cards::update_metadata_changed_card_in_vault(
                    state,
                    &customer_id,
                    merchant_id,
                    &add_card_resp.card_reference,
                    &locker_req,
                )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to update metadata-changed card in legacy locker during backward compatibility PT",
                    )?;

                    if older_payment_method.locker_id.as_deref()
                        != Some(updated_card_resp.card_reference.as_str())
                    {
                        let pm_update = storage::PaymentMethodUpdate::AdditionalDataUpdate {
                            payment_method_data: None,
                            status: None,
                            locker_id: Some(updated_card_resp.card_reference.clone()),
                            payment_method: None,
                            payment_method_type: None,
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
                        key_store,
                        older_payment_method,
                        pm_update,
                        storage_scheme,
                        // Metadata-change locker reconciliation is already inside backward compat.
                        None,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to update legacy PM locker id after metadata-changed backward compatibility PT",
                    )?;
                    }

                    logger::info!(
                        process_id=%process_id,
                        payment_method_id=%tracking_data.payment_method_id,
                        old_card_reference=%add_card_resp.card_reference,
                        new_card_reference=%updated_card_resp.card_reference,
                        "Reinserted metadata-changed card and updated referencing PM in backward compatibility PT"
                    );
                }

                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%tracking_data.payment_method_id,
                    "Upserted card into legacy locker in modular backward compatibility PT"
                );
            }
        }

        Ok(())
    }
}

#[cfg(feature = "v2")]
impl BackwardCompatWorkflowBuilder<BackwardDbCompatPrepared> {
    async fn backfill_legacy_locker_card(
        state: &SessionState,
        platform: &domain::Platform,
        payment_method: &domain::PaymentMethod,
        tracking_data: &PaymentMethodModularCompatTrackingData,
        process_id: &str,
    ) -> Result<(), errors::ProcessTrackerError> {
        let legacy_locker_skip_reason = match (
            payment_method.get_payment_method_type(),
            payment_method.locker_id.as_ref(),
            payment_method.customer_id.as_ref(),
        ) {
            (Some(common_enums::PaymentMethod::Card), Some(_), Some(_)) => None,
            (Some(common_enums::PaymentMethod::Card), None, _) => {
                Some("locker reference is missing")
            }
            (Some(common_enums::PaymentMethod::Card), Some(_), None) => {
                Some("customer_id is missing")
            }
            _ => Some("payment method is not card"),
        };

        if let Some(skip_reason) = legacy_locker_skip_reason {
            logger::info!(
                process_id=%process_id,
                payment_method_id=%tracking_data.payment_method_id,
                skip_reason,
                "Skipping legacy locker card backfill in modular backward compatibility inline flow"
            );
        } else {
            let pm_customer_id = payment_method
                .customer_id
                .clone()
                .get_required_value("customer_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "customer_id not found for card payment method in backward compatibility inline flow",
                )?;

            let customer_id = id_type::CustomerId::try_from(pm_customer_id.clone())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to convert global customer id for backward compatibility inline flow",
                )?;

            let card_reference = payment_method
                .locker_id
                .clone()
                .get_required_value("locker_id")?
                .get_string_repr()
                .to_owned();

            let legacy_card_exists = match cards::get_card_from_locker(
                state,
                &customer_id,
                platform.get_provider().get_account().get_id(),
                &card_reference,
            )
            .await
            {
                Ok(_) => {
                    logger::info!(
                        process_id=%process_id,
                        payment_method_id=%tracking_data.payment_method_id,
                        card_reference=%card_reference,
                        "Skipping legacy locker write in modular backward compatibility inline flow because card already exists"
                    );
                    true
                }
                Err(err) => {
                    logger::info!(
                        ?err,
                        process_id=%process_id,
                        payment_method_id=%tracking_data.payment_method_id,
                        card_reference=%card_reference,
                        "Legacy locker card not found or not readable in modular backward compatibility inline flow; proceeding with legacy locker upsert"
                    );
                    false
                }
            };

            if !legacy_card_exists {
                let should_trigger_fingerprint_migration =
                    payment_method_utils::get_should_trigger_fingerprint_migration(
                        state,
                        None,
                        platform.get_provider().get_provider_merchant_id(),
                    )
                    .await;

                let payload = cards::encode_vault_retrieve_request(
                    should_trigger_fingerprint_migration,
                    platform.get_provider().get_account().get_id().clone(),
                    &pm_customer_id,
                    &card_reference,
                )
                .attach_printable(
                    "Failed to encode generic locker retrieve request in backward compatibility inline flow",
                )?;
                let vault_response =
                    vault::call_to_vault::<pm_types::VaultRetrieve>(state, payload, None, None)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(
                "Failed to retrieve card from generic locker in backward compatibility inline flow",
            )?;
                let stored_pm_resp: pm_types::VaultRetrieveResponse = vault_response
                .parse_struct("VaultRetrieveResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to parse generic locker retrieve response in backward compatibility inline flow",
                )?;
                if !matches!(
                    stored_pm_resp.data,
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(_)
                ) {
                    Err(
                    error_stack::report!(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable(
                            "Generic locker returned non-card data in backward compatibility inline flow",
                        ),
                )?
                }

                let add_card_resp = add_payment_method_to_legacy_locker(
                    state,
                    platform,
                    &stored_pm_resp.data,
                    Some(domain::VaultId::generate(card_reference.clone())),
                    &pm_customer_id,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to add card to legacy locker in backward compatibility inline flow",
                )?;

                if matches!(
                add_card_resp.duplication_check,
                Some(crate::core::payment_methods::transformers::DataDuplicationCheck::MetaDataChanged)
            ) {
                let db = &*state.store;
                let old_card_reference = add_card_resp.card_reference;
                let older_payment_method = db
                    .find_payment_method_by_locker_id(
                        platform.get_provider().get_key_store(),
                        &old_card_reference,
                        platform.get_provider().get_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to fetch legacy PM referencing metadata-changed locker id",
                    )?;

                let updated_card_resp = update_metadata_changed_payment_method_in_legacy_locker(
                    state,
                    platform,
                    &stored_pm_resp.data,
                    Some(domain::VaultId::generate(card_reference.clone())),
                    &pm_customer_id,
                    old_card_reference.clone(),
                )
                .await
                .attach_printable(
                    "Failed to update metadata-changed card in legacy locker during backward compatibility inline flow",
                )?;

                let updated_card_reference = updated_card_resp.vault_id.get_string_repr().to_owned();
                let older_locker_id = older_payment_method
                    .locker_id
                    .as_ref()
                    .map(|locker_id| locker_id.get_string_repr().to_owned());

                if older_locker_id.as_deref() != Some(updated_card_reference.as_str()) {
                    let pm_update = storage::PaymentMethodUpdate::GenericUpdate {
                        payment_method_data: None,
                        status: None,
                        locker_id: Some(updated_card_reference.clone()),
                        payment_method_type_v2: None,
                        payment_method_subtype: None,
                        network_token_requestor_reference_id: None,
                        network_token_locker_id: None,
                        network_token_payment_method_data: None,
                        locker_fingerprint_id: None,
                        connector_mandate_details: Box::new(None),
                        external_vault_source: None,
                        network_transaction_id: None,
                        network_transaction_link_id: None,
                        last_modified_by: tracking_data.last_modified_by.clone(),
                    };

                    db.update_payment_method(
                        platform.get_provider().get_key_store(),
                        older_payment_method,
                        pm_update,
                        platform.get_provider().get_account().storage_scheme,
                        // Backfill of the older PM is already part of backward compat execution.
                        None,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to update legacy PM locker id after metadata-changed backward compatibility inline flow",
                    )?;
                }

                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%tracking_data.payment_method_id,
                    old_card_reference=%old_card_reference,
                    new_card_reference=%updated_card_reference,
                    "Reinserted metadata-changed card and updated referencing PM in backward compatibility inline flow"
                );
            }

                logger::info!(
                    process_id=%process_id,
                    payment_method_id=%tracking_data.payment_method_id,
                    "Upserted card into legacy locker in modular backward compatibility inline flow"
                );
            }
        }

        Ok(())
    }
}

pub async fn run_payment_method_modular_backward_compat_backfill(
    state: &SessionState,
    tracking_data: PaymentMethodModularCompatTrackingData,
    process_id: &str,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let workflow = BackwardCompatWorkflowBuilder::new().load_tracking_data(tracking_data);
    let merchant_id = workflow.merchant_id().clone();
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

    #[cfg(feature = "v1")]
    let payment_method_id = workflow.tracking_data().payment_method_id.as_str();
    #[cfg(feature = "v2")]
    let payment_method_id = id_type::GlobalPaymentMethodId::generate_from_string(
        workflow.tracking_data().payment_method_id.clone(),
    );

    let payment_method = db
        .find_payment_method(
            workflow.key_store(),
            #[cfg(feature = "v1")]
            payment_method_id,
            #[cfg(feature = "v2")]
            &payment_method_id,
            workflow.merchant_account().storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to fetch payment method for modular backward compatibility backfill",
        )?;
    let workflow = workflow.load_payment_method(payment_method);

    if workflow.should_skip() {
        logger::info!(
            process_id=%process_id,
            payment_method_id=%workflow.tracking_data().payment_method_id,
            "Skipping modular backward compatibility backfill because legacy PM is not forward compatible"
        );
    } else {
        workflow
            .prepare_db_compat()
            .apply_locker_compat(state, db, process_id)
            .await?
            .mark_complete(db)
            .await?;
    }

    Ok(())
}

pub struct PaymentMethodModularBackwardCompatWorkflow;

#[async_trait::async_trait]
impl ProcessTrackerWorkflow<SessionState> for PaymentMethodModularBackwardCompatWorkflow {
    async fn execute_workflow<'a>(
        &'a self,
        state: &'a SessionState,
        process: storage::ProcessTracker,
    ) -> Result<(), errors::ProcessTrackerError> {
        logger::info!(process_id=%process.id, "Starting payment method modular backward compatibility PT");
        let tracking_data: PaymentMethodModularCompatTrackingData =
            process
                .tracking_data
                .clone()
                .parse_value("PaymentMethodModularCompatTrackingData")?;
        logger::info!(process_id=%process.id, ?tracking_data, "Parsed modular backward compatibility PT tracking data");

        Box::pin(run_payment_method_modular_backward_compat_backfill(
            state,
            tracking_data,
            &process.id,
        ))
        .await?;

        let db = &*state.store;
        db.as_scheduler()
            .finish_process_with_business_status(process, "COMPLETED_BY_PT")
            .await?;
        crate::logger::info!(
            business_status = "COMPLETED_BY_PT",
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
