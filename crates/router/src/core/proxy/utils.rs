use api_models::{payment_methods::PaymentMethodId, proxy as proxy_api_models};
use common_utils::{
    crypto::{DecodeMessage, GcmAes256},
    encryption::Encryption,
    ext_traits::{BytesExt, Encode, OptionExt},
    id_type,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{behaviour::Conversion, payment_methods};
use masking::{Mask, PeekInterface};
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
        payment_methods::{cards, vault},
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
        key_store: &domain::MerchantKeyStore,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> RouterResult<ProxyRecord> {
        let token = &self.0.token;

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
                    .find_payment_method(key_store, &pm_id, storage_scheme)
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
                    .get_entity_id_vault_id_by_token_id(&token_id, key_store)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while fetching tokenization record from vault")?;

                Ok(ProxyRecord::TokenizationRecord(Box::new(
                    tokenization_record,
                )))
            }
            proxy_api_models::TokenType::VolatilePaymentMethodId => {
                let pm_id = token.as_str();
                let encryption_key = key_store.key.get_inner();

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
                            key_store.key.get_inner(),
                            key_store.merchant_id.clone().into(),
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
        }
    }

    pub fn get_headers(&self) -> Vec<(String, masking::Maskable<String>)> {
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
            Self::PaymentMethodRecord(_) => {
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

                Ok(vault_resp
                    .data
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
            Self::VolatilePaymentMethodRecord(_) => {
                //retrieve from redis
                let vault_id = self.get_vault_id()?;
                let key_store = platform.get_provider().get_key_store();
                let encryption_key = key_store.key.get_inner();

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

                        Ok(decrypted_payload
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

#[derive(Debug)]
pub struct TokenReference {
    pub field: String,
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

pub fn contains_token(s: &str) -> bool {
    s.contains("{{") && s.contains("$") && s.contains("}}")
}
