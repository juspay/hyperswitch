use common_utils::generate_id_with_default_len;
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "basilisk")]
use josekit::jwe;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    configs::settings,
    core::errors::{self, CustomResult, RouterResult},
    logger, routes,
    types::{
        api,
        storage::{self, enums},
    },
    utils::{self, StringExt},
};
#[cfg(feature = "basilisk")]
use crate::{
    core::payment_methods::transformers as payment_methods,
    services::{self, kms},
    utils::BytesExt,
};
#[cfg(feature = "basilisk")]
use crate::{
    db,
    scheduler::{metrics, process_data, utils as process_tracker_utils},
    types::storage::ProcessTrackerExt,
};
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
            card_issuer: None,
            card_network: None,
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
            data: self.to_owned(),
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

        let wallet = value1.data;

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

impl Vaultable for api::PaymentMethodData {
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
    ) -> RouterResult<(Option<api::PaymentMethodData>, SupplementaryVaultData)> {
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
            api::PaymentMethodData::from_values(tokenize_value.value1, tokenize_value.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), supp_data))
    }

    #[instrument(skip_all)]
    pub async fn store_payment_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payment_method: &api::PaymentMethodData,
        customer_id: Option<String>,
        _pm: enums::PaymentMethod,
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
    ) -> RouterResult<(Option<api::PaymentMethodData>, SupplementaryVaultData)> {
        let de_tokenize = get_tokenized_data(state, lookup_key, true).await?;
        let (payment_method, customer_id) =
            api::PaymentMethodData::from_values(de_tokenize.value1, de_tokenize.value2)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Error parsing Payment Method from Values")?;

        Ok((Some(payment_method), customer_id))
    }

    #[instrument(skip_all)]
    pub async fn store_payment_method_data_in_locker(
        state: &routes::AppState,
        token_id: Option<String>,
        payment_method: &api::PaymentMethodData,
        customer_id: Option<String>,
        pm: enums::PaymentMethod,
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

        let lookup_key = create_tokenize(state, value1, Some(value2), lookup_key).await?;
        add_delete_tokenized_data_task(&*state.store, &lookup_key, pm).await?;
        metrics::TOKENIZED_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        Ok(lookup_key)
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
pub fn get_key_id(keys: &settings::Jwekey) -> &str {
    let key_identifier = "1"; // [#46]: Fetch this value from redis or external sources
    if key_identifier == "1" {
        &keys.locker_key_identifier1
    } else {
        &keys.locker_key_identifier2
    }
}

#[cfg(feature = "basilisk")]
async fn get_locker_jwe_keys(
    keys: &settings::Jwekey,
    kms_config: &settings::Kms,
) -> CustomResult<(String, String), errors::EncryptionError> {
    let key_id = get_key_id(keys);
    let (encryption_key, decryption_key) = if key_id == keys.locker_key_identifier1 {
        (&keys.locker_encryption_key1, &keys.locker_decryption_key1)
    } else if key_id == keys.locker_key_identifier2 {
        (&keys.locker_encryption_key2, &keys.locker_decryption_key2)
    } else {
        return Err(errors::EncryptionError.into());
    };

    let public_key = kms::get_kms_client(kms_config)
        .await
        .decrypt(encryption_key)
        .await
        .change_context(errors::EncryptionError)?;
    let private_key = kms::get_kms_client(kms_config)
        .await
        .decrypt(decryption_key)
        .await
        .change_context(errors::EncryptionError)?;

    Ok((public_key, private_key))
}

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

    let (public_key, private_key) = get_locker_jwe_keys(&state.conf.jwekey, &state.conf.kms)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encryption key")?;
    let encrypted_payload = services::encrypt_jwe(payload.as_bytes(), public_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;

    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: get_key_id(&state.conf.jwekey).to_string(),
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
            let alg = jwe::RSA_OAEP_256;
            let decrypted_payload = services::decrypt_jwe(
                &resp.payload,
                get_key_id(&state.conf.jwekey),
                &resp.key_id,
                private_key,
                alg,
            )
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
        service_name: VAULT_SERVICE_NAME.to_string(),
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;

    let (public_key, private_key) = get_locker_jwe_keys(&state.conf.jwekey, &state.conf.kms)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encryption key")?;
    let encrypted_payload = services::encrypt_jwe(payload.as_bytes(), public_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: get_key_id(&state.conf.jwekey).to_string(),
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
            let alg = jwe::RSA_OAEP_256;
            let decrypted_payload = services::decrypt_jwe(
                &resp.payload,
                get_key_id(&state.conf.jwekey),
                &resp.key_id,
                private_key,
                alg,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("GetTokenizedApi: Decrypt Jwe failed for TokenizePayloadEncrypted")?;
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
        service_name: VAULT_SERVICE_NAME.to_string(),
    };
    let payload = serde_json::to_string(&payload_to_be_encrypted)
        .map_err(|_x| errors::ApiErrorResponse::InternalServerError)?;

    let (public_key, private_key) = get_locker_jwe_keys(&state.conf.jwekey, &state.conf.kms)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encryption key")?;
    let encrypted_payload = services::encrypt_jwe(payload.as_bytes(), public_key)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Encrypt JWE response")?;
    let create_tokenize_request = api::TokenizePayloadEncrypted {
        payload: encrypted_payload,
        key_id: get_key_id(&state.conf.jwekey).to_string(),
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
            let alg = jwe::RSA_OAEP_256;
            let decrypted_payload = services::decrypt_jwe(
                &resp.payload,
                get_key_id(&state.conf.jwekey),
                &resp.key_id,
                private_key,
                alg,
            )
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

// ********************************************** PROCESS TRACKER **********************************************
#[cfg(feature = "basilisk")]
pub async fn add_delete_tokenized_data_task(
    db: &dyn db::StorageInterface,
    lookup_key: &str,
    pm: enums::PaymentMethod,
) -> RouterResult<storage::ProcessTracker> {
    let runner = "DELETE_TOKENIZE_DATA_WORKFLOW";
    let current_time = common_utils::date_time::now();
    let tracking_data = serde_json::to_value(storage::TokenizeCoreWorkflow {
        lookup_key: lookup_key.to_owned(),
        pm,
    })
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| format!("unable to convert into value {:?}", lookup_key))?;

    let schedule_time = get_delete_tokenize_schedule_time(db, &pm, 0).await;

    let process_tracker_entry = storage::ProcessTrackerNew {
        id: format!("{}_{}", runner, lookup_key),
        name: Some(String::from(runner)),
        tag: vec![String::from("BASILISK-V3")],
        runner: Some(String::from(runner)),
        retry_count: 0,
        schedule_time,
        rule: String::new(),
        tracking_data,
        business_status: String::from("Pending"),
        status: enums::ProcessTrackerStatus::New,
        event: vec![],
        created_at: current_time,
        updated_at: current_time,
    };
    let response = db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting task in process_tracker: lookup_key: {}",
                lookup_key
            )
        })?;
    Ok(response)
}

#[cfg(feature = "basilisk")]
pub async fn start_tokenize_data_workflow(
    state: &routes::AppState,
    tokenize_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let delete_tokenize_data = serde_json::from_value::<storage::TokenizeCoreWorkflow>(
        tokenize_tracker.tracking_data.clone(),
    )
    .into_report()
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "unable to convert into DeleteTokenizeByTokenRequest {:?}",
            tokenize_tracker.tracking_data
        )
    })?;

    let delete_resp = delete_tokenized_data(state, &delete_tokenize_data.lookup_key).await;
    match delete_resp {
        Ok(resp) => {
            if resp == "Ok" {
                logger::info!("Card From locker deleted Successfully");
                //mark task as finished
                let id = tokenize_tracker.id.clone();
                tokenize_tracker
                    .clone()
                    .finish_with_status(db, format!("COMPLETED_BY_PT_{id}"))
                    .await?;
            } else {
                logger::error!("Error: Deleting Card From Locker : {}", resp);
                retry_delete_tokenize(db, &delete_tokenize_data.pm, tokenize_tracker.to_owned())
                    .await?;
                metrics::RETRIED_DELETE_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
            }
        }
        Err(err) => {
            logger::error!("Err: Deleting Card From Locker : {}", err);
            retry_delete_tokenize(db, &delete_tokenize_data.pm, tokenize_tracker.to_owned())
                .await?;
            metrics::RETRIED_DELETE_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        }
    }
    Ok(())
}

#[cfg(feature = "basilisk")]
pub async fn get_delete_tokenize_schedule_time(
    db: &dyn db::StorageInterface,
    pm: &enums::PaymentMethod,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let redis_mapping = db::get_and_deserialize_key(
        db,
        &format!("pt_mapping_delete_{pm}_tokenize_data"),
        "PaymentMethodsPTMapping",
    )
    .await;
    let mapping = match redis_mapping {
        Ok(x) => x,
        Err(err) => {
            logger::info!("Redis Mapping Error: {}", err);
            process_data::PaymentMethodsPTMapping::default()
        }
    };
    let time_delta = process_tracker_utils::get_pm_schedule_time(mapping, pm, retry_count + 1);

    process_tracker_utils::get_time_from_delta(time_delta)
}

#[cfg(feature = "basilisk")]
pub async fn retry_delete_tokenize(
    db: &dyn db::StorageInterface,
    pm: &enums::PaymentMethod,
    pt: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time = get_delete_tokenize_schedule_time(db, pm, pt.retry_count).await;

    match schedule_time {
        Some(s_time) => pt.retry(db, s_time).await,
        None => {
            pt.finish_with_status(db, "RETRIES_EXCEEDED".to_string())
                .await
        }
    }
}
