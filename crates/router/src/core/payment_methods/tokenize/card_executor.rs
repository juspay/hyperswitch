use std::str::FromStr;

use api_models::{enums as api_enums, payment_methods as payment_methods_api};
use common_utils::{
    consts,
    ext_traits::OptionExt,
    generate_customer_id_of_default_length, id_type,
    pii::Email,
    type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use masking::{ExposeInterface, PeekInterface, SwitchStrategy};
use router_env::logger;

use super::{
    migration, CardNetworkTokenizeExecutor, NetworkTokenizationBuilder, NetworkTokenizationProcess,
    NetworkTokenizationResponse, State, StoreLockerResponse, TransitionTo,
};
use crate::{
    core::payment_methods::{
        cards::{add_card_to_hs_locker, create_payment_method},
        transformers as pm_transformers,
    },
    errors::{self, RouterResult},
    types::{api, domain},
    utils,
};

// Available states for card tokenization
pub struct TokenizeWithCard;
pub struct CardRequestValidated;
pub struct CardDetailsAssigned;
pub struct CustomerAssigned;
pub struct CardTokenized;
pub struct CardStored;
pub struct CardTokenStored;
pub struct PaymentMethodCreated;

impl State for TokenizeWithCard {}
impl State for CustomerAssigned {}
impl State for CardRequestValidated {}
impl State for CardDetailsAssigned {}
impl State for CardTokenized {}
impl State for CardStored {}
impl State for CardTokenStored {}
impl State for PaymentMethodCreated {}

// State transitions for card tokenization
impl TransitionTo<CardRequestValidated> for TokenizeWithCard {}
impl TransitionTo<CardDetailsAssigned> for CardRequestValidated {}
impl TransitionTo<CustomerAssigned> for CardDetailsAssigned {}
impl TransitionTo<CardTokenized> for CustomerAssigned {}
impl TransitionTo<CardTokenStored> for CardTokenized {}
impl TransitionTo<PaymentMethodCreated> for CardTokenStored {}

impl Default for NetworkTokenizationBuilder<'_, TokenizeWithCard> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> NetworkTokenizationBuilder<'a, TokenizeWithCard> {
    pub fn new() -> Self {
        Self {
            state: std::marker::PhantomData,
            customer: None,
            card: None,
            card_cvc: None,
            network_token: None,
            stored_card: None,
            stored_token: None,
            payment_method_response: None,
            card_tokenized: false,
            error_code: None,
            error_message: None,
        }
    }
    pub fn set_validate_result(self) -> NetworkTokenizationBuilder<'a, CardRequestValidated> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            customer: self.customer,
            card: self.card,
            card_cvc: self.card_cvc,
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

impl<'a> NetworkTokenizationBuilder<'a, CardRequestValidated> {
    pub fn set_card_details(
        self,
        card_req: &'a domain::TokenizeCardRequest,
        optional_card_info: Option<diesel_models::CardInfo>,
    ) -> NetworkTokenizationBuilder<'a, CardDetailsAssigned> {
        let card = domain::CardDetail {
            card_number: card_req.raw_card_number.clone(),
            card_exp_month: card_req.card_expiry_month.clone(),
            card_exp_year: card_req.card_expiry_year.clone(),
            bank_code: optional_card_info
                .as_ref()
                .and_then(|card_info| card_info.bank_code.clone()),
            nick_name: card_req.nick_name.clone(),
            card_holder_name: card_req.card_holder_name.clone(),
            card_issuer: optional_card_info
                .as_ref()
                .map_or(card_req.card_issuer.clone(), |card_info| {
                    card_info.card_issuer.clone()
                }),
            card_network: optional_card_info
                .as_ref()
                .map_or(card_req.card_network.clone(), |card_info| {
                    card_info.card_network.clone()
                }),
            card_type: optional_card_info.as_ref().map_or(
                card_req
                    .card_type
                    .as_ref()
                    .map(|card_type| card_type.to_string()),
                |card_info| card_info.card_type.clone(),
            ),
            card_issuing_country: optional_card_info
                .as_ref()
                .map_or(card_req.card_issuing_country.clone(), |card_info| {
                    card_info.card_issuing_country.clone()
                }),
        };
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            card: Some(card),
            card_cvc: card_req.card_cvc.clone(),
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

impl<'a> NetworkTokenizationBuilder<'a, CardDetailsAssigned> {
    pub fn set_customer(
        self,
        customer: &'a api::CustomerDetails,
    ) -> NetworkTokenizationBuilder<'a, CustomerAssigned> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            customer: Some(customer),
            card: self.card,
            card_cvc: self.card_cvc,
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

impl<'a> NetworkTokenizationBuilder<'a, CustomerAssigned> {
    pub fn get_optional_card_and_cvc(
        &self,
    ) -> (Option<domain::CardDetail>, Option<masking::Secret<String>>) {
        (self.card.clone(), self.card_cvc.clone())
    }
    pub fn set_token_details(
        self,
        network_token: &'a NetworkTokenizationResponse,
    ) -> NetworkTokenizationBuilder<'a, CardTokenized> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            network_token: Some(&network_token.0),
            customer: self.customer,
            card: self.card,
            card_cvc: self.card_cvc,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, CardTokenized> {
    pub fn set_stored_card_response(
        self,
        store_card_response: &'a StoreLockerResponse,
    ) -> NetworkTokenizationBuilder<'a, CardStored> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            stored_card: Some(&store_card_response.store_card_resp),
            customer: self.customer,
            card: self.card,
            card_cvc: self.card_cvc,
            network_token: self.network_token,
            stored_token: self.stored_token,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, CardStored> {
    pub fn set_stored_token_response(
        self,
        store_token_response: &'a StoreLockerResponse,
    ) -> NetworkTokenizationBuilder<'a, CardTokenStored> {
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            card_tokenized: true,
            stored_token: Some(&store_token_response.store_token_resp),
            customer: self.customer,
            card: self.card,
            card_cvc: self.card_cvc,
            network_token: self.network_token,
            stored_card: self.stored_card,
            payment_method_response: self.payment_method_response,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl<'a> NetworkTokenizationBuilder<'a, CardTokenStored> {
    pub fn set_payment_method_response(
        self,
        payment_method: &'a domain::PaymentMethod,
    ) -> NetworkTokenizationBuilder<'a, PaymentMethodCreated> {
        let card_detail_from_locker = self.card.as_ref().map(|card| api::CardDetailFromLocker {
            scheme: None,
            issuer_country: card.card_issuing_country.clone(),
            last4_digits: Some(card.card_number.clone().get_last4()),
            card_number: None,
            expiry_month: Some(card.card_exp_month.clone().clone()),
            expiry_year: Some(card.card_exp_year.clone().clone()),
            card_token: None,
            card_holder_name: card.card_holder_name.clone(),
            card_fingerprint: None,
            nick_name: card.nick_name.clone(),
            card_network: card.card_network.clone(),
            card_isin: Some(card.card_number.clone().get_card_isin()),
            card_issuer: card.card_issuer.clone(),
            card_type: card.card_type.clone(),
            saved_to_locker: true,
        });
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: payment_method.merchant_id.clone(),
            customer_id: Some(payment_method.customer_id.clone()),
            payment_method_id: payment_method.payment_method_id.clone(),
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            card: card_detail_from_locker,
            recurring_enabled: true,
            installment_payment_enabled: false,
            metadata: payment_method.metadata.clone(),
            created: Some(payment_method.created_at),
            last_used_at: Some(payment_method.last_used_at),
            client_secret: payment_method.client_secret.clone(),
            bank_transfer: None,
            payment_experience: None,
        };
        NetworkTokenizationBuilder {
            state: std::marker::PhantomData,
            payment_method_response: Some(payment_method_response),
            customer: self.customer,
            card: self.card,
            card_cvc: self.card_cvc,
            network_token: self.network_token,
            stored_card: self.stored_card,
            stored_token: self.stored_token,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

impl NetworkTokenizationBuilder<'_, PaymentMethodCreated> {
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

// Specific executor for card tokenization
impl CardNetworkTokenizeExecutor<'_, domain::TokenizeCardRequest> {
    pub async fn validate_request_and_fetch_optional_customer(
        &self,
    ) -> RouterResult<Option<api::CustomerDetails>> {
        // Validate card's expiry
        migration::validate_card_expiry(&self.data.card_expiry_month, &self.data.card_expiry_year)?;

        // Validate customer ID
        let customer_id = self
            .customer
            .customer_id
            .as_ref()
            .get_required_value("customer_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer.customer_id",
            })?;

        // Fetch customer details if present
        let db = &*self.state.store;
        let key_manager_state: &KeyManagerState = &self.state.into();
        db.find_customer_optional_by_customer_id_merchant_id(
            key_manager_state,
            customer_id,
            self.merchant_account.get_id(),
            self.key_store,
            self.merchant_account.storage_scheme,
        )
        .await
        .inspect_err(|err| logger::info!("Error fetching customer: {:?}", err))
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .map_or(
            // Validate if customer creation is feasible
            if self.customer.name.is_some()
                || self.customer.email.is_some()
                || self.customer.phone.is_some()
            {
                Ok(None)
            } else {
                Err(report!(errors::ApiErrorResponse::MissingRequiredFields {
                    field_names: vec!["customer.name", "customer.email", "customer.phone"],
                }))
            },
            // If found, send back CustomerDetails from DB
            |optional_customer| {
                Ok(optional_customer.map(|customer| api::CustomerDetails {
                    id: customer.customer_id.clone(),
                    name: customer.name.clone().map(|name| name.into_inner()),
                    email: customer.email.clone().map(Email::from),
                    phone: customer.phone.clone().map(|phone| phone.into_inner()),
                    phone_country_code: customer.phone_country_code.clone(),
                }))
            },
        )
    }

    pub async fn create_customer(&self) -> RouterResult<api::CustomerDetails> {
        let db = &*self.state.store;
        let customer_id = self
            .customer
            .customer_id
            .as_ref()
            .get_required_value("customer_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer_id",
            })?;
        let key_manager_state: &KeyManagerState = &self.state.into();

        let encrypted_data = crypto_operation(
            key_manager_state,
            type_name!(domain::Customer),
            CryptoOperation::BatchEncrypt(domain::FromRequestEncryptableCustomer::to_encryptable(
                domain::FromRequestEncryptableCustomer {
                    name: self.customer.name.clone(),
                    email: self
                        .customer
                        .email
                        .clone()
                        .map(|email| email.expose().switch_strategy()),
                    phone: self.customer.phone.clone(),
                },
            )),
            Identifier::Merchant(self.merchant_account.get_id().clone()),
            self.key_store.key.get_inner().peek(),
        )
        .await
        .inspect_err(|err| logger::info!("Error encrypting customer: {:?}", err))
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt customer")?;

        let encryptable_customer =
            domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to form EncryptableCustomer")?;

        let new_customer_id = generate_customer_id_of_default_length();
        let domain_customer = domain::Customer {
            customer_id: new_customer_id.clone(),
            merchant_id: self.merchant_account.get_id().clone(),
            name: encryptable_customer.name,
            email: encryptable_customer.email.map(|email| {
                utils::Encryptable::new(
                    email.clone().into_inner().switch_strategy(),
                    email.into_encrypted(),
                )
            }),
            phone: encryptable_customer.phone,
            description: None,
            phone_country_code: self.customer.phone_country_code.to_owned(),
            metadata: None,
            connector_customer: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            address_id: None,
            default_payment_method_id: None,
            updated_by: None,
            version: hyperswitch_domain_models::consts::API_VERSION,
        };

        db.insert_customer(
            domain_customer,
            key_manager_state,
            self.key_store,
            self.merchant_account.storage_scheme,
        )
        .await
        .inspect_err(|err| logger::info!("Error creating a customer: {:?}", err))
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed to insert customer [id - {:?}] for merchant [id - {:?}]",
                customer_id,
                self.merchant_account.get_id()
            )
        })?;

        Ok(api::CustomerDetails {
            id: new_customer_id,
            name: self.customer.name.clone(),
            email: self.customer.email.clone(),
            phone: self.customer.phone.clone(),
            phone_country_code: self.customer.phone_country_code.clone(),
        })
    }

    pub async fn store_card_and_token_in_locker(
        &self,
        network_token: &NetworkTokenizationResponse,
        card: &domain::CardDetail,
        customer_id: &id_type::CustomerId,
    ) -> RouterResult<StoreLockerResponse> {
        let stored_card_resp = self.store_card_in_locker(card, customer_id).await?;
        let stored_token_resp = self
            .store_network_token_in_locker(
                network_token,
                customer_id,
                card.card_holder_name.clone(),
                card.nick_name.clone(),
            )
            .await?;
        let store_locker_response = StoreLockerResponse {
            store_card_resp: stored_card_resp,
            store_token_resp: stored_token_resp,
        };
        Ok(store_locker_response)
    }

    pub async fn store_card_in_locker(
        &self,
        card: &domain::CardDetail,
        customer_id: &id_type::CustomerId,
    ) -> RouterResult<pm_transformers::StoreCardRespPayload> {
        let merchant_id = self.merchant_account.get_id();
        let locker_req =
            pm_transformers::StoreLockerReq::LockerCard(pm_transformers::StoreCardReq {
                merchant_id: merchant_id.clone(),
                merchant_customer_id: customer_id.clone(),
                card: payment_methods_api::Card {
                    card_number: card.card_number.clone(),
                    card_exp_month: card.card_exp_month.clone(),
                    card_exp_year: card.card_exp_year.clone(),
                    card_isin: Some(card.card_number.get_card_isin().clone()),
                    name_on_card: card.card_holder_name.clone(),
                    nick_name: card
                        .nick_name
                        .as_ref()
                        .map(|nick_name| nick_name.clone().expose()),
                    card_brand: None,
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

    pub async fn create_payment_method(
        &self,
        stored_locker_resp: &StoreLockerResponse,
        network_token_details: &NetworkTokenizationResponse,
        card_details: &domain::CardDetail,
        customer_id: &id_type::CustomerId,
    ) -> RouterResult<domain::PaymentMethod> {
        let payment_method_id = common_utils::generate_id(consts::ID_LENGTH, "pm");

        // Form encrypted PM data (original card)
        let enc_pm_data = self.encrypt_card(card_details, true).await?;

        // Form encrypted network token data
        let enc_token_data = self
            .encrypt_network_token(network_token_details, card_details, true)
            .await?;

        // Form PM create entry
        let payment_method_create = api::PaymentMethodCreate {
            payment_method: Some(api_enums::PaymentMethod::Card),
            payment_method_type: card_details
                .card_type
                .as_ref()
                .and_then(|card_type| api_enums::PaymentMethodType::from_str(card_type).ok()),
            payment_method_issuer: card_details.card_issuer.clone(),
            payment_method_issuer_code: None,
            card: Some(api::CardDetail {
                card_number: card_details.card_number.clone(),
                card_exp_month: card_details.card_exp_month.clone(),
                card_exp_year: card_details.card_exp_year.clone(),
                card_holder_name: card_details.card_holder_name.clone(),
                nick_name: card_details.nick_name.clone(),
                card_issuing_country: card_details.card_issuing_country.clone(),
                card_network: card_details.card_network.clone(),
                card_issuer: card_details.card_issuer.clone(),
                card_type: card_details.card_type.clone(),
            }),
            metadata: None,
            customer_id: Some(customer_id.clone()),
            card_network: card_details
                .card_network
                .as_ref()
                .map(|network| network.to_string()),
            bank_transfer: None,
            wallet: None,
            client_secret: None,
            payment_method_data: None,
            billing: None,
            connector_mandate_details: None,
            network_transaction_id: None,
        };
        create_payment_method(
            self.state,
            &payment_method_create,
            customer_id,
            &payment_method_id,
            Some(stored_locker_resp.store_card_resp.card_reference.clone()),
            self.merchant_account.get_id(),
            None,
            None,
            Some(enc_pm_data),
            self.key_store,
            None,
            None,
            None,
            self.merchant_account.storage_scheme,
            None,
            None,
            network_token_details.1.clone(),
            Some(stored_locker_resp.store_token_resp.card_reference.clone()),
            Some(enc_token_data),
        )
        .await
    }
}
