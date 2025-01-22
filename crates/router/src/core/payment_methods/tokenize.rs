use std::str::FromStr;

use actix_multipart::form::{bytes::Bytes, text::Text, MultipartForm};
use api_models::{enums as api_enums, payment_methods as payment_methods_api};
use cards::CardNumber;
use common_utils::{
    consts,
    ext_traits::OptionExt,
    generate_customer_id_of_default_length, id_type,
    pii::Email,
    transformers::{ForeignFrom, ForeignTryFrom},
    type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use masking::{ExposeInterface, PeekInterface, Secret, SwitchStrategy};
use rdkafka::message::ToBytes;
use router_env::logger;

use crate::{
    core::payment_methods::{
        cards::{
            add_card_to_hs_locker, create_encrypted_data, create_payment_method, tokenize_card_flow,
        },
        network_tokenization,
        transformers::{StoreCardReq, StoreCardRespPayload, StoreLockerReq},
    },
    errors::{self, RouterResult},
    services,
    types::{
        api,
        domain::{
            self,
            bulk_tokenization::{
                CardNetworkTokenizeRecord, CardNetworkTokenizeRequest, TokenizeCardRequest,
                TokenizePaymentMethodRequest,
            },
        },
    },
    utils::Encryptable,
    SessionState,
};

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
        .deserialize::<CardNetworkTokenizeRecord>()
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
    merchant_id: &id_type::MerchantId,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<Vec<payment_methods_api::CardNetworkTokenizeResponse>> {
    use futures::stream::StreamExt;

    // Process all records in parallel
    let responses = futures::stream::iter(records.into_iter())
        .map(|record| async move {
            let tokenize_request = record.data.clone();
            tokenize_card_flow(
                &state,
                CardNetworkTokenizeRequest::foreign_from(record),
                &merchant_id,
                &merchant_account,
                &key_store,
            )
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

// Builder
pub struct CardNetworkTokenizeResponseBuilder<D, S: State> {
    /// Current state
    state: std::marker::PhantomData<S>,

    /// State data
    data: D,

    /// Response for payment method entry in DB
    pub payment_method_response: Option<api::PaymentMethodResponse>,

    /// Customer details
    pub customer: Option<api::CustomerDetails>,

    /// Card network tokenization status
    pub card_tokenized: bool,

    /// Error code
    pub error_code: Option<String>,

    /// Error message
    pub error_message: Option<String>,
}

// Async executor
pub struct CardNetworkTokenizeExecutor<'a> {
    req: &'a CardNetworkTokenizeRequest,
    state: &'a SessionState,
    merchant_account: &'a domain::MerchantAccount,
    key_store: &'a domain::MerchantKeyStore,
}

type NetworkTokenizationResponse = (
    network_tokenization::CardNetworkTokenResponsePayload,
    Option<String>,
);

// State machine
pub trait State {}
pub trait TransitionTo<D, S: State> {}

// All available states
pub struct TokenizeWithCard;
pub struct CardValidated;
pub struct CustomerAssigned;
pub struct CardDetailsAssigned;
pub struct CardTokenized;
pub struct CardTokenStored;
pub struct PaymentMethodCreated;

pub struct TokenizeWithPmId;
pub struct PmValidated;
pub struct PmFetched;
pub struct PmAssigned;
pub struct PmTokenized;
pub struct PmTokenStored;
pub struct PmTokenUpdated;

impl State for TokenizeWithCard {}
impl State for CardValidated {}
impl State for CustomerAssigned {}
impl State for CardDetailsAssigned {}
impl State for CardTokenized {}
impl State for CardTokenStored {}
impl State for PaymentMethodCreated {}

impl State for TokenizeWithPmId {}
impl State for PmValidated {}
impl State for PmFetched {}
impl State for PmAssigned {}
impl State for PmTokenized {}
impl State for PmTokenStored {}
impl State for PmTokenUpdated {}

// Type safe transition
impl<D1, S1: State> CardNetworkTokenizeResponseBuilder<D1, S1> {
    pub fn transition<F, D2, S2>(self, f: F) -> CardNetworkTokenizeResponseBuilder<D2, S2>
    where
        S1: TransitionTo<D2, S2>,
        S2: State,
        F: FnOnce(D1) -> D2,
    {
        CardNetworkTokenizeResponseBuilder {
            state: std::marker::PhantomData::<S2>,
            data: f(self.data),
            customer: self.customer,
            payment_method_response: self.payment_method_response,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}

// State machine for card tokenization
impl TransitionTo<&TokenizeCardRequest, CardValidated> for TokenizeWithCard {}
impl TransitionTo<api::CustomerDetails, CustomerAssigned> for CardValidated {}
impl TransitionTo<domain::Card, CardDetailsAssigned> for CustomerAssigned {}
impl TransitionTo<NetworkTokenizationResponse, CardTokenized> for CardDetailsAssigned {}
impl TransitionTo<api::PaymentMethodResponse, CardTokenStored> for CardTokenized {}
impl TransitionTo<api::PaymentMethodResponse, PaymentMethodCreated> for CardTokenStored {}

impl<'a> CardNetworkTokenizeExecutor<'a> {
    pub fn new(
        req: &'a CardNetworkTokenizeRequest,
        state: &'a SessionState,
        merchant_account: &'a domain::MerchantAccount,
        key_store: &'a domain::MerchantKeyStore,
    ) -> Self {
        Self {
            req,
            state,
            merchant_account,
            key_store,
        }
    }

    pub fn validate_card_number(&self, card_number: Secret<String>) -> RouterResult<CardNumber> {
        CardNumber::from_str(card_number.peek()).change_context(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid card number".to_string(),
            },
        )
    }

    pub async fn get_or_create_customer(&self) -> RouterResult<api::CustomerDetails> {
        let db = &*self.state.store;
        let customer_details = self
            .req
            .customer
            .as_ref()
            .get_required_value("customer")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer",
            })?;
        let customer_id = customer_details
            .customer_id
            .as_ref()
            .get_required_value("customer_id")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer_id",
            })?;
        let key_manager_state: &KeyManagerState = &self.state.into();

        match db
            .find_customer_optional_by_customer_id_merchant_id(
                key_manager_state,
                customer_id,
                self.merchant_account.get_id(),
                self.key_store,
                self.merchant_account.storage_scheme,
            )
            .await
            .inspect_err(|err| logger::info!("Error fetching customer: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)?
        {
            // Customer found
            Some(customer) => Ok(api::CustomerDetails {
                id: customer.customer_id.clone(),
                name: customer.name.clone().map(|name| name.into_inner()),
                email: customer.email.clone().map(Email::from),
                phone: customer.phone.clone().map(|phone| phone.into_inner()),
                phone_country_code: customer.phone_country_code.clone(),
            }),
            // Customer not found
            None => {
                if customer_details.name.is_some()
                    || customer_details.email.is_some()
                    || customer_details.phone.is_some()
                {
                    let encrypted_data = crypto_operation(
                        key_manager_state,
                        type_name!(domain::Customer),
                        CryptoOperation::BatchEncrypt(
                            domain::FromRequestEncryptableCustomer::to_encryptable(
                                domain::FromRequestEncryptableCustomer {
                                    name: customer_details.name.clone(),
                                    email: customer_details
                                        .email
                                        .clone()
                                        .map(|email| email.expose().switch_strategy()),
                                    phone: customer_details.phone.clone(),
                                },
                            ),
                        ),
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

                    let domain_customer = domain::Customer {
                        customer_id: generate_customer_id_of_default_length(),
                        merchant_id: self.merchant_account.get_id().clone(),
                        name: encryptable_customer.name,
                        email: encryptable_customer.email.map(|email| {
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            )
                        }),
                        phone: encryptable_customer.phone,
                        description: None,
                        phone_country_code: customer_details.phone_country_code.to_owned(),
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
                        domain_customer.clone(),
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
                        id: domain_customer.customer_id,
                        name: customer_details.name.clone(),
                        email: customer_details.email.clone(),
                        phone: customer_details.phone.clone(),
                        phone_country_code: customer_details.phone_country_code.clone(),
                    })

                // Throw error if customer creation is not requested
                } else {
                    Err(report!(errors::ApiErrorResponse::MissingRequiredFields {
                        field_names: vec!["customer.name", "customer.email", "customer.phone"],
                    }))
                }
            }
        }
    }

    pub async fn tokenize_card(
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

    pub async fn store_in_locker(
        &self,
        network_token_details: &NetworkTokenizationResponse,
        customer_id: &id_type::CustomerId,
        card_holder_name: Option<Secret<String>>,
        nick_name: Option<Secret<String>>,
    ) -> RouterResult<StoreCardRespPayload> {
        let network_token = &network_token_details.0;
        let merchant_id = self.merchant_account.get_id();
        let locker_req = StoreLockerReq::LockerCard(StoreCardReq {
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

    pub async fn create_payment_method(
        &self,
        stored_card_resp: &StoreCardRespPayload,
        network_token_details: NetworkTokenizationResponse,
        card_details: &domain::Card,
        customer_id: &id_type::CustomerId,
    ) -> RouterResult<domain::PaymentMethod> {
        let payment_method_id = common_utils::generate_id(consts::ID_LENGTH, "pm");

        // Form encrypted PM data (original card)
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
            saved_to_locker: true,
        });
        let enc_pm_data = create_encrypted_data(&self.state.into(), self.key_store, pm_data)
            .await
            .inspect_err(|err| logger::info!("Error encrypting payment method data: {:?}", err))
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

        // Form encrypted network token data (tokenized card)
        let network_token_data = network_token_details.0;
        let token_data = api::PaymentMethodsData::Card(api::CardDetailsPaymentMethod {
            last4_digits: Some(network_token_data.token_last_four),
            expiry_month: Some(network_token_data.token_expiry_month),
            expiry_year: Some(network_token_data.token_expiry_year),
            card_isin: Some(network_token_data.token_isin),
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

        // Create payment method
        create_payment_method(
            self.state,
            &payment_method_create,
            customer_id,
            &payment_method_id,
            Some(stored_card_resp.card_reference.clone()),
            self.merchant_account.get_id(),
            None,
            None,
            Some(enc_pm_data),
            self.key_store,
            None,
            None,
            None, // TODO: update
            self.merchant_account.storage_scheme,
            None,
            None,
            network_token_details.1,
            Some(stored_card_resp.card_reference.clone()),
            Some(enc_token_data),
        )
        .await
    }

    pub async fn validate_payment_method_id(
        &self,
        payment_method_id: &str,
    ) -> RouterResult<(domain::PaymentMethod, String)> {
        let payment_method = self
            .state
            .store
            .find_payment_method(
                &self.state.into(),
                self.key_store,
                payment_method_id,
                self.merchant_account.storage_scheme,
            )
            .await
            .map_err(|err| match err.current_context() {
                storage_impl::errors::StorageError::ValueNotFound(_) => {
                    err.change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: "Invalid payment_method_id".to_string(),
                    })
                }
                e => {
                    logger::info!("Error fetching customer: {:?}", e);
                    err.change_context(errors::ApiErrorResponse::InternalServerError)
                }
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
        if payment_method
            .network_token_requestor_reference_id
            .is_some()
        {
            return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "Card is already tokenized".to_string()
            }));
        }

        // Ensure locker reference is present
        payment_method.locker_id.clone().map_or(
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "locker_id not found for given payment_method_id".to_string()
            })),
            |locker_id| Ok((payment_method, locker_id)),
        )
    }

    pub async fn update_payment_method(
        &self,
        stored_card_resp: &StoreCardRespPayload,
        payment_method: domain::PaymentMethod,
        network_token_details: NetworkTokenizationResponse,
        card_details: &domain::Card,
    ) -> RouterResult<domain::PaymentMethod> {
        // Form encrypted network token data (tokenized card)
        let network_token_data = network_token_details.0;
        let token_data = api::PaymentMethodsData::Card(api::CardDetailsPaymentMethod {
            last4_digits: Some(network_token_data.token_last_four),
            expiry_month: Some(network_token_data.token_expiry_month),
            expiry_year: Some(network_token_data.token_expiry_year),
            card_isin: Some(network_token_data.token_isin),
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
            network_token_requestor_reference_id: network_token_details.1,
            network_token_locker_id: Some(stored_card_resp.card_reference.clone()),
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

// Initialize builder for tokenizing raw card details
impl<'a> CardNetworkTokenizeResponseBuilder<&'a TokenizeCardRequest, TokenizeWithCard> {
    pub fn new(req: &CardNetworkTokenizeRequest, data: &'a TokenizeCardRequest) -> Self {
        Self {
            data,
            state: std::marker::PhantomData::<TokenizeWithCard>,
            customer: req
                .customer
                .as_ref()
                .map(|customer| api::CustomerDetails::foreign_try_from(customer.clone()))
                .transpose()
                .unwrap_or(None),
            payment_method_response: None,
            card_tokenized: false,
            error_code: None,
            error_message: None,
        }
    }
}

// Perform customer related operations
impl CardNetworkTokenizeResponseBuilder<&TokenizeCardRequest, CardValidated> {
    pub fn set_customer_details(
        mut self,
        customer: &api::CustomerDetails,
    ) -> CardNetworkTokenizeResponseBuilder<api::CustomerDetails, CustomerAssigned> {
        self.customer = Some(customer.clone());
        self.transition(|_| customer.to_owned())
    }
}

// Perform card related operations (post BIN lookup update)
impl CardNetworkTokenizeResponseBuilder<api::CustomerDetails, CustomerAssigned> {
    pub fn set_card_details(
        self,
        card_number: CardNumber,
        card_req: &TokenizeCardRequest,
        card_bin_details: &api::CardDetailFromLocker,
    ) -> CardNetworkTokenizeResponseBuilder<domain::Card, CardDetailsAssigned> {
        self.transition(|_| domain::Card {
            card_number,
            card_type: card_bin_details.card_type.clone(),
            card_network: card_bin_details.card_network.clone(),
            card_issuer: card_bin_details.card_issuer.clone(),
            card_issuing_country: card_bin_details.issuer_country.clone(),
            card_exp_month: card_req.card_expiry_month.clone(),
            card_exp_year: card_req.card_expiry_year.clone(),
            card_cvc: card_req.card_cvc.clone(),
            nick_name: card_req.nick_name.clone(),
            card_holder_name: card_req.card_holder_name.clone(),
            bank_code: None,
        })
    }
}

// Perform card network tokenization
impl CardNetworkTokenizeResponseBuilder<domain::Card, CardDetailsAssigned> {
    pub fn get_data(&self) -> domain::Card {
        self.data.clone()
    }
    pub fn set_tokenize_details(
        mut self,
        network_token: &network_tokenization::CardNetworkTokenResponsePayload,
        network_token_requestor_ref_id: Option<&String>,
    ) -> CardNetworkTokenizeResponseBuilder<NetworkTokenizationResponse, CardTokenized> {
        self.card_tokenized = true;
        self.transition(|_| {
            (
                network_token.clone(),
                network_token_requestor_ref_id.cloned(),
            )
        })
    }
}

// Perform locker related operations
impl CardNetworkTokenizeResponseBuilder<NetworkTokenizationResponse, CardTokenized> {
    pub fn set_locker_details(
        self,
        card_bin_details: &api::CardDetailFromLocker,
        stored_card_resp: &StoreCardRespPayload,
        merchant_id: id_type::MerchantId,
        customer_id: id_type::CustomerId,
    ) -> CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, CardTokenStored> {
        self.transition(|_| api::PaymentMethodResponse {
            merchant_id,
            customer_id: Some(customer_id),
            payment_method_id: stored_card_resp.card_reference.clone(),
            payment_method: Some(api_enums::PaymentMethod::Card),
            payment_method_type: card_bin_details
                .card_type
                .as_ref()
                .and_then(|card_type| api_enums::PaymentMethodType::from_str(card_type).ok()),
            card: Some(card_bin_details.clone()),
            recurring_enabled: true,
            installment_payment_enabled: false,
            created: Some(common_utils::date_time::now()),
            payment_experience: None,
            metadata: None,
            bank_transfer: None,
            last_used_at: None,
            client_secret: None,
        })
    }
}

// Create payment method entry
impl CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, CardTokenStored> {
    pub fn set_payment_method_response(
        self,
        payment_method: domain::PaymentMethod,
    ) -> CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, PaymentMethodCreated> {
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: payment_method.merchant_id,
            customer_id: Some(payment_method.customer_id),
            payment_method_id: payment_method.payment_method_id,
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            card: self.data.card.clone(),
            recurring_enabled: self.data.recurring_enabled,
            installment_payment_enabled: self.data.installment_payment_enabled,
            payment_experience: self.data.payment_experience.clone(),
            metadata: self.data.metadata.clone(),
            created: self.data.created,
            bank_transfer: self.data.bank_transfer.clone(),
            last_used_at: self.data.last_used_at,
            client_secret: self.data.client_secret.clone(),
        };
        self.transition(|_| payment_method_response)
    }
}

// Build return response
impl CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, PaymentMethodCreated> {
    pub fn build(self) -> payment_methods_api::CardNetworkTokenizeResponse {
        payment_methods_api::CardNetworkTokenizeResponse {
            payment_method_response: Some(self.data),
            customer: self.customer,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
            req: None,
        }
    }
}

// State machine for payment method ID tokenization
impl TransitionTo<domain::PaymentMethod, PmValidated> for TokenizeWithPmId {}
impl TransitionTo<payment_methods_api::Card, PmFetched> for PmValidated {}
impl TransitionTo<domain::Card, PmAssigned> for PmFetched {}
impl TransitionTo<NetworkTokenizationResponse, PmTokenized> for PmAssigned {}
impl TransitionTo<&StoreCardRespPayload, PmTokenStored> for PmTokenized {}
impl TransitionTo<api::PaymentMethodResponse, PmTokenUpdated> for PmTokenStored {}

// Initialize builder for tokenizing saved cards
impl CardNetworkTokenizeResponseBuilder<TokenizePaymentMethodRequest, TokenizeWithPmId> {
    pub fn new(req: &CardNetworkTokenizeRequest, data: TokenizePaymentMethodRequest) -> Self {
        Self {
            data,
            state: std::marker::PhantomData::<TokenizeWithPmId>,
            customer: req
                .customer
                .as_ref()
                .map(|customer| api::CustomerDetails::foreign_try_from(customer.clone()))
                .transpose()
                .unwrap_or(None),
            payment_method_response: None,
            card_tokenized: false,
            error_code: None,
            error_message: None,
        }
    }
}

impl CardNetworkTokenizeResponseBuilder<domain::PaymentMethod, PmValidated> {
    pub fn get_data(&self) -> domain::PaymentMethod {
        self.data.clone()
    }
}

impl CardNetworkTokenizeResponseBuilder<payment_methods_api::Card, PmFetched> {
    pub fn set_card_details(
        self,
        card_cvc: &Secret<String>,
        card_bin_details: &api::CardDetailFromLocker,
    ) -> CardNetworkTokenizeResponseBuilder<domain::Card, PmAssigned> {
        let card = domain::Card {
            card_number: self.data.card_number.clone(),
            card_exp_year: self.data.card_exp_year.clone(),
            card_exp_month: self.data.card_exp_month.clone(),
            card_cvc: card_cvc.clone(),
            card_holder_name: self.data.name_on_card.clone(),
            nick_name: self
                .data
                .nick_name
                .as_ref()
                .map(|name| Secret::new(name.clone())),
            card_type: card_bin_details.card_type.clone(),
            card_network: card_bin_details.card_network.clone(),
            card_issuer: card_bin_details.card_issuer.clone(),
            card_issuing_country: card_bin_details.issuer_country.clone(),
            bank_code: None,
        };
        self.transition(|_| card)
    }
}

impl CardNetworkTokenizeResponseBuilder<domain::Card, PmAssigned> {
    pub fn get_data(&self) -> domain::Card {
        self.data.clone()
    }
    pub fn set_tokenize_details(
        mut self,
        network_token: &network_tokenization::CardNetworkTokenResponsePayload,
        network_token_requestor_ref_id: Option<&String>,
    ) -> CardNetworkTokenizeResponseBuilder<NetworkTokenizationResponse, PmTokenized> {
        self.card_tokenized = true;
        self.transition(|_| {
            (
                network_token.clone(),
                network_token_requestor_ref_id.cloned(),
            )
        })
    }
}

impl CardNetworkTokenizeResponseBuilder<&StoreCardRespPayload, PmTokenStored> {
    pub fn set_payment_method_response(
        self,
        payment_method: domain::PaymentMethod,
        card_bin_details: &api::CardDetailFromLocker,
    ) -> CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, PmTokenUpdated> {
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: payment_method.merchant_id,
            customer_id: Some(payment_method.customer_id),
            payment_method_id: payment_method.payment_method_id,
            payment_method: payment_method.payment_method,
            payment_method_type: payment_method.payment_method_type,
            card: Some(card_bin_details.clone()),
            recurring_enabled: true,
            installment_payment_enabled: false,
            metadata: payment_method.metadata,
            created: Some(payment_method.created_at),
            last_used_at: Some(payment_method.last_used_at),
            client_secret: payment_method.client_secret,
            payment_experience: None,
            bank_transfer: None,
        };
        self.transition(|_| payment_method_response)
    }
}

impl CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, PmTokenUpdated> {
    pub fn build(self) -> payment_methods_api::CardNetworkTokenizeResponse {
        payment_methods_api::CardNetworkTokenizeResponse {
            payment_method_response: Some(self.data),
            customer: self.customer,
            card_tokenized: self.card_tokenized,
            error_code: self.error_code,
            error_message: self.error_message,
            req: None,
        }
    }
}
