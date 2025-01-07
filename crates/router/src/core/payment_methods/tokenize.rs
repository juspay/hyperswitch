use std::{collections::HashMap, str::FromStr};

use api_models::{enums as api_enums, payment_methods as payment_methods_api};
use cards::CardNumber;
use common_utils::{
    ext_traits::OptionExt,
    generate_customer_id_of_default_length,
    pii::{self, Email},
    type_name,
    types::keymanager::{Identifier, KeyManagerState, ToEncryptable},
};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use masking::{ExposeInterface, PeekInterface, SwitchStrategy};
use utoipa::ToSchema;

use crate::{
    core::payment_methods::{
        cards::{add_card_to_hs_locker, populate_bin_details_for_masked_card},
        network_tokenization,
        transformers::{DataDuplicationCheck, StoreCardReq, StoreLockerReq},
    },
    db,
    errors::{self, RouterResult},
    types::{
        api::{
            self,
            payment_methods::{CardNetworkTokenizeRequest, TokenizeCardRequest},
        },
        domain,
    },
    utils::Encryptable,
    SessionState,
};

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardNetworkTokenizeResponseBuilder<D, S> {
    /// Current state
    state: S,

    /// State data
    data: D,

    /// Response for payment method entry in DB
    pub payment_method_response: Option<api::PaymentMethodResponse>,

    /// Customer details
    pub customer: Option<api::CustomerDetails>,

    /// Card network tokenization status
    pub card_tokenized: Option<bool>,

    /// Card migration status
    pub card_migrated: Option<bool>,

    /// Network token data migration status
    pub network_token_migrated: Option<bool>,

    /// Network transaction ID migration status
    pub network_transaction_id_migrated: Option<bool>,

    /// Error code
    pub error_code: HashMap<String, String>,

    /// Error message
    pub error_message: HashMap<String, String>,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize, ToSchema)]
pub struct CardNetworkTokenizeResponse {
    /// Response for payment method entry in DB
    pub payment_method_response: Option<api::PaymentMethodResponse>,

    /// Customer details
    pub customer: Option<api::CustomerDetails>,

    /// Card network tokenization status
    pub card_tokenized: Option<bool>,

    /// Card migration status
    pub card_migrated: Option<bool>,

    /// Network token data migration status
    pub network_token_migrated: Option<bool>,

    /// Network transaction ID migration status
    pub network_transaction_id_migrated: Option<bool>,

    /// Error code
    pub error_code: HashMap<String, String>,

    /// Error message
    pub error_message: HashMap<String, String>,
}

impl common_utils::events::ApiEventMetric for CardNetworkTokenizeResponse {}

/// Tokenize using card details
pub struct TokenizeWithCard;

/// Tokenize using payment Method ID
pub struct TokenizeWithPmId;

/// Card details validated
pub struct CardValidated;

/// Payment method ID is tokenized
pub struct PaymentMethodValidated;

/// Stored card details are tokenized
pub struct PaymentMethodTokenized;

/// Card details are tokenized
pub struct CardTokenized;

/// Card details are stored in locker
pub struct CardStored;

// Initialize builder for tokenizing raw card details
impl CardNetworkTokenizeResponseBuilder<TokenizeCardRequest, TokenizeWithCard> {
    pub fn new(req: CardNetworkTokenizeRequest, data: TokenizeCardRequest) -> Self {
        CardNetworkTokenizeResponseBuilder {
            data,
            state: TokenizeWithCard,
            customer: req.customer,
            payment_method_response: None,
            card_tokenized: None,
            card_migrated: None,
            network_token_migrated: None,
            network_transaction_id_migrated: None,
            error_code: HashMap::new(),
            error_message: HashMap::new(),
        }
    }
}

// Validations for tokenizing raw card
impl CardNetworkTokenizeResponseBuilder<TokenizeCardRequest, TokenizeWithCard> {
    pub async fn get_or_create_customer(
        mut self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        let db = &*state.store;
        let customer_details = self
            .customer
            .as_ref()
            .get_required_value("customer")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer",
            })?;
        let key_manager_state: &KeyManagerState = &state.into();

        match db
            .find_customer_optional_by_customer_id_merchant_id(
                key_manager_state,
                &customer_details.id,
                merchant_account.get_id(),
                key_store,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?
        {
            // Customer found
            Some(customer) => {
                self.customer = Some(api::CustomerDetails {
                    id: customer.customer_id.clone(),
                    name: customer.name.clone().map(|name| name.into_inner()),
                    email: customer.email.clone().map(Email::from),
                    phone: customer.phone.clone().map(|phone| phone.into_inner()),
                    phone_country_code: customer.phone_country_code.clone(),
                });
                Ok(self)
            }
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
                        Identifier::Merchant(merchant_account.get_id().clone()),
                        key_store.key.get_inner().peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_batchoperation())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to encrypt customer")?;

                    let encryptable_customer =
                        domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to form EncryptableCustomer")?;

                    let domain_customer = domain::Customer {
                        customer_id: generate_customer_id_of_default_length(),
                        merchant_id: merchant_account.get_id().clone(),
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
                        key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to insert customer [id - {:?}] for merchant [id - {:?}]",
                            customer_details.id,
                            merchant_account.get_id()
                        )
                    })?;

                    self.customer = Some(api::CustomerDetails {
                        id: domain_customer.customer_id,
                        name: customer_details.name.clone(),
                        email: customer_details.email.clone(),
                        phone: customer_details.phone.clone(),
                        phone_country_code: customer_details.phone_country_code.clone(),
                    });
                    Ok(self)

                // Throw error if customer creation is not requested
                } else {
                    Err(report!(errors::ApiErrorResponse::MissingRequiredFields {
                        field_names: vec!["customer.name", "customer.email", "customer.phone"],
                    }))
                }
            }
        }
    }

    pub async fn insert_bin_details(
        self,
        card_number: CardNumber,
        db: &dyn db::StorageInterface,
    ) -> RouterResult<CardNetworkTokenizeResponseBuilder<domain::Card, CardValidated>> {
        let card_bin_details =
            populate_bin_details_for_masked_card(&api::MigrateCardDetail::from(&self.data), db)
                .await?;

        Ok(CardNetworkTokenizeResponseBuilder {
            state: CardValidated,
            data: domain::Card {
                card_number,
                card_type: card_bin_details.card_type,
                card_network: card_bin_details.card_network,
                card_issuer: card_bin_details.card_issuer,
                card_issuing_country: card_bin_details.issuer_country,
                card_exp_month: self.data.card_exp_month,
                card_exp_year: self.data.card_exp_year,
                card_cvc: self.data.card_cvc,
                nick_name: self.data.nick_name,
                card_holder_name: self.data.card_holder_name,
                bank_code: None,
            },
            payment_method_response: self.payment_method_response,
            customer: self.customer,
            card_tokenized: self.card_tokenized,
            card_migrated: self.card_migrated,
            network_token_migrated: self.network_token_migrated,
            network_transaction_id_migrated: self.network_transaction_id_migrated,
            error_code: self.error_code,
            error_message: self.error_message,
        })
    }

    pub async fn validate_request(
        self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<CardNetworkTokenizeResponseBuilder<domain::Card, CardValidated>> {
        // Validate card number
        let card_number = CardNumber::from_str(self.data.card_number.peek()).change_context(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid card number".to_string(),
            },
        )?;

        // Validate and insert customer details
        let builder_with_customer = self
            .get_or_create_customer(state, merchant_account, key_store)
            .await?;

        // Update card details after BIN lookup
        builder_with_customer
            .insert_bin_details(card_number, &*state.store)
            .await
    }
}

// Tokenize raw card details
impl CardNetworkTokenizeResponseBuilder<domain::Card, CardValidated> {
    pub async fn tokenize_card(
        self,
        state: &SessionState,
    ) -> RouterResult<
        CardNetworkTokenizeResponseBuilder<
            (
                network_tokenization::CardNetworkTokenResponsePayload,
                Option<String>,
            ),
            CardTokenized,
        >,
    > {
        match network_tokenization::make_card_network_tokenization_request(
            state,
            &self.data,
            &self
                .customer
                .as_ref()
                .get_required_value("customer")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "customer",
                })?
                .id
                .clone(),
        )
        .await
        {
            Ok(data) => Ok(CardNetworkTokenizeResponseBuilder {
                card_tokenized: Some(true),
                state: CardTokenized,
                data,
                payment_method_response: self.payment_method_response,
                customer: self.customer,
                card_migrated: self.card_migrated,
                network_token_migrated: self.network_token_migrated,
                network_transaction_id_migrated: self.network_transaction_id_migrated,
                error_code: self.error_code,
                error_message: self.error_message,
            }),
            Err(err) => Err(err.change_context(errors::ApiErrorResponse::InternalServerError)),
        }
    }
}

// Store in locker and create payment method entry
impl
    CardNetworkTokenizeResponseBuilder<
        (
            network_tokenization::CardNetworkTokenResponsePayload,
            Option<String>,
        ),
        CardTokenized,
    >
{
    pub async fn store_in_locker(
        self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<api::PaymentMethodResponse> {
        let customer_details = self
            .customer
            .get_required_value("customer")
            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "customer",
            })?;
        let network_token = self.data.0;
        let locker_req = StoreLockerReq::LockerCard(StoreCardReq {
            merchant_id: merchant_account.get_id().clone(),
            merchant_customer_id: customer_details.id.to_owned(),
            card: payment_methods_api::Card {
                card_number: network_token.token,
                card_exp_month: network_token.token_expiry_month,
                card_exp_year: network_token.token_expiry_year,
                card_brand: Some(network_token.card_brand.to_string()),
                card_isin: Some(network_token.token_isin),
                name_on_card: None, // TODO: Fetch from request
                nick_name: None,    // TODO: Fetch from request
            },
            requestor_card_reference: None,
            ttl: state.conf.locker.ttl_for_storage_in_secs,
        });

        let stored_resp = add_card_to_hs_locker(
            state,
            &locker_req,
            &customer_details.id,
            api_enums::LockerChoice::HyperswitchCardVault,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

        Ok(api::PaymentMethodResponse {
            merchant_id: todo!(),
            customer_id: todo!(),
            payment_method_id: todo!(),
            payment_method: todo!(),
            payment_method_type: todo!(),
            card: todo!(),
            recurring_enabled: todo!(),
            installment_payment_enabled: todo!(),
            payment_experience: todo!(),
            metadata: todo!(),
            created: todo!(),
            bank_transfer: todo!(),
            last_used_at: todo!(),
            client_secret: todo!(),
        })
    }
    pub async fn create_payment_method(
        self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, CardStored>>
    {
        let res = self.store_in_locker(state, merchant_account).await?;
        Ok(CardNetworkTokenizeResponseBuilder {
            data: res,
            state: CardStored,
            payment_method_response: todo!(),
            customer: todo!(),
            card_tokenized: todo!(),
            card_migrated: todo!(),
            network_token_migrated: todo!(),
            network_transaction_id_migrated: todo!(),
            error_code: todo!(),
            error_message: todo!(),
        })
    }
}

// Initialize builder for tokenizing saved cards
impl CardNetworkTokenizeResponseBuilder<String, TokenizeWithPmId> {
    pub fn new(req: CardNetworkTokenizeRequest, data: String) -> Self {
        CardNetworkTokenizeResponseBuilder {
            data,
            state: TokenizeWithPmId,
            customer: req.customer,
            payment_method_response: None,
            card_tokenized: None,
            card_migrated: None,
            network_token_migrated: None,
            network_transaction_id_migrated: None,
            error_code: HashMap::new(),
            error_message: HashMap::new(),
        }
    }
}

// Build return response
impl CardNetworkTokenizeResponseBuilder<api::PaymentMethodResponse, CardStored> {
    pub fn build(self) -> CardNetworkTokenizeResponse {
        CardNetworkTokenizeResponse {
            payment_method_response: self.payment_method_response,
            customer: self.customer,
            card_tokenized: self.card_tokenized,
            card_migrated: self.card_migrated,
            network_token_migrated: self.network_token_migrated,
            network_transaction_id_migrated: self.network_transaction_id_migrated,
            error_code: self.error_code,
            error_message: self.error_message,
        }
    }
}
