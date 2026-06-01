use api_models::{payment_methods::PaymentMethodId, proxy as proxy_api_models};
use common_utils::{
    crypto::{DecodeMessage, GcmAes256},
    encryption::Encryption,
    ext_traits::{BytesExt, Encode, OptionExt},
    id_type,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{behaviour::Conversion, payment_methods};
use hyperswitch_masking::{Mask, PeekInterface};
use serde_json::Value;
use x509_parser::nom::{
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    sequence::{delimited, preceded, terminated},
    IResult,
};

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::{
            cards, fetch_payment_method_by_storage, resolve_storage_type_from_token, vault,
        },
    },
    routes::SessionState,
    types::{domain, payment_methods as pm_types},
};

pub struct ProxyRequestWrapper(pub proxy_api_models::ProxyRequest);
#[derive(Debug)]
pub enum ProxyRecord {
    PaymentMethodRecord(Box<domain::PaymentMethod>),
    VolatilePaymentMethodRecord(Box<domain::PaymentMethod>),
    TokenizationRecord(Box<domain::Tokenization>),
}

impl ProxyRequestWrapper {
    pub async fn get_proxy_record(
        &self,
        state: &SessionState,
        provider: &domain::Provider,
    ) -> RouterResult<ProxyRecord> {
        let token = self
            .0
            .token
            .as_ref()
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "token",
            })
            .attach_printable("token is required for single-token proxy mode")?;

        match self.0.token_type {
            proxy_api_models::TokenType::PaymentMethodId => {
                let pm_id = PaymentMethodId {
                    payment_method_id: token.clone(),
                };
                let pm_id =
                    id_type::GlobalPaymentMethodId::generate_from_string(pm_id.payment_method_id)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

                let payment_method_record = state
                    .store
                    .find_payment_method(
                        provider.get_key_store(),
                        &pm_id,
                        provider.get_account().storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)?;
                Ok(ProxyRecord::PaymentMethodRecord(Box::new(
                    payment_method_record,
                )))
            }
            proxy_api_models::TokenType::TokenizationId => {
                let token_id = id_type::GlobalTokenId::from_string(token.clone().as_str())
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Error while coneverting from string to GlobalTokenId type",
                    )?;
                let db = state.store.as_ref();

                let tokenization_record = db
                    .get_entity_id_vault_id_by_token_id(&token_id, provider.get_key_store())
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while fetching tokenization record from vault")?;

                Ok(ProxyRecord::TokenizationRecord(Box::new(
                    tokenization_record,
                )))
            }
            proxy_api_models::TokenType::VolatilePaymentMethodId => {
                let pm_id = token.as_str();
                let encryption_key = provider.get_key_store().key.get_inner();

                let redis_conn = state
                    .store
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")?;

                let response = redis_conn.get_key::<bytes::Bytes>(&pm_id.into()).await;

                let payment_method_record = match response {
                    Ok(resp) => {
                        let payment_method = resp
                            .parse_struct::<diesel_models::PaymentMethod>("PaymentMethod")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Error getting PaymentMethod from redis")?;

                        let keymanager_state = &state.into();

                        let domain_payment_method = domain::PaymentMethod::convert_back(
                            keymanager_state,
                            payment_method,
                            provider.get_key_store().key.get_inner(),
                            provider.get_key_store().merchant_id.clone().into(),
                        )
                        .await
                        .change_context(errors::StorageError::EncryptionError)
                        .change_context(errors::ApiErrorResponse::InternalServerError)?;

                        Ok(domain_payment_method)
                    }
                    Err(err) => {
                        Err(err).change_context(errors::ApiErrorResponse::UnprocessableEntity {
                            message: "Token is invalid or expired".into(),
                        })
                    }
                }?;

                Ok(ProxyRecord::VolatilePaymentMethodRecord(Box::new(
                    payment_method_record,
                )))
            }
            proxy_api_models::TokenType::PaymentMethodToken => {
                // 1. Resolve parent token (if any) -> storage type & optional token data
                let (storage_type, card_token_data_opt) =
                    resolve_storage_type_from_token(state, &token.to_string()).await?;

                let pm_id = PaymentMethodId {
                    payment_method_id: token.clone(),
                };

                // 2. Fetch payment method record based on resolved storage type
                let (storage_type, payment_method) = fetch_payment_method_by_storage(
                    state,
                    provider,
                    &pm_id,
                    storage_type,
                    card_token_data_opt,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
                .attach_printable("Failed to fetch payment method by storage")?;

                match storage_type {
                    common_enums::enums::StorageType::Persistent => {
                        Ok(ProxyRecord::PaymentMethodRecord(Box::new(payment_method)))
                    }
                    common_enums::enums::StorageType::Volatile => Ok(
                        ProxyRecord::VolatilePaymentMethodRecord(Box::new(payment_method)),
                    ),
                }
            }
        }
    }

    pub fn get_headers(&self) -> Vec<(String, hyperswitch_masking::Maskable<String>)> {
        self.0
            .headers
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone().into_masked()))
            .collect()
    }

    pub fn get_destination_url(&self) -> &str {
        self.0.destination_url.as_str()
    }

    pub fn get_method(&self) -> common_utils::request::Method {
        self.0.method
    }
}

impl ProxyRecord {
    fn get_vault_id(&self) -> RouterResult<payment_methods::VaultId> {
        match self {
            Self::PaymentMethodRecord(payment_method) => payment_method
                .locker_id
                .clone()
                .get_required_value("vault_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Locker id not present in Payment Method Entry"),
            Self::TokenizationRecord(tokenization_record) => Ok(
                payment_methods::VaultId::generate(tokenization_record.locker_id.clone()),
            ),
            Self::VolatilePaymentMethodRecord(payment_method) => payment_method
                .locker_id
                .clone()
                .get_required_value("vault_id")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Locker id not present in Volatile Payment Method Entry"),
        }
    }

    fn get_customer_id(&self) -> RouterResult<Option<id_type::GlobalCustomerId>> {
        match self {
            Self::PaymentMethodRecord(payment_method) => {
                let customer_id = payment_method
                    .customer_id
                    .clone()
                    .get_required_value("customer_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Customer id not present in Payment Method Entry")?;
                Ok(Some(customer_id))
            }
            Self::TokenizationRecord(tokenization_record) => {
                Ok(Some(tokenization_record.customer_id.clone()))
            }
            Self::VolatilePaymentMethodRecord(payment_method) => {
                Ok(payment_method.customer_id.clone())
            }
        }
    }

    pub async fn get_vault_data(
        &self,
        state: &SessionState,
        platform: domain::Platform,
    ) -> RouterResult<Value> {
        match self {
            Self::PaymentMethodRecord(payment_method) => {
                let customer_id = self
                    .get_customer_id()?
                    .get_required_value("customer_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Locker id not present in Payment Method Entry")?;
                let vault_resp = vault::retrieve_payment_method_from_vault_internal(
                    state,
                    &platform,
                    &self.get_vault_id()?,
                    &customer_id,
                    payment_method.payment_method_type,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while fetching data from vault")?;

                let mut vault_data = vault_resp.data;

                // If vault data is card, try to retrieve CVC from redis and attach it
                if vault_data.get_card().is_some() {
                    let payment_method_id_str =
                        payment_method.get_id().get_string_repr().to_string();
                    let key_store = platform.get_provider().get_key_store();

                    match vault::retrieve_and_delete_cvc_from_payment_token(
                        state,
                        &payment_method_id_str,
                        key_store,
                    )
                    .await
                    {
                        Ok(card_cvc) => {
                            vault_data.set_card_cvc(card_cvc);
                        }
                        Err(err) => {
                            router_env::logger::warn!(
                                "Failed to retrieve CVC from redis: {:?}",
                                err
                            );
                        }
                    }
                }

                Ok(vault_data
                    .encode_to_value()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to serialize vault data")?)
            }
            Self::TokenizationRecord(_) => {
                let customer_id = self
                    .get_customer_id()?
                    .get_required_value("customer_id")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Locker id not present in Tokenization Record")?;
                let vault_request = pm_types::VaultRetrieveRequest {
                    entity_id: customer_id,
                    vault_id: self.get_vault_id()?,
                };

                let vault_data = vault::retrieve_value_from_vault(state, vault_request)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to retrieve vault data")?;

                Ok(vault_data.get("data").cloned().unwrap_or(Value::Null))
            }
            Self::VolatilePaymentMethodRecord(payment_method) => {
                //retrieve from redis
                let vault_id = self.get_vault_id()?;
                let key_store = platform.get_provider().get_key_store();
                let _encryption_key = key_store.key.get_inner();

                let redis_conn = state
                    .store
                    .get_redis_conn()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to get redis connection")?;

                let response = redis_conn
                    .get_and_deserialize_key::<Encryption>(
                        &vault_id.get_string_repr().into(),
                        "Vec<u8>",
                    )
                    .await;

                match response {
                    Ok(resp) => {
                        let decrypted_payload: domain::PaymentMethodVaultingData = cards::decrypt_generic_data(state, Some(resp), key_store)
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to decrypt volatile payment method vault data")?.get_required_value("PaymentMethodVaultingData")
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to get required decrypted volatile payment method vault data")?;

                        let mut vault_data = decrypted_payload.clone();

                        // If vault data is card, try to retrieve CVC from redis and attach it
                        if vault_data.get_card().is_some() {
                            let payment_method_id_str =
                                payment_method.get_id().get_string_repr().to_string();
                            let key_store = platform.get_provider().get_key_store();

                            match vault::retrieve_and_delete_cvc_from_payment_token(
                                state,
                                &payment_method_id_str,
                                key_store,
                            )
                            .await
                            {
                                Ok(card_cvc) => {
                                    vault_data.set_card_cvc(card_cvc);
                                }
                                Err(err) => {
                                    router_env::logger::warn!(
                                        "Failed to retrieve CVC from redis: {:?}",
                                        err
                                    );
                                }
                            }
                        }

                        Ok(vault_data
                            .encode_to_value()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to serialize vault data")?)
                    }
                    Err(err) => {
                        Err(err).change_context(errors::ApiErrorResponse::UnprocessableEntity {
                            message: "Token is invalid or expired".into(),
                        })
                    }
                }
            }
        }
    }
}

/// Intermediate result after fetching payment method from Redis or DB.
/// Indicates whether vault data still needs to be fetched from external vault service.
enum ProxyRecordFetchResult {
    /// Volatile token: vault data found directly in Redis (encrypted). No PM record, no external vault call needed.
    VaultDataDirectlyFromRedis(Value),
    /// Volatile PM: vault data already in Redis (encrypted). Fetch it directly without calling external vault.
    VolatileWithVaultDataInRedis(Box<domain::PaymentMethod>),
    /// Persistent PM: vault data must be fetched from external vault service using this proxy record.
    PersistentNeedsVaultFetch(ProxyRecord),
}

/// Fetches payment method record for a single token in multi-token mode.
/// 
/// Strategy:
///  1. Try Redis first – attempt to deserialize as `PaymentMethod` struct (volatile PM).
///     If found → return `VolatileWithVaultDataInRedis` (vault data is in Redis, no external call needed).
///  2. On Redis miss → fall back to DB (persistent PM).
///     Fetch from DB → return `PersistentNeedsVaultFetch` (external vault call needed).
async fn fetch_proxy_record_for_token(
    state: &SessionState,
    platform: &domain::Platform,
    token: &str,
    token_type: &proxy_api_models::TokenType,
) -> RouterResult<ProxyRecordFetchResult> {
    let provider = platform.get_provider();

    // Check if this is a temporary token (ends with :)
    let is_temp_token = token.ends_with(':');
    
    // Strip the colon for processing
    let token_without_colon = if is_temp_token {
        token.strip_suffix(':').unwrap_or(token)
    } else {
        token
    };

    // Determine token type based on prefix
    let is_payment_method_id = token_without_colon.starts_with("12345_");

    if is_payment_method_id && !is_temp_token {
        // This is a payment method ID → fetch from DB, then call vault
        router_env::logger::info!(
            token = %token,
            "multi-token: detected payment method ID (starts with 12345_), fetching from DB"
        );
    } else if is_temp_token {
        // This is a temporary token (ends with :) → try Redis first
        router_env::logger::debug!(
            token = %token,
            token_without_colon = %token_without_colon,
            "multi-token: detected temporary token (ends with :), trying Redis"
        );

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        // Use the key format: pm_token_{token}_hyperswitch for temporary tokens
        let redis_key = format!("pm_token_{}_hyperswitch", token_without_colon);

        // Temporary tokens store vault data directly as JSON in Redis (not encrypted Encryption objects)
        match redis_conn.get_key::<bytes::Bytes>(&redis_key.clone().into()).await {
            Ok(raw_bytes) => {
                // Parse the JSON vault data directly
                let vault_data: Value = raw_bytes
                    .parse_struct("vault_data")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse vault data JSON from Redis")?;
                
                router_env::logger::info!(
                    token = %token,
                    redis_key = %redis_key,
                    "multi-token: found vault data in Redis (temporary token)"
                );
                
                return Ok(ProxyRecordFetchResult::VaultDataDirectlyFromRedis(vault_data));
            }
            Err(err) => {
                router_env::logger::error!(
                    token = %token,
                    redis_key = %redis_key,
                    error = ?err,
                    "multi-token: Redis error for temporary token - token not found or expired"
                );
                return Err(errors::ApiErrorResponse::UnprocessableEntity {
                    message: format!("Token '{}' is invalid or expired", token),
                })
                .attach_printable(format!("Temporary token not found in Redis: {:?}", err))?;
            }
        }
    } else {
        // This is a volatile token (but not temporary) → try Redis first
        router_env::logger::debug!(
            token = %token,
            "multi-token: detected volatile token, trying Redis"
        );

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        // Use the key format: pm_token_{token}_hyperswitch for volatile tokens
        let redis_key = format!("pm_token_{}_hyperswitch", token_without_colon);

        // Volatile tokens store vault data directly as JSON in Redis (not encrypted Encryption objects)
        match redis_conn.get_key::<bytes::Bytes>(&redis_key.clone().into()).await {
            Ok(raw_bytes) => {
                // Parse the JSON vault data directly
                let vault_data: Value = raw_bytes
                    .parse_struct("vault_data")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to parse vault data JSON from Redis")?;
                
                router_env::logger::info!(
                    token = %token,
                    redis_key = %redis_key,
                    "multi-token: found vault data in Redis (volatile token)"
                );
                
                return Ok(ProxyRecordFetchResult::VaultDataDirectlyFromRedis(vault_data));
            }
            Err(err) => {
                router_env::logger::error!(
                    token = %token,
                    redis_key = %redis_key,
                    error = ?err,
                    "multi-token: Redis error for volatile token - token not found or expired"
                );
                return Err(errors::ApiErrorResponse::UnprocessableEntity {
                    message: format!("Token '{}' is invalid or expired", token),
                })
                .attach_printable(format!("Volatile token not found in Redis: {:?}", err))?;
            }
        }
    }

    // ── Fallback: DB (persistent PM) → will need vault fetch ─────────────────
    let proxy_record = match token_type {
        proxy_api_models::TokenType::TokenizationId => {
            let token_id = id_type::GlobalTokenId::from_string(token_without_colon)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while converting from string to GlobalTokenId type")?;
            let tokenization_record = state
                .store
                .get_entity_id_vault_id_by_token_id(&token_id, provider.get_key_store())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error while fetching tokenization record from vault")?;
            ProxyRecord::TokenizationRecord(Box::new(tokenization_record))
        }
        _ => {
            let global_pm_id = id_type::GlobalPaymentMethodId::generate_from_string(
                token_without_colon.to_string(),
            )
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to generate GlobalPaymentMethodId for DB fallback")?;

            let payment_method_record = state
                .store
                .find_payment_method(
                    provider.get_key_store(),
                    &global_pm_id,
                    provider.get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
                .attach_printable("Payment method not found in DB after Redis miss")?;

            router_env::logger::info!(
                token = %token,
                "multi-token: loaded payment method from DB (persistent)"
            );

            ProxyRecord::PaymentMethodRecord(Box::new(payment_method_record))
        }
    };

    Ok(ProxyRecordFetchResult::PersistentNeedsVaultFetch(proxy_record))
}

/// Fetches complete vault data for a single token string, used in multi-token mode.
///
/// This is a two-phase process:
///  1. Fetch payment method record (from Redis or DB)
///  2. Fetch vault data based on the record type:
///     - Volatile PM: vault data is in Redis (encrypted)
///     - Persistent PM: vault data must be fetched from external vault service
pub async fn get_vault_data_for_token(
    state: &SessionState,
    platform: &domain::Platform,
    token: &str,
    token_type: &proxy_api_models::TokenType,
) -> RouterResult<Value> {
    let fetch_result = fetch_proxy_record_for_token(state, platform, token, token_type).await?;

    match fetch_result {
        ProxyRecordFetchResult::VaultDataDirectlyFromRedis(vault_data) => {
            // Vault data was found directly in Redis – return it
            Ok(vault_data)
        }
        ProxyRecordFetchResult::VolatileWithVaultDataInRedis(volatile_pm) => {
            // Vault data is in Redis – fetch it using the payment method record
            let proxy_record = ProxyRecord::VolatilePaymentMethodRecord(volatile_pm);
            proxy_record.get_vault_data(state, platform.clone()).await
        }
        ProxyRecordFetchResult::PersistentNeedsVaultFetch(proxy_record) => {
            // Vault data must be fetched from external vault service
            proxy_record.get_vault_data(state, platform.clone()).await
        }
    }
}

#[derive(Debug)]
pub struct TokenReference {
    pub field: String,
}

/// A token reference that embeds both the field name and the token value.
/// Parsed from the `{{$field_name: token_value}}` syntax used in multi-token mode.
#[derive(Debug, Clone)]
pub struct MultiTokenReference {
    pub field: String,
    pub token: String,
}

pub fn parse_token(input: &str) -> IResult<&str, TokenReference> {
    let (input, field) = delimited(
        tag("{{"),
        preceded(
            multispace0,
            preceded(
                char('$'),
                terminated(
                    take_while1(|c: char| c.is_alphanumeric() || c == '_'),
                    multispace0,
                ),
            ),
        ),
        tag("}}"),
    )(input)?;

    Ok((
        input,
        TokenReference {
            field: field.to_string(),
        },
    ))
}

/// Parses a multi-token placeholder of the form `{{$field_name: token_value}}`.
pub fn parse_multi_token(input: &str) -> IResult<&str, MultiTokenReference> {
    use x509_parser::nom::{
        bytes::complete::take_while,
        character::complete::space0,
    };

    let (input, _) = tag("{{")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('$')(input)?;
    let (input, field) = take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, token) = take_while(|c: char| c != '}')(input)?;
    let (input, _) = tag("}}")(input)?;

    Ok((
        input,
        MultiTokenReference {
            field: field.trim().to_string(),
            token: token.trim().to_string(),
        },
    ))
}

pub fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}")
}

/// Returns true if the string contains a multi-token placeholder (`{{$field: token}}`).
pub fn contains_multi_token(s: &str) -> bool {
    contains_token(s) && s.contains(':')
}

/// Collects all unique token values from multi-token placeholders in the entire JSON value tree.
pub fn collect_tokens_from_value(value: &Value) -> Vec<String> {
    let mut tokens = Vec::new();
    collect_tokens_recursive(value, &mut tokens);
    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    tokens.retain(|t| seen.insert(t.clone()));
    tokens
}

fn collect_tokens_recursive(value: &Value, tokens: &mut Vec<String>) {
    match value {
        Value::Object(obj) => {
            for v in obj.values() {
                collect_tokens_recursive(v, tokens);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_tokens_recursive(v, tokens);
            }
        }
        Value::String(s) => {
            if contains_multi_token(s) {
                if let Ok((_, multi_ref)) = parse_multi_token(s) {
                    tokens.push(multi_ref.token);
                }
            }
        }
        _ => {}
    }
}
