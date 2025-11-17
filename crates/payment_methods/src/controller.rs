use std::fmt::Debug;

use crate::core::migration::validate_card_expiry;
use crate::{
    core::{cards, transformers::{self, DataDuplicationCheck}, errors, network_tokenization},
    helpers::{self, domain, StorageErrorExt},
    metrics, state,
};
use api_models::payouts::PayoutMethodData;
use common_utils::types::keymanager::Identifier;
use common_utils::types::keymanager::KeyManagerState;
use common_utils::encryption::Encryption;
use hyperswitch_domain_models::type_encryption::AsyncLift;
use crate::types::PaymentMethodCreateExt;
#[cfg(feature = "payouts")]
use api_models::payouts;
use api_models::{
    enums as api_enums,
    payment_methods::{self as api, Card, CardDetailsPaymentMethod, PaymentMethodsData},
};
#[cfg(feature = "v1")]
use common_enums::enums as common_enums;
use common_utils::crypto::Encryptable;
#[cfg(feature = "v2")]
use common_utils::encryption;
use common_utils::{
    consts, crypto,
    ext_traits::{self, AsyncExt},
    generate_id, id_type, type_name,
};
use error_stack::report;
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payment_methods::PaymentMethodVaultSourceDetails;
use hyperswitch_domain_models::{
    api as domain_api, customer::CustomerUpdate, ext_traits::OptionExt, merchant_context,
    merchant_key_store, payment_methods, type_encryption,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::{instrument, tracing, logger};
#[cfg(feature = "v1")]
use scheduler::errors as sch_errors;
use serde::{Deserialize, Serialize};
use storage_impl::{errors as storage_errors, payment_method};

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteCardResp {
    pub status: String,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
}

pub struct PmController<'a> {
    pub state: &'a state::PaymentMethodsState,
    pub merchant_context: &'a merchant_context::MerchantContext,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCardResponse {
    pub card_id: Option<String>,
    pub external_id: Option<String>,
    pub card_isin: Option<Secret<String>>,
    pub status: String,
}

#[async_trait::async_trait]
pub trait PaymentMethodsController {
    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn create_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        payment_method_id: &str,
        locker_id: Option<String>,
        merchant_id: &id_type::MerchantId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        payment_method_data: crypto::OptionalEncryptableValue,
        connector_mandate_details: Option<serde_json::Value>,
        status: Option<common_enums::PaymentMethodStatus>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        card_scheme: Option<String>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn insert_payment_method(
        &self,
        resp: &api::PaymentMethodResponse,
        req: &api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        locker_id: Option<String>,
        connector_mandate_details: Option<serde_json::Value>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn insert_payment_method(
        &self,
        resp: &api::PaymentMethodResponse,
        req: &api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        locker_id: Option<String>,
        connector_mandate_details: Option<serde_json::Value>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: Option<encryption::Encryption>,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v1")]
    async fn add_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
    ) -> errors::PmResponse<api::PaymentMethodResponse>;

    #[cfg(feature = "v1")]
    async fn retrieve_payment_method(
        &self,
        pm: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodResponse>;

    #[cfg(feature = "v1")]
    async fn delete_payment_method(
        &self,
        pm_id: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodDeleteResponse>;

    async fn add_card_hs(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        locker_choice: api_enums::LockerChoice,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    /// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
    async fn add_card_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(feature = "payouts")]
    async fn add_bank_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        bank: &payouts::Bank,
        customer_id: &id_type::CustomerId,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)>;

    #[cfg(feature = "v1")]
    async fn get_or_insert_payment_method(
        &self,
        req: api::PaymentMethodCreate,
        resp: &mut api::PaymentMethodResponse,
        customer_id: &id_type::CustomerId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod>;

    #[cfg(feature = "v2")]
    async fn get_or_insert_payment_method(
        &self,
        _req: api::PaymentMethodCreate,
        _resp: &mut api::PaymentMethodResponse,
        _customer_id: &id_type::CustomerId,
        _key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn get_card_details_with_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<Option<api::CardDetailFromLocker>>;

    #[cfg(feature = "v1")]
    async fn get_card_details_without_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker>;

    async fn delete_card_from_locker(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        card_reference: &str,
    ) -> errors::PmResult<DeleteCardResp>;

    #[cfg(feature = "v1")]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>);

    #[cfg(feature = "v2")]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>);

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn save_network_token_and_update_payment_method(
        &self,
        req: &api::PaymentMethodMigrate,
        key_store: &merchant_key_store::MerchantKeyStore,
        network_token_data: &api_models::payment_methods::MigrateNetworkTokenData,
        network_token_requestor_ref_id: String,
        pm_id: String,
    ) -> errors::PmResult<bool>;

    #[cfg(feature = "v1")]
    async fn set_default_payment_method(
        &self,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        payment_method_id: String,
    ) -> errors::PmResponse<api_models::payment_methods::CustomerDefaultPaymentMethodResponse>;

    #[cfg(feature = "v1")]
    async fn add_payment_method_status_update_task(
        &self,
        payment_method: &payment_methods::PaymentMethod,
        prev_status: common_enums::PaymentMethodStatus,
        curr_status: common_enums::PaymentMethodStatus,
        merchant_id: &id_type::MerchantId,
    ) -> Result<(), sch_errors::ProcessTrackerError>;

    #[cfg(feature = "v1")]
    async fn validate_merchant_connector_ids_in_connector_mandate_details(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
        merchant_id: &id_type::MerchantId,
        card_network: Option<common_enums::CardNetwork>,
    ) -> errors::PmResult<()>;

    #[cfg(feature = "v1")]
    async fn get_card_details_from_locker(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker>;
}

pub async fn create_encrypted_data<T>(
    key_manager_state: &KeyManagerState,
    key_store: &merchant_key_store::MerchantKeyStore,
    data: T,
) -> Result<
    Encryptable<Secret<serde_json::Value>>,
    error_stack::Report<storage_errors::StorageError>,
>
where
    T: Debug + Serialize,
{
    let key = key_store.key.get_inner().peek();
    let identifier = Identifier::Merchant(key_store.merchant_id.clone());

    let encoded_data = ext_traits::Encode::encode_to_value(&data)
        .change_context(storage_errors::StorageError::SerializationFailed)
        .attach_printable("Unable to encode data")?;

    let secret_data = Secret::<_, masking::WithType>::new(encoded_data);

    let encrypted_data = type_encryption::crypto_operation(
        key_manager_state,
        type_name!(payment_method::PaymentMethod),
        type_encryption::CryptoOperation::Encrypt(secret_data),
        identifier.clone(),
        key,
    )
    .await
    .and_then(|val| val.try_into_operation())
    .change_context(storage_errors::StorageError::EncryptionError)
    .attach_printable("Unable to encrypt data")?;

    Ok(encrypted_data)
}

#[async_trait::async_trait]
impl PaymentMethodsController for PmController<'_> {
    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    #[allow(clippy::too_many_arguments)]
    async fn create_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        payment_method_id: &str,
        locker_id: Option<String>,
        merchant_id: &id_type::MerchantId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        payment_method_data: crypto::OptionalEncryptableValue,
        connector_mandate_details: Option<serde_json::Value>,
        status: Option<common_enums::PaymentMethodStatus>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        card_scheme: Option<String>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        let db = &*self.state.store;
        let customer = db
            .find_customer_by_customer_id_merchant_id(
                customer_id,
                merchant_id,
                self.merchant_context.get_merchant_key_store(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

        let client_secret = generate_id(
            consts::ID_LENGTH,
            format!("{payment_method_id}_secret").as_str(),
        );

        let current_time = common_utils::date_time::now();

        let response = db
            .insert_payment_method(
                self.merchant_context.get_merchant_key_store(),
                domain::PaymentMethod {
                    customer_id: customer_id.to_owned(),
                    merchant_id: merchant_id.to_owned(),
                    payment_method_id: payment_method_id.to_string(),
                    locker_id,
                    payment_method: req.payment_method,
                    payment_method_type: req.payment_method_type,
                    payment_method_issuer: req.payment_method_issuer.clone(),
                    scheme: req.card_network.clone().or(card_scheme),
                    metadata: pm_metadata.map(Secret::new),
                    payment_method_data,
                    connector_mandate_details,
                    customer_acceptance: customer_acceptance.map(Secret::new),
                    client_secret: Some(client_secret),
                    status: status.unwrap_or(common_enums::PaymentMethodStatus::Active),
                    network_transaction_id: network_transaction_id.to_owned(),
                    payment_method_issuer_code: None,
                    accepted_currency: None,
                    token: None,
                    cardholder_name: None,
                    issuer_name: None,
                    issuer_country: None,
                    payer_country: None,
                    is_stored: None,
                    swift_code: None,
                    direct_debit_token: None,
                    created_at: current_time,
                    last_modified: current_time,
                    last_used_at: current_time,
                    payment_method_billing_address,
                    updated_by: None,
                    version: common_types::consts::API_VERSION,
                    network_token_requestor_reference_id,
                    network_token_locker_id,
                    network_token_payment_method_data,
                    vault_source_details: vault_source_details
                        .unwrap_or(PaymentMethodVaultSourceDetails::InternalVault),
                },
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in db")?;

        if customer.default_payment_method_id.is_none() && req.payment_method.is_some() {
            let _ = self
                .set_default_payment_method(merchant_id, customer_id, payment_method_id.to_owned())
                .await
                .map_err(|error| {
                    logger::error!(?error, "Failed to set the payment method as default")
                });
        }
        Ok(response)
    }

    #[cfg(feature = "v1")]
    fn store_default_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>) {
        let pm_id = generate_id(consts::ID_LENGTH, "pm");
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: merchant_id.to_owned(),
            customer_id: Some(customer_id.to_owned()),
            payment_method_id: pm_id,
            payment_method: req.payment_method,
            payment_method_type: req.payment_method_type,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            card: None,
            metadata: req.metadata.clone(),
            created: Some(common_utils::date_time::now()),
            recurring_enabled: Some(false),           //[#219]
            installment_payment_enabled: Some(false), //[#219]
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            last_used_at: Some(common_utils::date_time::now()),
            client_secret: None,
        };

        (payment_method_response, None)
    }

    #[cfg(feature = "v2")]
    fn store_default_payment_method(
        &self,
        _req: &api::PaymentMethodCreate,
        _customer_id: &id_type::CustomerId,
        _merchant_id: &id_type::MerchantId,
    ) -> (api::PaymentMethodResponse, Option<DataDuplicationCheck>) {
        todo!()
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn get_or_insert_payment_method(
        &self,
        req: api::PaymentMethodCreate,
        resp: &mut api::PaymentMethodResponse,
        customer_id: &id_type::CustomerId,
        key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        let mut payment_method_id = resp.payment_method_id.clone();
        let mut locker_id = None;
        let db = &*self.state.store;
        let payment_method = {
            let existing_pm_by_pmid = db
                .find_payment_method(
                    key_store,
                    &payment_method_id,
                    self.merchant_context.get_merchant_account().storage_scheme,
                )
                .await;

            if let Err(err) = existing_pm_by_pmid {
                if err.current_context().is_db_not_found() {
                    locker_id = Some(payment_method_id.clone());
                    let existing_pm_by_locker_id = db
                        .find_payment_method_by_locker_id(
                            key_store,
                            &payment_method_id,
                            self.merchant_context.get_merchant_account().storage_scheme,
                        )
                        .await;

                    match &existing_pm_by_locker_id {
                        Ok(pm) => payment_method_id.clone_from(pm.get_id()),
                        Err(_) => payment_method_id = generate_id(consts::ID_LENGTH, "pm"),
                    };
                    existing_pm_by_locker_id
                } else {
                    Err(err)
                }
            } else {
                existing_pm_by_pmid
            }
        };
        payment_method_id.clone_into(&mut resp.payment_method_id);

        match payment_method {
            Ok(pm) => Ok(pm),
            Err(err) => {
                if err.current_context().is_db_not_found() {
                    self.insert_payment_method(
                        resp,
                        &req,
                        key_store,
                        self.merchant_context.get_merchant_account().get_id(),
                        customer_id,
                        resp.metadata.clone().map(|val| val.expose()),
                        None,
                        locker_id,
                        None,
                        req.network_transaction_id.clone(),
                        None,
                        None,
                        None,
                        None,
                        Default::default(),
                    )
                    .await
                } else {
                    Err(err)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Error while finding payment method")
                }
            }
        }
    }

    #[cfg(feature = "v2")]
    async fn get_or_insert_payment_method(
        &self,
        _req: api::PaymentMethodCreate,
        _resp: &mut api::PaymentMethodResponse,
        _customer_id: &id_type::CustomerId,
        _key_store: &merchant_key_store::MerchantKeyStore,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        todo!()
    }

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn save_network_token_and_update_payment_method(
        &self,
        req: &api::PaymentMethodMigrate,
        key_store: &merchant_key_store::MerchantKeyStore,
        network_token_data: &api_models::payment_methods::MigrateNetworkTokenData,
        network_token_requestor_ref_id: String,
        pm_id: String,
    ) -> errors::PmResult<bool> {
        let payment_method_create_request =
            api::PaymentMethodCreate::get_payment_method_create_from_payment_method_migrate(
                network_token_data.network_token_number.clone(),
                req,
            );
        let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

        let network_token_details = api::CardDetail {
            card_number: network_token_data.network_token_number.clone(),
            card_exp_month: network_token_data.network_token_exp_month.clone(),
            card_exp_year: network_token_data.network_token_exp_year.clone(),
            card_holder_name: network_token_data.card_holder_name.clone(),
            nick_name: network_token_data.nick_name.clone(),
            card_issuing_country: network_token_data.card_issuing_country.clone(),
            card_network: network_token_data.card_network.clone(),
            card_issuer: network_token_data.card_issuer.clone(),
            card_type: network_token_data.card_type.clone(),
        };

        logger::debug!(
            "Adding network token to locker for customer_id: {:?}",
            customer_id
        );

        let token_resp = Box::pin(self.add_card_to_locker(
            payment_method_create_request.clone(),
            &network_token_details,
            &customer_id,
            None,
        ))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Network Token failed");
        let key_manager_state = &self.state.into();

        match token_resp {
            Ok(resp) => {
                logger::debug!("Network token added to locker");
                let (token_pm_resp, _duplication_check) = resp;
                let pm_token_details = token_pm_resp.card.as_ref().map(|card| {
                    PaymentMethodsData::Card(CardDetailsPaymentMethod::from((card.clone(), None)))
                });
                let pm_network_token_data_encrypted = pm_token_details
                    .async_map(|pm_card| {
                        create_encrypted_data(key_manager_state, key_store, pm_card)
                    })
                    .await
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encrypt payment method data")?;

                let pm_update = payment_methods::PaymentMethodUpdate::NetworkTokenDataUpdate {
                    network_token_requestor_reference_id: Some(network_token_requestor_ref_id),
                    network_token_locker_id: Some(token_pm_resp.payment_method_id),
                    network_token_payment_method_data: pm_network_token_data_encrypted
                        .map(Into::into),
                };
                let db = &*self.state.store;
                let existing_pm = db
                    .find_payment_method(
                        key_store,
                        &pm_id,
                        self.merchant_context.get_merchant_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(format!(
                        "Failed to fetch payment method for existing pm_id: {pm_id:?} in db",
                    ))?;

                db.update_payment_method(
                    key_store,
                    existing_pm,
                    pm_update,
                    self.merchant_context.get_merchant_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(format!(
                    "Failed to update payment method for existing pm_id: {pm_id:?} in db",
                ))?;

                logger::debug!("Network token added to locker and payment method updated");
                Ok(true)
            }
            Err(err) => {
                logger::debug!("Network token added to locker failed {:?}", err);
                Ok(false)
            }
        }
    }

    #[cfg(feature = "v1")]
    #[allow(clippy::too_many_arguments)]
    async fn insert_payment_method(
        &self,
        resp: &api::PaymentMethodResponse,
        req: &api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        pm_metadata: Option<serde_json::Value>,
        customer_acceptance: Option<serde_json::Value>,
        locker_id: Option<String>,
        connector_mandate_details: Option<serde_json::Value>,
        network_transaction_id: Option<String>,
        payment_method_billing_address: crypto::OptionalEncryptableValue,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: crypto::OptionalEncryptableValue,
        vault_source_details: Option<PaymentMethodVaultSourceDetails>,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        let pm_card_details = resp.card.clone().map(|card| {
            PaymentMethodsData::Card(CardDetailsPaymentMethod::from((card.clone(), None)))
        });
        let key_manager_state = self.state.into();
        let pm_data_encrypted: crypto::OptionalEncryptableValue = pm_card_details
            .clone()
            .async_map(|pm_card| create_encrypted_data(&key_manager_state, key_store, pm_card))
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt payment method data")?;

        self.create_payment_method(
            req,
            customer_id,
            &resp.payment_method_id,
            locker_id,
            merchant_id,
            pm_metadata,
            customer_acceptance,
            pm_data_encrypted,
            connector_mandate_details,
            None,
            network_transaction_id,
            payment_method_billing_address,
            resp.card.clone().and_then(|card| {
                card.card_network
                    .map(|card_network| card_network.to_string())
            }),
            network_token_requestor_reference_id,
            network_token_locker_id,
            network_token_payment_method_data,
            vault_source_details,
        )
        .await
    }

    #[cfg(feature = "v2")]
    #[allow(clippy::too_many_arguments)]
    async fn insert_payment_method(
        &self,
        _resp: &api::PaymentMethodResponse,
        _req: &api::PaymentMethodCreate,
        _key_store: &merchant_key_store::MerchantKeyStore,
        _merchant_id: &id_type::MerchantId,
        _customer_id: &id_type::CustomerId,
        _pm_metadata: Option<serde_json::Value>,
        _customer_acceptance: Option<serde_json::Value>,
        _locker_id: Option<String>,
        _connector_mandate_details: Option<serde_json::Value>,
        _network_transaction_id: Option<String>,
        _payment_method_billing_address: Option<encryption::Encryption>,
    ) -> errors::PmResult<payment_methods::PaymentMethod> {
        todo!()
    }

    #[cfg(feature = "payouts")]
    async fn add_bank_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        key_store: &merchant_key_store::MerchantKeyStore,
        bank: &payouts::Bank,
        customer_id: &id_type::CustomerId,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)> {
        let key = key_store.key.get_inner().peek();
        let payout_method_data = PayoutMethodData::Bank(bank.clone());
        let key_manager_state: KeyManagerState = self.state.into();
        let enc_data = async {
            serde_json::to_value(payout_method_data.to_owned())
                .map_err(|err| {
                    logger::error!("Error while encoding payout method data: {err:?}");
                    errors::VaultError::SavePaymentMethodFailed
                })
                .change_context(errors::VaultError::SavePaymentMethodFailed)
                .attach_printable("Unable to encode payout method data")
                .ok()
                .map(|v| {
                    let secret: Secret<String> = Secret::new(v.to_string());
                    secret
                })
                .async_lift(|inner| async {
                    type_encryption::crypto_operation(
                        &key_manager_state,
                        type_name!(payment_method::PaymentMethod),
                        type_encryption::CryptoOperation::EncryptOptional(inner),
                        Identifier::Merchant(key_store.merchant_id.clone()),
                        key,
                    )
                    .await
                    .and_then(|val| val.try_into_optionaloperation())
                })
                .await
        }
        .await
        .change_context(errors::VaultError::SavePaymentMethodFailed)
        .attach_printable("Failed to encrypt payout method data")?
        .map(Encryption::from)
        .map(|e| e.into_inner())
        .map_or(Err(errors::VaultError::SavePaymentMethodFailed), |e| {
            Ok(hex::encode(e.peek()))
        })?;

        let payload =
            transformers::StoreLockerReq::LockerGeneric(transformers::StoreGenericReq {
                merchant_id: self
                    .merchant_context
                    .get_merchant_account()
                    .get_id()
                    .to_owned(),
                merchant_customer_id: customer_id.to_owned(),
                enc_data,
                ttl: self.state.conf.locker.ttl_for_storage_in_secs,
            });
        let store_resp = cards::add_card_to_hs_locker(
            self.state,
            &payload,
            customer_id,
            api_enums::LockerChoice::HyperswitchCardVault,
        )
        .await?;
        let payment_method_resp = transformers::mk_add_bank_response_hs(
            bank.clone(),
            store_resp.card_reference,
            req,
            self.merchant_context.get_merchant_account().get_id(),
        );
        Ok((payment_method_resp, store_resp.duplication_check))
    }

    async fn add_card_to_locker(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)> {
         metrics::STORED_TO_LOCKER.add(1, &[]);
        let add_card_to_hs_resp = Box::pin(common_utils::metrics::utils::record_operation_time(
            async {
                self.add_card_hs(
                    req.clone(),
                    card,
                    customer_id,
                    api_enums::LockerChoice::HyperswitchCardVault,
                    card_reference,
                )
                .await
                .inspect_err(|_| {
                    metrics::CARD_LOCKER_FAILURES.add(
                        1,
                        router_env::metric_attributes!(("locker", "rust"), ("operation", "add")),
                    );
                })
            },
            &metrics::CARD_ADD_TIME,
            router_env::metric_attributes!(("locker", "rust")),
        ))
        .await?;

        logger::debug!("card added to hyperswitch-card-vault");
        Ok(add_card_to_hs_resp)
    }

    async fn delete_card_from_locker(
        &self,
        customer_id: &id_type::CustomerId,
        merchant_id: &id_type::MerchantId,
        card_reference: &str,
    ) -> errors::PmResult<DeleteCardResp> {
        metrics::DELETE_FROM_LOCKER.add(1, &[]);

        common_utils::metrics::utils::record_operation_time(
            async move {
                cards::delete_card_from_hs_locker(
                    self.state,
                    customer_id,
                    merchant_id,
                    card_reference,
                )
                .await
                .inspect_err(|_| {
                    metrics::CARD_LOCKER_FAILURES.add(
                        1,
                        router_env::metric_attributes!(("locker", "rust"), ("operation", "delete")),
                    );
                })
            },
            &metrics::CARD_DELETE_TIME,
            &[],
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while deleting card from locker")
    }

    async fn add_card_hs(
        &self,
        req: api::PaymentMethodCreate,
        card: &api::CardDetail,
        customer_id: &id_type::CustomerId,
        locker_choice: api_enums::LockerChoice,
        card_reference: Option<&str>,
    ) -> errors::VaultResult<(api::PaymentMethodResponse, Option<DataDuplicationCheck>)> {
        let payload = transformers::StoreLockerReq::LockerCard(transformers::StoreCardReq {
            merchant_id: self
                .merchant_context
                .get_merchant_account()
                .get_id()
                .to_owned(),
            merchant_customer_id: customer_id.to_owned(),
            requestor_card_reference: card_reference.map(str::to_string),
            card: Card {
                card_number: card.card_number.to_owned(),
                name_on_card: card.card_holder_name.to_owned(),
                card_exp_month: card.card_exp_month.to_owned(),
                card_exp_year: card.card_exp_year.to_owned(),
                card_brand: card.card_network.as_ref().map(ToString::to_string),
                card_isin: None,
                nick_name: card.nick_name.as_ref().map(Secret::peek).cloned(),
            },
            ttl: self.state.conf.locker.ttl_for_storage_in_secs,
        });

        let store_card_payload =
            cards::add_card_to_hs_locker(self.state, &payload, customer_id, locker_choice).await?;

        let payment_method_resp = transformers::mk_add_card_response_hs(
            card.clone(),
            store_card_payload.card_reference,
            req,
            self.merchant_context.get_merchant_account().get_id(),
        );
        Ok((payment_method_resp, store_card_payload.duplication_check))
    }

    #[cfg(feature = "v1")]
    async fn get_card_details_with_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<Option<api::CardDetailFromLocker>> {
        let card_decrypted = pm
            .payment_method_data
            .clone()
            .map(|x| x.into_inner().expose())
            .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
            .and_then(|pmd| match pmd {
                PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                _ => None,
            });

        Ok(if let Some(mut crd) = card_decrypted {
            crd.scheme.clone_from(&pm.scheme);
            Some(crd)
        } else {
            logger::debug!(
                "Getting card details from locker as it is not found in payment methods table"
            );
            Some(self.get_card_details_from_locker(pm).await?)
        })
    }

    #[cfg(feature = "v1")]
    async fn get_card_details_without_locker_fallback(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker> {
        let card_decrypted = pm
            .payment_method_data
            .clone()
            .map(|x| x.into_inner().expose())
            .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
            .and_then(|pmd| match pmd {
                PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                _ => None,
            });

        Ok(if let Some(mut crd) = card_decrypted {
            crd.scheme.clone_from(&pm.scheme);
            crd
        } else {
            logger::debug!(
                "Getting card details from locker as it is not found in payment methods table"
            );
            self.get_card_details_from_locker(pm).await?
        })
    }

        #[cfg(feature = "v1")]
    async fn set_default_payment_method(
        &self,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::CustomerId,
        payment_method_id: String,
    ) -> errors::PmResponse<api_models::payment_methods::CustomerDefaultPaymentMethodResponse> {
        let db = &*self.state.store;
        // check for the customer
        // TODO: customer need not be checked again here, this function can take an optional customer and check for existence of customer based on the optional value
        let customer = db
            .find_customer_by_customer_id_merchant_id(
                customer_id,
                merchant_id,
                self.merchant_context.get_merchant_key_store(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
        // check for the presence of payment_method
        let payment_method = db
            .find_payment_method(
                self.merchant_context.get_merchant_key_store(),
                &payment_method_id,
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
        let pm = payment_method
            .get_payment_method_type()
            .get_required_value("payment_method")?;

        common_utils::fp_utils::when(
            &payment_method.customer_id != customer_id
                || payment_method.merchant_id != *merchant_id,
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "The payment_method_id is not valid".to_string(),
                })
            },
        )?;

        common_utils::fp_utils::when(
            Some(payment_method_id.clone()) == customer.default_payment_method_id,
            || {
                Err(errors::ApiErrorResponse::PreconditionFailed {
                    message: "Payment Method is already set as default".to_string(),
                })
            },
        )?;

        let customer_id = customer.customer_id.clone();

        let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
            default_payment_method_id: Some(Some(payment_method_id.to_owned())),
        };
        // update the db with the default payment method id

        let updated_customer_details = db
            .update_customer_by_customer_id_merchant_id(
                customer_id.to_owned(),
                merchant_id.to_owned(),
                customer,
                customer_update,
                self.merchant_context.get_merchant_key_store(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update the default payment method id for the customer")?;

        let resp = api_models::payment_methods::CustomerDefaultPaymentMethodResponse {
            default_payment_method_id: updated_customer_details.default_payment_method_id,
            customer_id,
            payment_method_type: payment_method.get_payment_method_subtype(),
            payment_method: pm,
        };

        Ok(domain_api::ApplicationResponse::Json(resp))
    }

    #[cfg(feature = "v1")]
    async fn add_payment_method_status_update_task(
        &self,
        payment_method: &payment_methods::PaymentMethod,
        prev_status: common_enums::PaymentMethodStatus,
        curr_status: common_enums::PaymentMethodStatus,
        merchant_id: &id_type::MerchantId,
    ) -> Result<(), sch_errors::ProcessTrackerError> {
        cards::add_payment_method_status_update_task(
            &*self.state.store,
            payment_method,
            prev_status,
            curr_status,
            merchant_id,
        ).await
    }

    #[cfg(feature = "v1")]
    async fn validate_merchant_connector_ids_in_connector_mandate_details(
        &self,
        key_store: &merchant_key_store::MerchantKeyStore,
        connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
        merchant_id: &id_type::MerchantId,
        card_network: Option<common_enums::CardNetwork>,
    ) -> errors::PmResult<()> {
        helpers::validate_merchant_connector_ids_in_connector_mandate_details(
            self.state,
            key_store,
            connector_mandate_details,
            merchant_id,
            card_network,
        )
        .await
    }

    #[cfg(feature = "v1")]
    async fn get_card_details_from_locker(
        &self,
        pm: &payment_methods::PaymentMethod,
    ) -> errors::PmResult<api::CardDetailFromLocker> {
        let card = cards::get_card_from_locker(
        self.state,
        &pm.customer_id,
        &pm.merchant_id,
        pm.locker_id.as_ref().unwrap_or(pm.get_id()),
    )
    .await
    .attach_printable("Error getting card from card vault")?;

    cards::get_card_detail(pm, card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")
    }

    #[cfg(feature = "v1")]
    async fn retrieve_payment_method(
        &self,
        pm: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodResponse> {
        let db = self.state.store.as_ref();
        let pm = db
            .find_payment_method(
                self.merchant_context.get_merchant_key_store(),
                &pm.payment_method_id,
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        let card = if pm.get_payment_method_type() == Some(common_enums::PaymentMethod::Card) {
            let card_detail = if self.state.conf.locker.locker_enabled {
                let card = cards::get_card_from_locker(
                    self.state,
                    &pm.customer_id,
                    &pm.merchant_id,
                    pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting card from card vault")?;
                cards::get_card_detail(&pm, card)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while getting card details from locker")?
            } else {
                self.get_card_details_without_locker_fallback(&pm).await?
            };
            Some(card_detail)
        } else {
            None
        };
        Ok(domain_api::ApplicationResponse::Json(
            api::PaymentMethodResponse {
                merchant_id: pm.merchant_id.clone(),
                customer_id: Some(pm.customer_id.clone()),
                payment_method_id: pm.payment_method_id.clone(),
                payment_method: pm.get_payment_method_type(),
                payment_method_type: pm.get_payment_method_subtype(),
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card,
                metadata: pm.metadata,
                created: Some(pm.created_at),
                recurring_enabled: Some(false),
                installment_payment_enabled: Some(false),
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
                last_used_at: Some(pm.last_used_at),
                client_secret: pm.client_secret,
            },
        ))
    }

    #[cfg(feature = "v1")]
    #[instrument(skip_all)]
    async fn delete_payment_method(
        &self,
        pm_id: api::PaymentMethodId,
    ) -> errors::PmResponse<api::PaymentMethodDeleteResponse> {
        let db = self.state.store.as_ref();
        let key = db
            .find_payment_method(
                self.merchant_context.get_merchant_key_store(),
                pm_id.payment_method_id.as_str(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        let customer = db
            .find_customer_by_customer_id_merchant_id(
                &key.customer_id,
                self.merchant_context.get_merchant_account().get_id(),
                self.merchant_context.get_merchant_key_store(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Customer not found for the payment method")?;

        if key.get_payment_method_type() == Some(api_enums::PaymentMethod::Card) {
            let response = self
                .delete_card_from_locker(
                    &key.customer_id,
                    &key.merchant_id,
                    key.locker_id.as_ref().unwrap_or(&key.payment_method_id),
                )
                .await?;

            if let Some(network_token_ref_id) = key.network_token_requestor_reference_id {
                let resp =
                    network_tokenization::delete_network_token_from_locker_and_token_service(
                        self.state,
                        &key.customer_id,
                        &key.merchant_id,
                        key.payment_method_id.clone(),
                        key.network_token_locker_id,
                        network_token_ref_id,
                        self.merchant_context,
                    )
                    .await?;

                if resp.status == "Ok" {
                    logger::info!("Token From locker deleted Successfully!");
                } else {
                    logger::error!("Error: Deleting Token From Locker!\n{:#?}", resp);
                }
            }

            if response.status == "Ok" {
                logger::info!("Card From locker deleted Successfully!");
            } else {
                logger::error!("Error: Deleting Card From Locker!\n{:#?}", response);
                Err(errors::ApiErrorResponse::InternalServerError)?
            }
        }

        db.delete_payment_method_by_merchant_id_payment_method_id(
            self.merchant_context.get_merchant_key_store(),
            self.merchant_context.get_merchant_account().get_id(),
            pm_id.payment_method_id.as_str(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        if customer.default_payment_method_id.as_ref() == Some(&pm_id.payment_method_id) {
            let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
                default_payment_method_id: Some(None),
            };
            db.update_customer_by_customer_id_merchant_id(
                key.customer_id,
                key.merchant_id,
                customer,
                customer_update,
                self.merchant_context.get_merchant_key_store(),
                self.merchant_context.get_merchant_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update the default payment method id for the customer")?;
        };

        Ok(domain_api::ApplicationResponse::Json(
            api::PaymentMethodDeleteResponse {
                payment_method_id: key.payment_method_id.clone(),
                deleted: true,
            },
        ))
    }

    #[cfg(feature = "v1")]
    async fn add_payment_method(
        &self,
        req: &api::PaymentMethodCreate,
    ) -> errors::PmResponse<api::PaymentMethodResponse> {
        req.validate()?;
        let db = &*self.state.store;
        let merchant_id = self.merchant_context.get_merchant_account().get_id();
        let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
        let payment_method = req.payment_method.get_required_value("payment_method")?;
        let key_manager_state = self.state.into();
        let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
            .billing
            .clone()
            .async_map(|billing| {
                create_encrypted_data(
                    &key_manager_state,
                    self.merchant_context.get_merchant_key_store(),
                    billing,
                )
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt Payment method billing address")?;

        let connector_mandate_details = req
            .connector_mandate_details
            .clone()
            .map(serde_json::to_value)
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        let response = match payment_method {
            #[cfg(feature = "payouts")]
            api_enums::PaymentMethod::BankTransfer => match req.bank_transfer.clone() {
                Some(bank) => self
                    .add_bank_to_locker(
                        req.clone(),
                        self.merchant_context.get_merchant_key_store(),
                        &bank,
                        &customer_id,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Add PaymentMethod Failed"),
                _ => Ok(self.store_default_payment_method(req, &customer_id, merchant_id)),
            },
            api_enums::PaymentMethod::Card => match req.card.clone() {
                Some(card) => {
                    let mut card_details = card;
                    card_details = helpers::populate_bin_details_for_payment_method_create(
                        card_details.clone(),
                        db,
                    )
                    .await;
                    validate_card_expiry(
                        &card_details.card_exp_month,
                        &card_details.card_exp_year,
                    )?;
                    Box::pin(self.add_card_to_locker(
                        req.clone(),
                        &card_details,
                        &customer_id,
                        None,
                    ))
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Add Card Failed")
                }
                _ => Ok(self.store_default_payment_method(req, &customer_id, merchant_id)),
            },
            _ => Ok(self.store_default_payment_method(req, &customer_id, merchant_id)),
        };

        let (mut resp, duplication_check) = response?;

        match duplication_check {
            Some(duplication_check) => match duplication_check {
                DataDuplicationCheck::Duplicated => {
                    let existing_pm = self
                        .get_or_insert_payment_method(
                            req.clone(),
                            &mut resp,
                            &customer_id,
                            self.merchant_context.get_merchant_key_store(),
                        )
                        .await?;

                    resp.client_secret = existing_pm.client_secret;
                }
                DataDuplicationCheck::MetaDataChanged => {
                    if let Some(card) = req.card.clone() {
                        let existing_pm = self
                            .get_or_insert_payment_method(
                                req.clone(),
                                &mut resp,
                                &customer_id,
                                self.merchant_context.get_merchant_key_store(),
                            )
                            .await?;

                        let client_secret = existing_pm.client_secret.clone();

                        self.delete_card_from_locker(
                            &customer_id,
                            merchant_id,
                            existing_pm
                                .locker_id
                                .as_ref()
                                .unwrap_or(&existing_pm.payment_method_id),
                        )
                        .await?;

                        let add_card_resp = self
                            .add_card_hs(
                                req.clone(),
                                &card,
                                &customer_id,
                                api_enums::LockerChoice::HyperswitchCardVault,
                                Some(
                                    existing_pm
                                        .locker_id
                                        .as_ref()
                                        .unwrap_or(&existing_pm.payment_method_id),
                                ),
                            )
                            .await;

                        if let Err(err) = add_card_resp {
                            logger::error!(vault_err=?err);
                            db.delete_payment_method_by_merchant_id_payment_method_id(
                                self.merchant_context.get_merchant_key_store(),
                                merchant_id,
                                &resp.payment_method_id,
                            )
                            .await
                            .to_not_found_response(
                                errors::ApiErrorResponse::PaymentMethodNotFound,
                            )?;

                            Err(report!(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Failed while updating card metadata changes"))?
                        };

                        let existing_pm_data = self
                            .get_card_details_without_locker_fallback(&existing_pm)
                            .await?;

                        let updated_card = Some(api::CardDetailFromLocker {
                            scheme: existing_pm.scheme.clone(),
                            last4_digits: Some(card.card_number.get_last4()),
                            issuer_country: card
                                .card_issuing_country
                                .or(existing_pm_data.issuer_country),
                            card_isin: Some(card.card_number.get_card_isin()),
                            card_number: Some(card.card_number),
                            expiry_month: Some(card.card_exp_month),
                            expiry_year: Some(card.card_exp_year),
                            card_token: None,
                            card_fingerprint: None,
                            card_holder_name: card
                                .card_holder_name
                                .or(existing_pm_data.card_holder_name),
                            nick_name: card.nick_name.or(existing_pm_data.nick_name),
                            card_network: card.card_network.or(existing_pm_data.card_network),
                            card_issuer: card.card_issuer.or(existing_pm_data.card_issuer),
                            card_type: card.card_type.or(existing_pm_data.card_type),
                            saved_to_locker: true,
                        });

                        let updated_pmd = updated_card.as_ref().map(|card| {
                            PaymentMethodsData::Card(CardDetailsPaymentMethod::from((
                                card.clone(),
                                None,
                            )))
                        });
                        let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> =
                            updated_pmd
                                .async_map(|updated_pmd| {
                                    create_encrypted_data(
                                        &key_manager_state,
                                        self.merchant_context.get_merchant_key_store(),
                                        updated_pmd,
                                    )
                                })
                                .await
                                .transpose()
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Unable to encrypt payment method data")?;

                        let pm_update = payment_methods::PaymentMethodUpdate::PaymentMethodDataUpdate {
                            payment_method_data: pm_data_encrypted.map(Into::into),
                        };

                        db.update_payment_method(
                            self.merchant_context.get_merchant_key_store(),
                            existing_pm,
                            pm_update,
                            self.merchant_context.get_merchant_account().storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to add payment method in db")?;

                        resp.client_secret = client_secret;
                    }
                }
            },
            None => {
                let pm_metadata = resp.metadata.as_ref().map(|data| data.peek());

                let locker_id = if resp.payment_method == Some(api_enums::PaymentMethod::Card)
                    || resp.payment_method == Some(api_enums::PaymentMethod::BankTransfer)
                {
                    Some(resp.payment_method_id)
                } else {
                    None
                };
                resp.payment_method_id = generate_id(consts::ID_LENGTH, "pm");
                let pm = self
                    .insert_payment_method(
                        &resp,
                        req,
                        self.merchant_context.get_merchant_key_store(),
                        merchant_id,
                        &customer_id,
                        pm_metadata.cloned(),
                        None,
                        locker_id,
                        connector_mandate_details,
                        req.network_transaction_id.clone(),
                        payment_method_billing_address,
                        None,
                        None,
                        None,
                        Default::default(), //Currently this method is used for adding payment method via PaymentMethodCreate API which doesn't support external vault. hence Default i.e. InternalVault is passed for vault source and type
                    )
                    .await?;

                resp.client_secret = pm.client_secret;
            }
        }

        Ok(domain_api::ApplicationResponse::Json(resp))
    }
}
