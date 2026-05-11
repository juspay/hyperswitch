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
                    resolve_storage_type_from_token(state, token).await?;

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

/// Fetches vault data for a single token string + token type, used in multi-token mode.
///
/// Strategy (multi-token):
///  1. Try to load the value directly from **Redis** as a `serde_json::Value`.
///  2. If Redis returns an error or a miss, fall back to loading from the **DB** (persistent).
pub async fn get_vault_data_for_token(
    state: &SessionState,
    platform: &domain::Platform,
    token: &str,
    token_type: &proxy_api_models::TokenType,
) -> RouterResult<Value> {
    let provider = platform.get_provider();

    // ── 1. Try Redis – return the raw JSON value directly ────────────────────
    let redis_result: RouterResult<Option<Value>> = async {
        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;

        match redis_conn.get_key::<bytes::Bytes>(&token.into()).await {
            Ok(resp) => {
                let value = resp
                    .parse_struct::<Value>("ProxyTokenValue")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error parsing token value from Redis")?;
                Ok(Some(value))
            }
            // Redis miss / key not found → signal fallback
            Err(_) => Ok(None),
        }
    }
    .await;

    match redis_result {
        Ok(Some(value)) => {
            router_env::logger::info!(
                token = %token,
                "multi-token: loaded value from Redis"
            );
            return Ok(value);
        }
        Ok(None) => {
            router_env::logger::info!(
                token = %token,
                "multi-token: Redis miss, falling back to DB"
            );
        }
        Err(err) => {
            router_env::logger::warn!(
                token = %token,
                error = ?err,
                "multi-token: Redis error, falling back to DB"
            );
        }
    }

    // ── 2. Fallback: DB (persistent) → vault ─────────────────────────────────
    let proxy_record = match token_type {
        proxy_api_models::TokenType::TokenizationId => {
            let token_id = id_type::GlobalTokenId::from_string(token)
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
        // PaymentMethodId | VolatilePaymentMethodId | PaymentMethodToken
        _ => {
            let global_pm_id = id_type::GlobalPaymentMethodId::generate_from_string(
                token.to_string(),
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

    proxy_record.get_vault_data(state, platform.clone()).await
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
