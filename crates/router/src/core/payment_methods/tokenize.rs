use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::payment_methods as payment_methods_api;
use cards::CardNumber;
use common_utils::{
    id_type,
    transformers::{ForeignFrom, ForeignTryFrom},
};
use hyperswitch_domain_models::router_request_types as domain_request_types;
use masking::Secret;
use router_env::logger;

use crate::{
    core::payment_methods::{
        cards::tokenize_card_flow, network_tokenization, transformers as pm_transformers,
    },
    errors::{self, RouterResult},
    services,
    types::{api, domain},
    SessionState,
};

use super::migration;

pub mod card_executor;
pub mod payment_method_executor;

pub use card_executor::*;
pub use payment_method_executor::*;

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
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<Vec<payment_methods_api::CardNetworkTokenizeResponse>> {
    use futures::stream::StreamExt;

    // Process all records in parallel
    let responses = futures::stream::iter(records.into_iter())
        .map(|record| async move {
            let tokenize_request = record.data.clone();
            Box::pin(tokenize_card_flow(
                state,
                domain::CardNetworkTokenizeRequest::foreign_from(record),
                merchant_account,
                key_store,
            ))
            .await
            .unwrap_or_else(|e| {
                let err = e.current_context();
                payment_methods_api::CardNetworkTokenizeResponse {
                    req: Some(tokenize_request),
                    error_code: Some(err.error_code()),
                    error_message: Some(err.error_message()),
                    card_tokenized: false,
                    payment_method_response: None,
                    customer: None,
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
type NetworkTokenizationResponse = (
    network_tokenization::CardNetworkTokenResponsePayload,
    Option<String>,
);

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
    pub card: Option<domain::Card>,

    /// Network token details
    pub network_token: Option<&'a network_tokenization::CardNetworkTokenResponsePayload>,

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
    customer: Option<&'a domain_request_types::CustomerDetails>,
}

// State machine
pub trait State {}
pub trait TransitionTo<S: State> {}

// Trait for network tokenization
#[async_trait::async_trait]
pub trait NetworkTokenizationProcess<'a, D> {
    fn new(
        state: &'a SessionState,
        key_store: &'a domain::MerchantKeyStore,
        merchant_account: &'a domain::MerchantAccount,
        data: &'a D,
        customer: Option<&'a domain_request_types::CustomerDetails>,
    ) -> Self;
    async fn fetch_bin_details(
        &self,
        card_number: CardNumber,
    ) -> RouterResult<Option<diesel_models::CardInfo>>;
    async fn tokenize_card(
        &self,
        customer_id: &id_type::CustomerId,
        card: &domain::Card,
    ) -> RouterResult<NetworkTokenizationResponse>;
    async fn store_network_token_in_locker(
        &self,
        network_token: &NetworkTokenizationResponse,
        customer_id: &id_type::CustomerId,
        card_holder_name: Option<Secret<String>>,
        nick_name: Option<Secret<String>>,
    ) -> RouterResult<pm_transformers::StoreCardRespPayload>;
}
