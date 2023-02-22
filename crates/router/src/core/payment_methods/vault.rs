use std::str::FromStr;

use common_utils::generate_id_with_default_len;
use error_stack::{IntoReport, ResultExt};
use masking::PeekInterface;
use router_env::{instrument, tracing};

#[cfg(not(feature = "basilisk"))]
use crate::types::storage;
use crate::{
    core::errors::{self, CustomResult, RouterResult},
    logger, routes,
    types::api,
    utils::{self, StringExt},
};
#[cfg(feature = "basilisk")]
use crate::{core::payment_methods::transformers as payment_methods, services, utils::BytesExt};
#[cfg(feature = "basilisk")]
const VAULT_SERVICE_NAME: &str = "CARD";
#[cfg(feature = "basilisk")]
const VAULT_VERSION: &str = "0";

pub struct SupplementaryVaultData {
    pub customer_id: Option<String>,
    pub payment_method_id: Option<String>,
}

pub trait Vaultable: Sized {
    fn get_value1(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError>;
    fn get_value2(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        Ok(String::new())
    }
    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError>;
}

impl Vaultable for api::Card {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedCardValue1 {
            card_number: self.card_number.peek().clone(),
            exp_year: self.card_exp_year.peek().clone(),
            exp_month: self.card_exp_month.peek().clone(),
            name_on_card: Some(self.card_holder_name.peek().clone()),
            nickname: None,
            card_last_four: None,
            card_token: None,
        };

        utils::Encode::<api::TokenizedCardValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedCardValue2 {
            card_security_code: Some(self.card_cvc.peek().clone()),
            card_fingerprint: None,
            external_id: None,
            customer_id,
            payment_method_id: None,
        };

        utils::Encode::<api::TokenizedCardValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode card value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedCardValue1 = value1
            .parse_struct("TokenizedCardValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value1")?;

        let value2: api::TokenizedCardValue2 = value2
            .parse_struct("TokenizedCardValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into card value2")?;

        let card = Self {
            card_number: value1.card_number.into(),
            card_exp_month: value1.exp_month.into(),
            card_exp_year: value1.exp_year.into(),
            card_holder_name: value1.name_on_card.unwrap_or_default().into(),
            card_cvc: value2.card_security_code.unwrap_or_default().into(),
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: value2.payment_method_id,
        };

        Ok((card, supp_data))
    }
}

impl Vaultable for api::WalletData {
    fn get_value1(&self, _customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = api::TokenizedWalletValue1 {
            issuer: self.issuer_name.to_string(),
            token: self.token.clone(),
        };

        utils::Encode::<api::TokenizedWalletValue1>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = api::TokenizedWalletValue2 { customer_id };

        utils::Encode::<api::TokenizedWalletValue2>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode wallet data value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: api::TokenizedWalletValue1 = value1
            .parse_struct("TokenizedWalletValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value1")?;

        let value2: api::TokenizedWalletValue2 = value2
            .parse_struct("TokenizedWalletValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into wallet data value2")?;

        let wallet = Self {
            issuer_name: api::enums::WalletIssuer::from_str(&value1.issuer)
                .into_report()
                .change_context(errors::VaultError::ResponseDeserializationFailed)
                .attach_printable("Invalid issuer name when deserializing wallet data")?,
            token: value1.token,
        };

        let supp_data = SupplementaryVaultData {
            customer_id: value2.customer_id,
            payment_method_id: None,
        };

        Ok((wallet, supp_data))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum VaultPaymentMethod {
    Card(String),
    Wallet(String),
}

impl Vaultable for api::PaymentMethod {
    fn get_value1(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value1 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value1(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value1(customer_id)?),
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported")?,
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value1)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payment method value1")
    }

    fn get_value2(&self, customer_id: Option<String>) -> CustomResult<String, errors::VaultError> {
        let value2 = match self {
            Self::Card(card) => VaultPaymentMethod::Card(card.get_value2(customer_id)?),
            Self::Wallet(wallet) => VaultPaymentMethod::Wallet(wallet.get_value2(customer_id)?),
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported")?,
        };

        utils::Encode::<VaultPaymentMethod>::encode_to_string_of_json(&value2)
            .change_context(errors::VaultError::RequestEncodingFailed)
            .attach_printable("Failed to encode payment method value2")
    }

    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError> {
        let value1: VaultPaymentMethod = value1
            .parse_struct("PaymentMethodValue1")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into payment method value 1")?;

        let value2: VaultPaymentMethod = value2
            .parse_struct("PaymentMethodValue2")
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Could not deserialize into payment method value 2")?;

        match (value1, value2) {
            (VaultPaymentMethod::Card(mvalue1), VaultPaymentMethod::Card(mvalue2)) => {
                let (card, supp_data) = api::Card::from_values(mvalue1, mvalue2)?;
                Ok((Self::Card(card), supp_data))
            }
            (VaultPaymentMethod::Wallet(mvalue1), VaultPaymentMethod::Wallet(mvalue2)) => {
                let (wallet, supp_data) = api::WalletData::from_values(mvalue1, mvalue2)?;
                Ok((Self::Wallet(wallet), supp_data))
            }
            _ => Err(errors::VaultError::PaymentMethodNotSupported)
                .into_report()
                .attach_printable("Payment method not supported"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MockTokenizeDBValue {
    pub value1: String,
    pub value2: String,
}

pub struct Vault;

#[cfg(not(feature = "basilisk"))]
impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &routes::AppState,
        lookup_key: &str,
    ) -> RouterResult<(Option<api::PaymentMethod>, SupplementaryVaultData)> {
        let config = state
            .store
            .find_config_by_key(lookup_key)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Could not find payment method in vault")?;

        let tokenize_value: MockTokenizeDBValue = config
            .config
            .parse_struct("MockTokenizeDBValue")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to deserialize Mock tokenize db value")?;

        let (payment_method, supp_data) =
            api::PaymentMethod::from_values(tokenize_value.value1, tokenize_value.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), supp_data))
    }

    #[instrument(skip_all)]
    pub async fn store_payment_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payment_method: &api::PaymentMethod,
        customer_id: Option<String>,
    ) -> RouterResult<String> {
        let value1 = payment_method
            .get_value1(customer_id.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value1 for locker")?;

        let value2 = payment_method
            .get_value2(customer_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value12 for locker")?;

        let lookup_key = token_id.unwrap_or_else(|| generate_id_with_default_len("token"));

        let db_value = MockTokenizeDBValue { value1, value2 };

        let value_string =
            utils::Encode::<MockTokenizeDBValue>::encode_to_string_of_json(&db_value)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encode payment method as mock tokenize db value")?;

        let already_present = state.store.find_config_by_key(&lookup_key).await;

        if already_present.is_err() {
            let config = storage::ConfigNew {
                key: lookup_key.clone(),
                config: value_string,
            };

            state
                .store
                .insert_config(config)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Mock tokenization save to db failed insert")?;
        } else {
            let config_update = storage::ConfigUpdate::Update {
                config: Some(value_string),
            };
            state
                .store
                .update_config_by_key(&lookup_key, config_update)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Mock tokenization save to db failed update")?;
        }

        Ok(lookup_key)
    }

    #[instrument(skip_all)]
    pub async fn delete_locker_payment_method_by_lookup_key(
        state: &routes::AppState,
        lookup_key: &Option<String>,
    ) {
        let db = &*state.store;
        if let Some(id) = lookup_key {
            match db.delete_config_by_key(id).await {
                Ok(_) => logger::info!("Card Deleted from locker mock up"),
                Err(err) => logger::error!("Err: Card Delete from locker Failed : {}", err),
            }
        }
    }
}

#[cfg(feature = "basilisk")]
impl Vault {
    #[instrument(skip_all)]
    pub async fn get_payment_method_data_from_locker(
        state: &routes::AppState,
        lookup_key: &str,
    ) -> RouterResult<(Option<api::PaymentMethod>, SupplementaryVaultData)> {
        let de_tokenize = get_tokenized_data(state, lookup_key, true).await?;
        let (payment_method, customer_id) =
            api::PaymentMethod::from_values(de_tokenize.value1, de_tokenize.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), customer_id))
    }

    #[instrument(skip_all)]
    pub async fn store_payment_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payment_method: &api::PaymentMethod,
        customer_id: Option<String>,
    ) -> RouterResult<String> {
        let value1 = payment_method
            .get_value1(customer_id.clone())
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value1 for locker")?;

        let value2 = payment_method
            .get_value2(customer_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting Value12 for locker")?;

        let lookup_key = token_id.unwrap_or_else(|| generate_id_with_default_len("token"));

        create_tokenize(state, value1, Some(value2), lookup_key).await
    }

    #[instrument(skip_all)]
    pub async fn delete_locker_payment_method_by_lookup_key(
        state: &routes::AppState,
        lookup_key: &Option<String>,
    ) {
        if let Some(lookup_key) = lookup_key {
            let delete_resp = delete_tokenized_data(state, lookup_key).await;
            match delete_resp {
                Ok(resp) => {
                    if resp == "Ok" {
                        logger::info!("Card From locker deleted Successfully")
                    } else {
                        logger::error!("Error: Deleting Card From Locker : {}", resp)
                    }
                }
                Err(err) => logger::error!("Err: Deleting Card From Locker : {}", err),
            }
        }
    }
}

//------------------------------------------------TokenizeService------------------------------------------------
#[cfg(feature = "basilisk")]
pub async fn create_tokenize(
    state: &routes::AppState,
    value1: String,
    value2: Option<String>,
    lookup_key: String,
) -> RouterResult<String> {
    let payload_to_be_encrypted = api::TokenizePayloadRequest {
        value1,
        value2: value2.unwrap_or_default(),
        lookup_key,
        service_name: VAULT_SERVICE_NAME.to_string(),
    };
    let payload = utils::Encode::<api::TokenizePayloadRequest>::encode_to_string_of_json(
        &payload_to_be_encrypted,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;

    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some(VAULT_VERSION.to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making tokenize request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Decrypt Jwe failed for TokenizePayloadEncrypted")?;
            let get_response: api::GetTokenizePayloadResponse = decrypted_payload
                .parse_struct("GetTokenizePayloadResponse")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error getting GetTokenizePayloadResponse from tokenize response",
                )?;
            Ok(get_response.lookup_key)
        }
        Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}")),
    }
}

#[cfg(feature = "basilisk")]
pub async fn get_tokenized_data(
    state: &routes::AppState,
    lookup_key: &str,
    should_get_value2: bool,
) -> RouterResult<api::TokenizePayloadRequest> {
    let payload_to_be_encrypted = api::GetTokenizePayloadRequest {
        lookup_key: lookup_key.to_string(),
        get_value2: should_get_value2,
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some("0".to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize/get",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making Get Tokenized request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "GetTokenizedApi: Decrypt Jwe failed for TokenizePayloadEncrypted",
                    )?;
            let get_response: api::TokenizePayloadRequest = decrypted_payload
                .parse_struct("TokenizePayloadRequest")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error getting TokenizePayloadRequest from tokenize response")?;
            Ok(get_response)
        }
        Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}")),
    }
}

#[cfg(feature = "basilisk")]
pub async fn delete_tokenized_data(
    state: &routes::AppState,
    lookup_key: &str,
) -> RouterResult<String> {
    let payload_to_be_encrypted = api::DeleteTokenizeByTokenRequest {
        lookup_key: lookup_key.to_string(),
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;
    let encrypted_payload = services::encrypt_jwe(&state.conf.jwekey, &payload)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: services::get_key_id(&state.conf.jwekey).to_string(),
        version: Some("0".to_string()),
    };
    let request = payment_methods::mk_crud_locker_request(
        &state.conf.locker,
        "/tokenize/delete/token",
        create_tokenize_request,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making Delete Tokenized request failed")?;
    let response = services::call_connector_api(state, request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match response {
        Ok(r) => {
            let resp: api::TokenizePayloadEncrypted = r
                .response
                .parse_struct("TokenizePayloadEncrypted")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Decoding Failed for TokenizePayloadEncrypted")?;
            let decrypted_payload =
                services::decrypt_jwe(&state.conf.jwekey, &resp.payload, &resp.key_id)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "DeleteTokenizedApi: Decrypt Jwe failed for TokenizePayloadEncrypted",
                    )?;
            let delete_response = decrypted_payload
                .parse_struct("Delete")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Error getting TokenizePayloadEncrypted from tokenize response",
                )?;
            Ok(delete_response)
        }
        Err(err) => Err(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable(format!("Got 4xx from the basilisk locker: {err:?}")),
    }
}
