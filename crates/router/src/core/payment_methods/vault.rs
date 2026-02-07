use common_utils::{encryption::Encryption, request};
use common_utils::{
    ext_traits::{BytesExt, Encode},
    id_type, type_name,
};
use error_stack::{report, ResultExt};

use hyperswitch_domain_models::types::VaultRouterData;
use masking::PeekInterface;
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::errors::{self, CustomResult, RouterResult},
    routes::{self},
    services::{self},
    types::domain,
    utils::StringExt,
};

use crate::{
    core::payment_methods::{cards as pm_cards, transformers as pm_transforms},
    headers, settings,
    types::payment_methods as pm_types,
    utils::ext_traits::OptionExt,
};

#[cfg(feature = "v2")]
use crate::{
    core::{
        errors::StorageErrorExt,
        payment_methods::utils,
        payments::{self as payments_core},
    },
    utils::ConnectorResponseExt,
};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::router_flow_types::{
    ExternalVaultDeleteFlow, ExternalVaultRetrieveFlow,
};

mod external_vault;
mod internal_vault;
mod temp_locker;
mod transformers;

pub(crate) use temp_locker::process_tracker::*;
pub(crate) use temp_locker::*;

/// Re-export the strategy implementations from submodules
#[cfg(feature = "v2")]
use external_vault::ExternalVault;
#[cfg(feature = "v2")]
use internal_vault::InternalVault;

/// Vault strategy trait defining the interface for vault operations
///
/// This trait is implemented by different vault strategies (internal, external, mock)
/// to provide a unified interface for vault operations.
#[cfg(feature = "v2")]
#[async_trait::async_trait]
pub trait VaultStrategy {
    /// Vault a payment method
    async fn vault_payment_method(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        pmd: &domain::PaymentMethodVaultingData,
        existing_vault_id: Option<domain::VaultId>,
        customer_id: &id_type::GlobalCustomerId,
    ) -> RouterResult<(
        pm_types::AddVaultResponse,
        Option<id_type::MerchantConnectorAccountId>,
    )>;

    /// Retrieve a payment method from vault
    async fn retrieve_payment_method(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultRetrieveResponse>;

    /// Delete a payment method from vault
    async fn delete_payment_method(
        &self,
        state: &routes::SessionState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        pm: &domain::PaymentMethod,
    ) -> RouterResult<pm_types::VaultDeleteResponse>;
}

/// Select the appropriate vault strategy based on profile configuration.
/// Returns a trait object to eliminate repeated match statements.
#[cfg(feature = "v2")]
pub async fn select_vault_strategy(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResult<Box<dyn VaultStrategy + Send>> {
    if profile.is_external_vault_enabled() {
        let external_vault_source = profile
            .external_vault_connector_details
            .clone()
            .map(|details| details.vault_connector_id)
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("mca_id not present for external vault")?;

        let merchant_connector_account = payments_core::helpers::get_merchant_connector_account_v2(
            state,
            platform.get_processor(),
            Some(&external_vault_source),
        )
        .await
        .attach_printable("failed to fetch merchant connector account for vault")?;

        Ok(Box::new(ExternalVault::new(merchant_connector_account)))
    } else {
        Ok(Box::new(InternalVault))
    }
}

/// Facade function to vault a payment method.
///
/// This function uses the Strategy pattern to delegate vault operations
/// to the appropriate implementation (internal or external vault).
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &routes::SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    profile: &domain::Profile,
    existing_vault_id: Option<domain::VaultId>,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<(
    pm_types::AddVaultResponse,
    Option<id_type::MerchantConnectorAccountId>,
)> {
    let strategy = select_vault_strategy(state, platform, profile).await?;
    strategy
        .vault_payment_method(
            state,
            platform,
            profile,
            pmd,
            existing_vault_id,
            customer_id,
        )
        .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_from_vault(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    pm: &domain::PaymentMethod,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    let strategy = select_vault_strategy(state, platform, profile).await?;
    strategy
        .retrieve_payment_method(state, platform, profile, pm)
        .await
}

#[cfg(feature = "v2")]
pub async fn delete_payment_method_data_from_vault(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    pm: &domain::PaymentMethod,
) -> RouterResult<pm_types::VaultDeleteResponse> {
    let strategy = select_vault_strategy(state, platform, profile).await?;
    strategy
        .delete_payment_method(state, platform, profile, pm)
        .await
}

#[instrument(skip_all)]
pub async fn vault_payment_method_v1(
    state: &routes::SessionState,
    pmd: &hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    should_generate_multiple_tokens: Option<bool>,
) -> RouterResult<pm_types::AddVaultResponse> {
    external_vault::vault_payment_method_v1(
        state,
        pmd,
        merchant_account,
        merchant_connector_account,
        should_generate_multiple_tokens,
    )
    .await
}

#[cfg(feature = "v2")]
async fn create_vault_request<R: pm_types::VaultingInterface>(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    payload: Vec<u8>,
    tenant_id: id_type::TenantId,
) -> CustomResult<request::Request, errors::VaultError> {
    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = services::encryption::jws_sign_payload(
        &payload,
        &locker.locker_signing_key_id,
        private_key,
    )
    .await
    .change_context(errors::VaultError::RequestEncryptionFailed)?;

    let jwe_payload = transformers::create_jwe_body_for_vault(jwekey, &jws).await?;

    let mut url = locker.host.to_owned();
    url.push_str(R::get_vaulting_request_url());
    let mut request = request::Request::new(services::Method::Post, &url);
    request.add_header(
        headers::CONTENT_TYPE,
        consts::VAULT_HEADER_CONTENT_TYPE.into(),
    );
    request.add_header(
        headers::X_TENANT_ID,
        tenant_id.get_string_repr().to_owned().into(),
    );
    request.set_body(request::RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn call_to_vault<V: pm_types::VaultingInterface>(
    state: &routes::SessionState,
    payload: Vec<u8>,
) -> CustomResult<String, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();

    let request =
        create_vault_request::<V>(jwekey, locker, payload, state.tenant.tenant_id.to_owned())
            .await?;
    let response = services::call_connector_api(state, request, V::get_vaulting_flow_name())
        .await
        .change_context(errors::VaultError::VaultAPIError);

    let jwe_body: services::JweBody = response
        .get_response_inner("JweBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to get JweBody from vault response")?;

    let decrypted_payload = transformers::get_decrypted_vault_response_payload(
        jwekey,
        jwe_body,
        locker.decryption_scheme.clone(),
    )
    .await
    .change_context(errors::VaultError::ResponseDecryptionFailed)
    .attach_printable("Error getting decrypted vault response payload")?;

    Ok(decrypted_payload)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn get_fingerprint_id_from_vault<D: domain::VaultingDataInterface + serde::Serialize>(
    state: &routes::SessionState,
    data: &D,
    key: String,
) -> CustomResult<String, errors::VaultError> {
    internal_vault::get_fingerprint_id_from_vault(state, data, key).await
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn add_payment_method_to_internal_vault(
    state: &routes::SessionState,
    platform: &domain::Platform,
    pmd: &domain::PaymentMethodVaultingData,
    existing_vault_id: Option<domain::VaultId>,
    customer_id: &id_type::GlobalCustomerId,
) -> CustomResult<pm_types::AddVaultResponse, errors::VaultError> {
    internal_vault::add_payment_method_to_vault(state, pmd, existing_vault_id, customer_id).await
}

pub async fn call_vault_api<'a, Req, Res>(
    state: &routes::SessionState,
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    payload: &'a Req,
    endpoint_path: &str,
    tenant_id: id_type::TenantId,
    request_id: Option<router_env::RequestId>,
) -> CustomResult<Res, errors::VaultError>
where
    Req: Encode<'a> + serde::Serialize,
    Res: serde::de::DeserializeOwned,
{
    let encoded_payload = payload
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();
    let jws = services::encryption::jws_sign_payload(
        &encoded_payload,
        &locker.locker_signing_key_id,
        private_key,
    )
    .await
    .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload = transformers::mk_vault_req(jwekey, &jws).await?;

    let url = locker.get_host(endpoint_path);

    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(headers::X_TENANT_ID, tenant_id.get_string_repr().into());

    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }

    request.set_body(request::RequestContent::Json(Box::new(jwe_payload)));

    let response = call_vault_service::<Res>(state, request, endpoint_path)
        .await
        .change_context(errors::VaultError::VaultAPIError)?;

    Ok(response)
}

#[instrument(skip_all)]
pub async fn call_vault_service<T>(
    state: &routes::SessionState,
    request: request::Request,
    flow_name: &str,
) -> CustomResult<T, errors::VaultError>
where
    T: serde::de::DeserializeOwned,
{
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let response_type_name = common_utils::type_name!(T);

    let response = services::call_connector_api(state, request, flow_name)
        .await
        .change_context(errors::VaultError::ApiError)?;

    let is_locker_call_succeeded = response.is_ok();

    let jwe_body = response
        .unwrap_or_else(|err| err)
        .response
        .parse_struct::<services::JweBody>("JweBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed while parsing locker response into JweBody")?;

    let decrypted_payload = transformers::get_decrypted_response_payload(
        jwekey,
        jwe_body,
        locker.decryption_scheme.clone(),
    )
    .await
    .change_context(errors::VaultError::ResponseDeserializationFailed)
    .attach_printable("Failed while decrypting locker payload response")?;

    // Irrespective of locker's response status, payload is JWE + JWS decrypted. But based on locker's status,
    // if Ok, deserialize the decrypted payload into given type T
    // if Err, raise an error including locker error message too
    if is_locker_call_succeeded {
        let stored_card_resp: Result<T, error_stack::Report<errors::VaultError>> =
            decrypted_payload
                .parse_struct(response_type_name)
                .change_context(errors::VaultError::ResponseDeserializationFailed)
                .attach_printable_lazy(|| {
                    format!("Failed while parsing locker response into {response_type_name}")
                });
        stored_card_resp
    } else {
        Err::<T, error_stack::Report<errors::VaultError>>((errors::VaultError::ApiError).into())
            .attach_printable_lazy(|| format!("Locker error response: {decrypted_payload:?}"))
    }
}

#[instrument(skip_all)]
pub async fn get_encrypted_data_from_vault<'a>(
    state: &'a routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payment_method_reference: &'a str,
) -> CustomResult<masking::Secret<String>, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();

    let payment_method_data = if !locker.mock_locker {
        let payload = pm_transforms::CardReqBody {
            merchant_id: merchant_id.to_owned(),
            merchant_customer_id: customer_id.to_owned(),
            card_reference: payment_method_reference.to_string(),
        };

        let get_card_resp: pm_transforms::RetrieveCardResp = call_vault_api(
            state,
            jwekey,
            locker,
            &payload,
            consts::LOCKER_RETRIEVE_CARD_PATH,
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
        )
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)
        .attach_printable("Making get payment method request failed")?;

        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed to retrieve field - payload from RetrieveCardResp")?;
        let enc_card_data = retrieve_card_resp
            .enc_card_data
            .get_required_value("enc_card_data")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable(
                "Failed to retrieve field - enc_card_data from RetrieveCardRespPayload",
            )?;
        decode_and_decrypt_locker_data(state, key_store, enc_card_data.peek().to_string()).await?
    } else {
        pm_cards::mock_get_payment_method(state, key_store, payment_method_reference)
            .await?
            .payment_method
            .payment_method_data
    };
    Ok(payment_method_data)
}

#[instrument(skip_all)]
pub async fn decode_and_decrypt_locker_data(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    enc_card_data: String,
) -> CustomResult<masking::Secret<String>, errors::VaultError> {
    let key = key_store.key.get_inner().peek();
    let decoded_bytes = hex::decode(&enc_card_data)
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode hex string into bytes")?;
    // Decrypt
    domain::types::crypto_operation(
        &state.into(),
        type_name!(diesel_models::PaymentMethod),
        domain::types::CryptoOperation::DecryptOptional(Some(Encryption::new(
            decoded_bytes.into(),
        ))),
        common_utils::types::keymanager::Identifier::Merchant(key_store.merchant_id.clone()),
        key,
    )
    .await
    .and_then(|val| val.try_into_optionaloperation())
    .change_context(errors::VaultError::FetchPaymentMethodFailed)?
    .map_or(
        Err(report!(errors::VaultError::FetchPaymentMethodFailed)),
        |d| Ok(d.into_inner()),
    )
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_from_internal_vault(
    state: &routes::SessionState,
    request: pm_types::VaultRetrieveRequest,
) -> CustomResult<pm_types::VaultRetrieveResponse, errors::VaultError> {
    internal_vault::retrieve_payment_method_from_vault(state, request).await
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_value_from_internal_vault(
    state: &routes::SessionState,
    request: pm_types::VaultRetrieveRequest,
) -> CustomResult<serde_json::Value, errors::VaultError> {
    internal_vault::retrieve_value_from_vault(state, request).await
}

#[cfg(feature = "v2")]
pub fn get_vault_response_for_retrieve_payment_method_data<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    external_vault::get_vault_response_for_retrieve_payment_method_data(router_data)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_from_vault_using_payment_token(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_token: &String,
    payment_method_type: &common_enums::PaymentMethod,
) -> RouterResult<(domain::PaymentMethod, domain::PaymentMethodVaultingData)> {
    let pm_token_data =
        utils::retrieve_payment_token_data(state, payment_token.to_string()).await?;

    let payment_method_id = match pm_token_data {
        storage::PaymentTokenData::PermanentCard(card_token_data) => {
            card_token_data.payment_method_id
        }
        storage::PaymentTokenData::TemporaryGeneric(_) => {
            Err(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "TemporaryGeneric Token not implemented".to_string(),
                ),
            })?
        }
        storage::PaymentTokenData::AuthBankDebit(_) => {
            Err(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "AuthBankDebit Token not implemented".to_string(),
                ),
            })?
        }
    };
    let db = &*state.store;

    let storage_scheme = platform.get_processor().get_account().storage_scheme;

    let payment_method = db
        .find_payment_method(
            platform.get_processor().get_key_store(),
            &payment_method_id,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

    let vault_data = retrieve_payment_method_from_vault(state, platform, profile, &payment_method)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve payment method from vault")?
        .data;

    Ok((payment_method, vault_data))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemporaryVaultCvc {
    card_cvc: masking::Secret<String>,
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn insert_cvc_using_payment_token(
    state: &routes::SessionState,
    payment_method_id: &id_type::GlobalPaymentMethodId,
    card_cvc: masking::Secret<String>,
    fulfillment_time: i64,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<api_models::payment_methods::CardCVCTokenStorageDetails> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let key = format!(
        "pm_token_{}_hyperswitch_cvc",
        payment_method_id.get_string_repr()
    );

    let payload_to_be_encrypted = TemporaryVaultCvc { card_cvc };

    // Encrypt the CVC and store it in Redis
    let encrypted_payload: Encryption = pm_cards::create_encrypted_data(
        &(state.into()),
        key_store,
        payload_to_be_encrypted.clone(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to encrypt TemporaryVaultCvc for vault")?
    .into();

    redis_conn
        .serialize_and_set_key_with_expiry(
            &key.as_str().into(),
            encrypted_payload,
            fulfillment_time,
        )
        .await
        .map_err(Into::<errors::StorageError>::into)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add encrypted cvc to redis")?;

    let card_token_cvc_storage =
        api_models::payment_methods::CardCVCTokenStorageDetails::generate_expiry_timestamp(
            fulfillment_time,
        );

    Ok(card_token_cvc_storage)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_and_delete_cvc_from_payment_token(
    state: &routes::SessionState,
    payment_method_id: &String,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<masking::Secret<String>> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let key = format!("pm_token_{payment_method_id}_hyperswitch_cvc");

    let resp: Encryption = redis_conn
        .get_and_deserialize_key::<Encryption>(&key.clone().into(), "Vec<u8>")
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let cvc_data: TemporaryVaultCvc = pm_cards::decrypt_generic_data(state, Some(resp), key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to decrypt volatile payment method vault data")?
        .get_required_value("PaymentMethodVaultingData")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get required decrypted volatile payment method vault data")?;

    logger::info!(
        "CVC retrieved successfully from redis for payment method id: {}",
        payment_method_id
    );

    // delete key after retrieving the cvc
    redis_conn.delete_key(&key.into()).await.map_err(|err| {
        logger::error!("Failed to delete token from redis: {:?}", err);
    });

    Ok(cvc_data.card_cvc)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_key_and_ttl_for_cvc_from_payment_method_id(
    state: &routes::SessionState,
    payment_method_id: id_type::GlobalPaymentMethodId,
) -> RouterResult<i64> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let key = format!(
        "pm_token_{}_hyperswitch_cvc",
        payment_method_id.get_string_repr()
    );

    // check if key exists and get ttl
    redis_conn
        .get_key::<bytes::Bytes>(&key.clone().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch the payment_method_token from redis")?;

    let ttl = redis_conn
        .get_ttl(&key.clone().into())
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch the payment_method_token ttl from redis")?;

    Ok(ttl)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn delete_payment_token(
    state: &routes::SessionState,
    key_for_token: &str,
    intent_status: enums::IntentStatus,
) -> RouterResult<()> {
    if ![
        enums::IntentStatus::RequiresCustomerAction,
        enums::IntentStatus::RequiresMerchantAction,
    ]
    .contains(&intent_status)
    {
        utils::delete_payment_token_data(state, key_for_token)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to delete payment_token")?;
    }
    Ok(())
}

#[cfg(feature = "v2")]
pub async fn retrieve_payment_method_data_from_storage(
    state: &routes::SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    pm: &domain::PaymentMethod,
    storage_type: enums::StorageType,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    let mut payment_method_data = match storage_type {
        enums::StorageType::Persistent => {
            retrieve_payment_method_from_vault(state, platform, profile, pm).await?
        }
        enums::StorageType::Volatile => {
            retrieve_volatile_payment_method_from_redis(
                state,
                platform.get_provider().get_key_store(),
                pm,
            )
            .await?
        }
    };

    let card_cvc = retrieve_and_delete_cvc_from_payment_token(
        state,
        &pm.id.get_string_repr().to_string(),
        platform.get_processor().get_key_store(),
    )
    .await
    .inspect_err(|err| {
        logger::warn!(
            "Failed to retrieve CVC for payment method {}",
            pm.id.get_string_repr()
        );
    });

    if let Ok(card_cvc) = card_cvc {
        payment_method_data.data.set_card_cvc(card_cvc);
    }

    Ok(payment_method_data)
}

#[cfg(feature = "v2")]
async fn retrieve_volatile_payment_method_from_redis(
    state: &routes::SessionState,
    key_store: &domain::MerchantKeyStore,
    pm: &domain::PaymentMethod,
) -> RouterResult<pm_types::VaultRetrieveResponse> {
    let vault_id = pm
        .locker_id
        .clone()
        .ok_or(errors::VaultError::MissingRequiredField {
            field_name: "locker_id",
        })
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Missing locker_id for VaultRetrieveRequest")?;

    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let response = redis_conn
        .get_and_deserialize_key::<Encryption>(&vault_id.get_string_repr().into(), "Vec<u8>")
        .await;

    match response {
        Ok(resp) => {
            let decrypted_payload: domain::PaymentMethodVaultingData =
                pm_cards::decrypt_generic_data(state, Some(resp), key_store)
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to decrypt volatile payment method vault data")?
                    .get_required_value("PaymentMethodVaultingData")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to get required decrypted volatile payment method vault data",
                    )?;

            Ok(pm_types::VaultRetrieveResponse {
                data: decrypted_payload,
            })
        }
        Err(err) => Err(err).change_context(errors::ApiErrorResponse::UnprocessableEntity {
            message: "Token is invalid or expired".into(),
        }),
    }
}

#[cfg(feature = "v2")]
pub async fn delete_payment_method_data_from_vault_internal(
    state: &routes::SessionState,
    vault_id: domain::VaultId,
    customer_id: &id_type::GlobalCustomerId,
) -> CustomResult<pm_types::VaultDeleteResponse, errors::VaultError> {
    internal_vault::delete_payment_method(state, vault_id, customer_id).await
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_from_vault_external_v1(
    state: &routes::SessionState,
    merchant_id: &id_type::MerchantId,
    pm: &domain::PaymentMethod,
    merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodVaultingData> {
    external_vault::retrieve_payment_method_v1(state, merchant_id, pm, merchant_connector_account)
        .await
}

pub fn get_vault_response_for_retrieve_payment_method_data_v1<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodVaultingData> {
    external_vault::get_vault_response_for_retrieve_payment_method_data_v1(router_data)
}
