use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::{enums as api_enums, payment_methods as payment_methods_api};
use cards::CardNumber;
use common_utils::{fp_utils::when, id_type};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::router_request_types as domain_request_types;
use masking::{ExposeInterface, Secret};
use router_env::logger;

use super::{
    CardNetworkTokenizeExecutor, NetworkTokenizationBuilder, NetworkTokenizationProcess,
    NetworkTokenizationResponse, State, StoreLockerResponse, TransitionTo,
};
use crate::{
    core::payment_methods::{
        cards::{add_card_to_hs_locker, create_encrypted_data},
        network_tokenization, transformers as pm_transformers,
    },
    errors::{self, RouterResult},
    types::{api, domain},
    SessionState,
};

// Available states for payment method tokenization
pub struct TokenizeWithPmId;
pub struct PmValidated;
pub struct PmFetched;
pub struct PmAssigned;
pub struct PmTokenized;
pub struct PmTokenStored;
pub struct PmTokenUpdated;

impl State for TokenizeWithPmId {}
impl State for PmValidated {}
impl State for PmFetched {}
impl State for PmAssigned {}
impl State for PmTokenized {}
impl State for PmTokenStored {}
impl State for PmTokenUpdated {}

// State transitions for payment method tokenization
impl TransitionTo<PmFetched> for TokenizeWithPmId {}
impl TransitionTo<PmValidated> for PmFetched {}
impl TransitionTo<PmAssigned> for PmValidated {}
impl TransitionTo<PmTokenized> for PmAssigned {}
impl TransitionTo<PmTokenStored> for PmTokenized {}
impl TransitionTo<PmTokenUpdated> for PmTokenStored {}

impl<'a> NetworkTokenizationBuilder<'a, TokenizeWithPmId> {
    pub fn new() -> Self {
        Self {
            state: std::marker::PhantomData,
            customer: None,
            card: None,
            network_token: None,
            stored_card: None,
            stored_token: None,
            payment_method_response: None,
            card_tokenized: false,
            error_code: None,
            error_message: None,
        }
    }
    pub fn set_payment_method(
        self,
        payment_method: &domain::PaymentMethod,
    ) -> NetworkTokenizationBuilder<'a, PmFetched> {
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: payment_method.merchant_id.clone(),
            customer_id: Some(payment_method.customer_id.clone()),
            payment_method_id: payment_method.payment_method_id.clone(),
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            recurring_enabled: true,
            installment_payment_enabled: false,
            metadata: payment_method.metadata.clone(),
            created: Some(payment_method.created_at),
            last_used_at: Some(payment_method.last_used_at),
            client_secret: payment_method.client_secret.clone(),
            card: None,
            bank_transfer: None,
            payment_experience: None,
        };
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            payment_method_response: Some(payment_method_response),
            customer: self.customer,
            card: self.card,
            network_token: self.network_token,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, PmFetched> {
    pub fn set_validate_result(self) -> NetworkTokenizationBuilder<'a, PmValidated> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            customer: self.customer,
            card: self.card,
            network_token: self.network_token,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, PmValidated> {
    pub fn set_card_details(
        self,
        card_from_locker: &'a api_models::payment_methods::Card,
        optional_card_info: Option<diesel_models::CardInfo>,
        card_cvc: Secret<String>,
    ) -> NetworkTokenizationBuilder<'a, PmAssigned> {
        let card = domain::Card {
            card_number: card_from_locker.card_number.clone(),
            card_exp_month: card_from_locker.card_exp_month.clone(),
            card_exp_year: card_from_locker.card_exp_year.clone(),
            card_cvc,
            bank_code: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.bank_code.clone()),
            nick_name: card_from_locker
                .nick_name
                .as_ref()
                .map(|nick_name| Secret::new(nick_name.clone())),
            card_holder_name: card_from_locker.name_on_card.clone(),
            card_issuer: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.card_issuer.clone()),
            card_network: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.card_network.clone()),
            card_type: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.card_type.clone()),
            card_issuing_country: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.card_issuing_country.clone()),
        };
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            card: Some(card),
            customer: self.customer,
            network_token: self.network_token,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, PmAssigned> {
    pub fn get_optional_card(&self) -> Option<domain::Card> {
        self.card.clone()
    }
    pub fn set_token_details(
        self,
        network_token: &'a NetworkTokenizationResponse,
    ) -> NetworkTokenizationBuilder<'a, PmTokenized> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            network_token: Some(&network_token.0),
            card_tokenized: true,
            customer: self.customer,
            card: self.card,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            payment_method_response: self.payment_method_response,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, PmTokenized> {
    pub fn set_stored_token_response(
        self,
        store_token_response: &'a pm_transformers::StoreCardRespPayload,
    ) -> NetworkTokenizationBuilder<'a, PmTokenStored> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            stored_token: Some(store_token_response),
            customer: self.customer,
            card: self.card,
            network_token: self.network_token,
            stored_card: self.stored_card,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, PmTokenStored> {
    pub fn set_payment_method(
        self,
        payment_method: &'a domain::PaymentMethod,
    ) -> NetworkTokenizationBuilder<'a, PmTokenUpdated> {
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: payment_method.merchant_id.clone(),
            customer_id: Some(payment_method.customer_id.clone()),
            payment_method_id: payment_method.payment_method_id.clone(),
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            recurring_enabled: true,
            installment_payment_enabled: false,
            metadata: payment_method.metadata.clone(),
            created: Some(payment_method.created_at),
            last_used_at: Some(payment_method.last_used_at),
            client_secret: payment_method.client_secret.clone(),
            card: None,
            bank_transfer: None,
            payment_experience: None,
        };
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            payment_method_response: Some(payment_method_response),
            customer: self.customer,
            card: self.card,
            stored_token: self.stored_token,
            network_token: self.network_token,
            stored_card: self.stored_card,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl NetworkTokenizationBuilder<'_, PmTokenUpdated> {
    pub fn build(self) -> api::CardNetworkTokenizeResponse {
        api::CardNetworkTokenizeResponse {
            payment_method_response: self.payment_method_response,
            customer: self.customer.cloned(),
            card_tokenized: self.card_tokenized,
            error_code: self.error_code.cloned(),
            error_message: self.error_message.cloned(),
            // Below field is mutated by caller functions for batched API operations
            req: None,
        }
    }
}

// Specific executor for payment method tokenization
impl<'a> CardNetworkTokenizeExecutor<'a, domain::TokenizePaymentMethodRequest> {
    pub async fn fetch_payment_method(
        &self,
        payment_method_id: &str,
    ) -> RouterResult<domain::PaymentMethod> {
        self.state
            .store
            .find_payment_method(
                &self.state.into(),
                self.key_store,
                payment_method_id,
                self.merchant_account.storage_scheme,
            )
            .await
            .map_err(|err| match err.current_context() {
                storage_impl::errors::StorageError::DatabaseError(err)
                    if matches!(
                        err.current_context(),
                        diesel_models::errors::DatabaseError::NotFound
                    ) =>
                {
                    report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payment_method_id".into(),
                    })
                }
                storage_impl::errors::StorageError::ValueNotFound(_) => {
                    report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payment_method_id".to_string(),
                    })
                }
                err => {
                    logger::info!("Error fetching payment_method: {:?}", err);
                    report!(errors::ApiErrorResponse::InternalServerError)
                }
            })
    }
    pub async fn validate_payment_method_and_get_locker_reference(
        &self,
        payment_method: &domain::PaymentMethod,
    ) -> RouterResult<String> {
        // Ensure payment method is card
        match payment_method.payment_method {
            Some(api_enums::PaymentMethod::Card) => Ok(()),
            Some(_) => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment method is not card".to_string()
            })),
            None => Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment method is empty".to_string()
            })),
        }?;

        // Ensure card is not tokenized already
        when(
            payment_method
                .network_token_requestor_reference_id
                .is_some(),
            || {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Card is already tokenized".to_string()
                }))
            },
        )?;

        // Ensure locker reference is present
        payment_method.locker_id.clone().map_or(
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "locker_id not found for given payment_method_id".to_string()
            })),
            Ok,
        )
    }
    pub async fn update_payment_method(
        &self,
        store_token_response: &pm_transformers::StoreCardRespPayload,
        payment_method: domain::PaymentMethod,
        network_token_details: &NetworkTokenizationResponse,
        card_details: &domain::Card,
    ) -> RouterResult<domain::PaymentMethod> {
        // Form encrypted network token data (tokenized card)
        let network_token_data = &network_token_details.0;
        let token_data = api::PaymentMethodsData::Card(api::CardDetailsPaymentMethod {
            last4_digits: Some(network_token_data.token_last_four.clone()),
            expiry_month: Some(network_token_data.token_expiry_month.clone()),
            expiry_year: Some(network_token_data.token_expiry_year.clone()),
            card_isin: Some(network_token_data.token_isin.clone()),
            nick_name: card_details.nick_name.clone(),
            card_holder_name: card_details.card_holder_name.clone(),
            issuer_country: card_details.card_issuing_country.clone(),
            card_issuer: card_details.card_issuer.clone(),
            card_network: card_details.card_network.clone(),
            card_type: card_details.card_type.clone(),
            saved_to_locker: true,
        });
        let enc_token_data = create_encrypted_data(&self.state.into(), self.key_store, token_data)
            .await
            .inspect_err(|err| logger::info!("Error encrypting network token data: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        // Update payment method
        let payment_method_update = diesel_models::PaymentMethodUpdate::NetworkTokenDataUpdate {
            network_token_requestor_reference_id: network_token_details.1.clone(),
            network_token_locker_id: Some(store_token_response.card_reference.clone()),
            network_token_payment_method_data: Some(enc_token_data.into()),
        };
        self.state
            .store
            .update_payment_method(
                &self.state.into(),
                self.key_store,
                payment_method,
                payment_method_update,
                self.merchant_account.storage_scheme,
            )
            .await
            .inspect_err(|err| logger::info!("Error updating payment method: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)
    }
}

// Common executor for payment method tokenization
#[async_trait::async_trait]
impl<'a> NetworkTokenizationProcess<'a, domain::TokenizePaymentMethodRequest>
    for CardNetworkTokenizeExecutor<'a, domain::TokenizePaymentMethodRequest>
{
    fn new(
        state: &'a SessionState,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        data: &'a domain::TokenizePaymentMethodRequest,
        customer: Option<&'a domain_request_types::CustomerDetails>,
    ) -> Self {
        Self {
            data,
            customer,
            state,
            merchant_account,
            key_store,
        }
    }

    async fn fetch_bin_details(
        &self,
        card_number: CardNumber,
    ) -> RouterResult<Option<diesel_models::CardInfo>> {
        let db = &*self.state.store;
        db.get_card_info(&card_number.get_card_isin())
            .await
            .attach_printable("Failed to perform BIN lookup")
            .change_context(errors::ApiErrorResponse::InternalServerError)
    }

    async fn tokenize_card(
        &self,
        customer_id: &id_type::CustomerId,
        card: &domain::Card,
    ) -> RouterResult<NetworkTokenizationResponse> {
        match network_tokenization::make_card_network_tokenization_request(
            self.state,
            card,
            customer_id,
        )
        .await
        {
            Ok(tokenization_response) => Ok(tokenization_response),
            Err(err) => {
                // TODO: revert this
                logger::error!(
                    "Failed to tokenize card with the network: {:?}\nUsing dummy response",
                    err
                );
                Ok((
                    network_tokenization::CardNetworkTokenResponsePayload {
                        card_brand: api_enums::CardNetwork::Visa,
                        card_fingerprint: None,
                        card_reference: uuid::Uuid::new_v4().to_string(),
                        correlation_id: uuid::Uuid::new_v4().to_string(),
                        customer_id: customer_id.get_string_repr().to_string(),
                        par: "".to_string(),
                        token: card.card_number.clone(),
                        token_expiry_month: card.card_exp_month.clone(),
                        token_expiry_year: card.card_exp_year.clone(),
                        token_isin: card.card_number.get_card_isin(),
                        token_last_four: card.card_number.get_last4(),
                        token_status: "active".to_string(),
                    },
                    Some(uuid::Uuid::new_v4().to_string()),
                ))
            }
        }
    }

    async fn store_network_token_in_locker(
        &self,
        network_token: &NetworkTokenizationResponse,
        customer_id: &id_type::CustomerId,
        card_holder_name: Option<Secret<String>>,
        nick_name: Option<Secret<String>>,
    ) -> RouterResult<pm_transformers::StoreCardRespPayload> {
        let network_token = &network_token.0;
        let merchant_id = self.merchant_account.get_id();
        let locker_req =
            pm_transformers::StoreLockerReq::LockerCard(pm_transformers::StoreCardReq {
                merchant_id: merchant_id.clone(),
                merchant_customer_id: customer_id.clone(),
                card: payment_methods_api::Card {
                    card_number: network_token.token.clone(),
                    card_exp_month: network_token.token_expiry_month.clone(),
                    card_exp_year: network_token.token_expiry_year.clone(),
                    card_brand: Some(network_token.card_brand.to_string()),
                    card_isin: Some(network_token.token_isin.clone()),
                    name_on_card: card_holder_name,
                    nick_name: nick_name.map(|nick_name| nick_name.expose()),
                },
                requestor_card_reference: None,
                ttl: self.state.conf.locker.ttl_for_storage_in_secs,
            });

        let stored_resp = add_card_to_hs_locker(
            self.state,
            &locker_req,
            customer_id,
            api_enums::LockerChoice::HyperswitchCardVault,
        )
        .await
        .inspect_err(|err| logger::info!("Error adding card in locker: {:?}", err))
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        Ok(stored_resp)
    }
}
