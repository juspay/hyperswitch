use api_models::enums as api_enums;
use common_utils::{
    ext_traits::OptionExt, fp_utils::when, pii::Email, types::keymanager::KeyManagerState,
};
use error_stack::{report, ResultExt};
use masking::Secret;
use router_env::logger;

use super::{
    CardNetworkTokenizeExecutor, NetworkTokenizationBuilder, NetworkTokenizationProcess,
    NetworkTokenizationResponse, State, TransitionTo,
};
use crate::{
    core::payment_methods::transformers as pm_transformers,
    errors::{self, RouterResult},
    types::{api, domain},
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

impl Default for NetworkTokenizationBuilder<'_, TokenizeWithPmId> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> NetworkTokenizationBuilder<'a, TokenizeWithPmId> {
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

impl<'a> NetworkTokenizationBuilder<'a, PmFetched> {
    pub fn set_validate_result(
        self,
        customer: &'a api::CustomerDetails,
    ) -> NetworkTokenizationBuilder<'a, PmValidated> {
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

impl<'a> NetworkTokenizationBuilder<'a, PmValidated> {
    pub fn set_card_details(
        self,
        card_from_locker: &'a api_models::payment_methods::Card,
        optional_card_info: Option<diesel_models::CardInfo>,
        card_cvc: Option<Secret<String>>,
    ) -> NetworkTokenizationBuilder<'a, PmAssigned> {
        let card = domain::CardDetail {
            card_number: card_from_locker.card_number.clone(),
            card_exp_month: card_from_locker.card_exp_month.clone(),
            card_exp_year: card_from_locker.card_exp_year.clone(),
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
            card_cvc,
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
    pub fn get_optional_card_and_cvc(
        &self,
    ) -> (Option<domain::CardDetail>, Option<Secret<String>>) {
        (self.card.clone(), self.card_cvc.clone())
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
            card_cvc: self.card_cvc,
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
            card_cvc: self.card_cvc,
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
            card_cvc: self.card_cvc,
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
            tokenization_data: None,
        }
    }
}

// Specific executor for payment method tokenization
impl CardNetworkTokenizeExecutor<'_, domain::TokenizePaymentMethodRequest> {
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
                errors::DataStorageError::DatabaseError(err)
                    if matches!(
                        err.current_context(),
                        diesel_models::errors::DatabaseError::NotFound
                    ) =>
                {
                    report!(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payment_method_id".into(),
                    })
                }
                errors::DataStorageError::ValueNotFound(_) => {
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
    pub async fn validate_request_and_locker_reference_and_customer(
        &self,
        payment_method: &domain::PaymentMethod,
    ) -> RouterResult<(String, api::CustomerDetails)> {
        // Ensure customer ID matches
        let customer_id_in_req = self
            .customer
            .customer_id
            .clone()
            .get_required_value("customer_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer",
            })?;
        when(payment_method.customer_id != customer_id_in_req, || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Payment method does not belong to the customer".to_string()
            }))
        })?;

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
        let locker_id = payment_method.locker_id.clone().ok_or(report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "locker_id not found for given payment_method_id".to_string()
            }
        ))?;

        // Fetch customer
        let db = &*self.state.store;
        let key_manager_state: &KeyManagerState = &self.state.into();
        let customer = db
            .find_customer_by_customer_id_merchant_id(
                key_manager_state,
                &payment_method.customer_id,
                self.merchant_account.get_id(),
                self.key_store,
                self.merchant_account.storage_scheme,
            )
            .await
            .inspect_err(|err| logger::info!("Error fetching customer: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        let customer_details = api::CustomerDetails {
            id: customer.customer_id.clone(),
            name: customer.name.clone().map(|name| name.into_inner()),
            email: customer.email.clone().map(Email::from),
            phone: customer.phone.clone().map(|phone| phone.into_inner()),
            phone_country_code: customer.phone_country_code.clone(),
        };

        Ok((locker_id, customer_details))
    }
    pub async fn update_payment_method(
        &self,
        store_token_response: &pm_transformers::StoreCardRespPayload,
        payment_method: domain::PaymentMethod,
        network_token_details: &NetworkTokenizationResponse,
        card_details: &domain::CardDetail,
    ) -> RouterResult<domain::PaymentMethod> {
        // Form encrypted network token data
        let enc_token_data = self
            .encrypt_network_token(network_token_details, card_details, true)
            .await?;

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
