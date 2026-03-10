use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::{enums as api_enums, payment_methods as payment_methods_api};
use cards::CardNumber;
use common_utils::{
    crypto::Encryptable,
    id_type,
    transformers::{ForeignFrom, ForeignTryFrom},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::router_request_types as domain_request_types;
use masking::{ExposeInterface, Secret};
use router_env::logger;

use crate::{
    core::payment_methods::{
        cards::{add_card_to_hs_locker, create_encrypted_data, tokenize_card_flow},
        network_tokenization, transformers as pm_transformers,
    },
    errors::{self, RouterResult},
    services,
    types::{api, domain, payment_methods as pm_types},
    SessionState,
};

pub mod card_executor;
pub mod payment_method_executor;

pub use card_executor::*;
pub use payment_method_executor::*;
use rdkafka::message::ToBytes;

#[derive(Debug, MultipartForm)]
pub struct CardNetworkTokenizeForm {
    #[multipart(limit = "1MB")]
    pub file: Bytes,
    pub merchant_id: Text<id_type::MerchantId>,
}

pub fn parse_csv(
    merchant_id: &id_type::MerchantId,
    data: &[u8],
) -> csv::Result<Vec<payment_methods_api::CardNetworkTokenizeRequest>> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(data);
    let mut records = Vec::new();
    let mut id_counter = 0;
    for (i, result) in csv_reader
        .deserialize::<domain::CardNetworkTokenizeRecord>()
        .enumerate()
    {
        match result {
            Ok(mut record) => {
                logger::info!("Parsed Record (line {}): {:?}", i + 1, record);
                id_counter += 1;
                record.line_number = Some(id_counter);
                record.merchant_id = Some(merchant_id.clone());
                match payment_methods_api::CardNetworkTokenizeRequest::foreign_try_from(record) {
                    Ok(record) => {
                        records.push(record);
                    }
                    Err(err) => {
                        logger::error!("Error parsing line {}: {}", i + 1, err.to_string());
                    }
                }
            }
            Err(e) => logger::error!("Error parsing line {}: {}", i + 1, e),
        }
    }
    Ok(records)
}

pub fn get_tokenize_card_form_records(
    form: CardNetworkTokenizeForm,
) -> Result<
    (
        id_type::MerchantId,
        Vec<payment_methods_api::CardNetworkTokenizeRequest>,
    ),
    errors::ApiErrorResponse,
> {
    match parse_csv(&form.merchant_id, form.file.data.to_bytes()) {
        Ok(records) => {
            logger::info!("Parsed a total of {} records", records.len());
            Ok((form.merchant_id.0, records))
        }
        Err(e) => {
            logger::error!("Failed to parse CSV: {:?}", e);
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: e.to_string(),
            })
        }
    }
}

pub async fn tokenize_cards(
    state: &SessionState,
    records: Vec<payment_methods_api::CardNetworkTokenizeRequest>,
    platform: &domain::Platform,
) -> errors::RouterResponse<Vec<payment_methods_api::CardNetworkTokenizeResponse>> {
    use futures::stream::StreamExt;

    // Process all records in parallel
    let responses = futures::stream::iter(records.into_iter())
        .map(|record| async move {
            let tokenize_request = record.data.clone();
            let customer = record.customer.clone();
            Box::pin(tokenize_card_flow(
                state,
                domain::CardNetworkTokenizeRequest::foreign_from(record),
                platform,
            ))
            .await
            .unwrap_or_else(|e| {
                let err = e.current_context();
                payment_methods_api::CardNetworkTokenizeResponse {
                    tokenization_data: Some(tokenize_request),
                    error_code: Some(err.error_code()),
                    error_message: Some(err.error_message()),
                    card_tokenized: false,
                    payment_method_response: None,
                    customer: Some(customer),
                }
            })
        })
        .buffer_unordered(10)
        .collect()
        .await;

    // Return the final response
    Ok(services::ApplicationResponse::Json(responses))
}

// Data types
type NetworkTokenizationResponse = (pm_types::CardNetworkTokenResponsePayload, Option<String>);

pub struct StoreLockerResponse {
    pub store_card_resp: pm_transformers::StoreCardRespPayload,
    pub store_token_resp: pm_transformers::StoreCardRespPayload,
}

// Builder
pub struct NetworkTokenizationBuilder<'a, S: State> {
    /// Current state
    state: std::marker::PhantomData<S>,

    /// Customer details
    pub customer: Option<&'a api::CustomerDetails>,

    /// Card details
    pub card: Option<domain::CardDetail>,

    /// CVC
    pub card_cvc: Option<Secret<String>>,

    /// Network token details
    pub network_token: Option<&'a pm_types::CardNetworkTokenResponsePayload>,

    /// Stored card details
    pub stored_card: Option<&'a pm_transformers::StoreCardRespPayload>,

    /// Stored token details
    pub stored_token: Option<&'a pm_transformers::StoreCardRespPayload>,

    /// Payment method response
    pub payment_method_response: Option<api::PaymentMethodResponse>,

    /// Card network tokenization status
    pub card_tokenized: bool,

    /// Error code
    pub error_code: Option<&'a String>,

    /// Error message
    pub error_message: Option<&'a String>,
}

// Async executor
pub struct CardNetworkTokenizeExecutor<'a, D> {
    pub state: &'a SessionState,
    pub merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
    data: &'a D,
    customer: &'a domain_request_types::CustomerDetails,
}

// State machine
pub trait State {}
pub trait TransitionTo<S: State> {}

// Trait for network tokenization
#[async_trait::async_trait]
pub trait NetworkTokenizationProcess<'a, D> {
    fn new(
        state: &'a SessionState,
        platform: &'a domain::Platform,
        data: &'a D,
        customer: &'a domain_request_types::CustomerDetails,
    ) -> Self;
    async fn encrypt_card(
        &self,
        card_details: &domain::CardDetail,
        saved_to_locker: bool,
    ) -> RouterResult<Encryptable<Secret<serde_json::Value>>>;
    async fn encrypt_network_token(
        &self,
        network_token_details: &NetworkTokenizationResponse,
        card_details: &domain::CardDetail,
        saved_to_locker: bool,
    ) -> RouterResult<Encryptable<Secret<serde_json::Value>>>;
    async fn fetch_bin_details_and_validate_card_network(
        &self,
        card_number: CardNumber,
        card_issuer: Option<&String>,
        card_network: Option<&api_enums::CardNetwork>,
        card_type: Option<&api_models::payment_methods::CardType>,
        card_issuing_country: Option<&String>,
    ) -> RouterResult<Option<diesel_models::CardInfo>>;
    fn validate_card_network(
        &self,
        optional_card_network: Option<&api_enums::CardNetwork>,
    ) -> RouterResult<()>;
    async fn tokenize_card(
        &self,
        customer_id: &id_type::CustomerId,
        card: &domain::CardDetail,
        optional_cvc: Option<Secret<String>>,
    ) -> RouterResult<NetworkTokenizationResponse>;
    async fn store_network_token_in_locker(
        &self,
        network_token: &NetworkTokenizationResponse,
        customer_id: &id_type::CustomerId,
        card_holder_name: Option<Secret<String>>,
        nick_name: Option<Secret<String>>,
    ) -> RouterResult<pm_transformers::StoreCardRespPayload>;
}

// Generic implementation
#[async_trait::async_trait]
impl<'a, D> NetworkTokenizationProcess<'a, D> for CardNetworkTokenizeExecutor<'a, D>
where
    D: Send + Sync + 'static,
{
    fn new(
        state: &'a SessionState,
        platform: &'a domain::Platform,
        data: &'a D,
        customer: &'a domain_request_types::CustomerDetails,
    ) -> Self {
        Self {
            data,
            customer,
            state,
            merchant_account: platform.get_processor().get_account(),
            key_store: platform.get_processor().get_key_store(),
        }
    }
    async fn encrypt_card(
        &self,
        card_details: &domain::CardDetail,
        saved_to_locker: bool,
    ) -> RouterResult<Encryptable<Secret<serde_json::Value>>> {
        let pm_data = api::PaymentMethodsData::Card(api::CardDetailsPaymentMethod {
            last4_digits: Some(card_details.card_number.get_last4()),
            expiry_month: Some(card_details.card_exp_month.clone()),
            expiry_year: Some(card_details.card_exp_year.clone()),
            card_isin: Some(card_details.card_number.get_card_isin()),
            nick_name: card_details.nick_name.clone(),
            card_holder_name: card_details.card_holder_name.clone(),
            issuer_country: card_details.card_issuing_country.clone(),
            card_issuer: card_details.card_issuer.clone(),
            card_network: card_details.card_network.clone(),
            card_type: card_details.card_type.clone(),
            saved_to_locker,
            co_badged_card_data: card_details
                .co_badged_card_data
                .as_ref()
                .map(|data| data.into()),
        });

        create_encrypted_data(&self.state.into(), self.key_store, pm_data)
            .await
            .inspect_err(|err| logger::info!("Error encrypting payment method data: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)
    }
    async fn encrypt_network_token(
        &self,
        network_token_details: &NetworkTokenizationResponse,
        card_details: &domain::CardDetail,
        saved_to_locker: bool,
    ) -> RouterResult<Encryptable<Secret<serde_json::Value>>> {
        let network_token = &network_token_details.0;
        let token_data = api::PaymentMethodsData::Card(api::CardDetailsPaymentMethod {
            last4_digits: Some(network_token.token_last_four.clone()),
            expiry_month: Some(network_token.token_expiry_month.clone()),
            expiry_year: Some(network_token.token_expiry_year.clone()),
            card_isin: Some(network_token.token_isin.clone()),
            nick_name: card_details.nick_name.clone(),
            card_holder_name: card_details.card_holder_name.clone(),
            issuer_country: card_details.card_issuing_country.clone(),
            card_issuer: card_details.card_issuer.clone(),
            card_network: card_details.card_network.clone(),
            card_type: card_details.card_type.clone(),
            saved_to_locker,
            co_badged_card_data: None,
        });
        create_encrypted_data(&self.state.into(), self.key_store, token_data)
            .await
            .inspect_err(|err| logger::info!("Error encrypting network token data: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)
    }
    async fn fetch_bin_details_and_validate_card_network(
        &self,
        card_number: CardNumber,
        card_issuer: Option<&String>,
        card_network: Option<&api_enums::CardNetwork>,
        card_type: Option<&api_models::payment_methods::CardType>,
        card_issuing_country: Option<&String>,
    ) -> RouterResult<Option<diesel_models::CardInfo>> {
        let db = &*self.state.store;
        if card_issuer.is_some()
            && card_network.is_some()
            && card_type.is_some()
            && card_issuing_country.is_some()
        {
            self.validate_card_network(card_network)?;
            return Ok(None);
        }

        db.get_card_info(&card_number.get_card_isin())
            .await
            .attach_printable("Failed to perform BIN lookup")
            .change_context(errors::ApiErrorResponse::InternalServerError)?
            .map(|card_info| {
                self.validate_card_network(card_info.card_network.as_ref())?;
                Ok(card_info)
            })
            .transpose()
    }
    async fn tokenize_card(
        &self,
        customer_id: &id_type::CustomerId,
        card: &domain::CardDetail,
        optional_cvc: Option<Secret<String>>,
    ) -> RouterResult<NetworkTokenizationResponse> {
        network_tokenization::make_card_network_tokenization_request(
            self.state,
            card,
            optional_cvc,
            customer_id,
        )
        .await
        .map_err(|err| {
            logger::error!("Failed to tokenize card with the network: {:?}", err);
            report!(errors::ApiErrorResponse::InternalServerError)
        })
    }
    fn validate_card_network(
        &self,
        optional_card_network: Option<&api_enums::CardNetwork>,
    ) -> RouterResult<()> {
        optional_card_network.map_or(
            Err(report!(errors::ApiErrorResponse::NotSupported {
                message: "Unknown card network".to_string()
            })),
            |card_network| {
                if self
                    .state
                    .conf
                    .network_tokenization_supported_card_networks
                    .card_networks
                    .contains(card_network)
                {
                    Ok(())
                } else {
                    Err(report!(errors::ApiErrorResponse::NotSupported {
                        message: format!(
                            "Network tokenization for {card_network} is not supported",
                        )
                    }))
                }
            },
        )
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
