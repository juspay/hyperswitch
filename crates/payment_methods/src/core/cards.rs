use std::fmt::Debug;

use crate::{
    configs::settings,
    controller,
    core::errors,
    headers,
    helpers::{self, domain, StorageErrorExt},
    state,
};
#[cfg(feature = "payouts")]
use api_models::payouts;
use api_models::{
    enums as api_enums,
    payment_methods::{self as api, Card, CardDetailsPaymentMethod, PaymentMethodsData},
};
#[cfg(feature = "v1")]
use common_enums::enums as common_enums;
#[cfg(feature = "v2")]
use common_utils::encryption;
use common_utils::errors::CustomResult;
use common_utils::ext_traits::BytesExt;
use common_utils::ext_traits::Encode;
use common_utils::ext_traits::StringExt;
use common_utils::request::Method;
use common_utils::request::Request;
use common_utils::request::RequestContent;
use common_utils::{
    consts, crypto, encryption,
    ext_traits::{self, AsyncExt},
    generate_id, id_type, type_name,
    types::keymanager,
};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::payment_methods::PaymentMethodVaultSourceDetails;
use hyperswitch_domain_models::{
    api as domain_api, customer::CustomerUpdate, ext_traits::OptionExt, merchant_context,
    merchant_key_store, payment_methods, type_encryption,
};
use hyperswitch_interfaces::api_client;
use josekit::jwe;
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::logger;
use router_env::{instrument, tracing, RequestId};
#[cfg(feature = "v1")]
use scheduler::errors as sch_errors;
use serde::{Deserialize, Serialize};
use storage_impl::{errors as storage_errors, payment_method};

#[instrument(skip_all)]
pub async fn delete_card_from_hs_locker<'a>(
    state: &state::PaymentMethodsState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &'a str,
) -> CustomResult<controller::DeleteCardResp, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    let request = mk_delete_card_request_hs(
        jwekey,
        locker,
        customer_id,
        merchant_id,
        card_reference,
        state.tenant.tenant_id.clone(),
        state.request_id.clone(),
    )
    .await
    .change_context(errors::VaultError::DeleteCardFailed)
    .attach_printable("Making delete card request failed")?;

    if !locker.mock_locker {
        call_locker_api::<controller::DeleteCardResp>(
            state,
            request,
            "delete_card_from_locker",
            Some(api_enums::LockerChoice::HyperswitchCardVault),
        )
        .await
        .change_context(errors::VaultError::DeleteCardFailed)
    } else {
        Ok(mock_delete_card_hs(&*state.store, card_reference)
            .await
            .change_context(errors::VaultError::DeleteCardFailed)?)
    }
}

#[instrument(skip_all)]
pub async fn call_locker_api<T>(
    state: &state::PaymentMethodsState,
    request: Request,
    flow_name: &str,
    locker_choice: Option<api_enums::LockerChoice>,
) -> CustomResult<T, errors::VaultError>
where
    T: serde::de::DeserializeOwned,
{
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let response_type_name = type_name!(T);

    let response = api_client::call_connector_api(state, request, flow_name)
        .await
        .change_context(errors::VaultError::ApiError)?;

    let is_locker_call_succeeded = response.is_ok();

    let jwe_body = response
        .unwrap_or_else(|err| err)
        .response
        .parse_struct::<encryption::JweBody>("JweBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed while parsing locker response into JweBody")?;

    let decrypted_payload = get_decrypted_response_payload(
        jwekey,
        jwe_body,
        locker_choice,
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
pub async fn mock_delete_card_hs<'a>(
    db: &dyn state::PaymentMethodsStorageInterface,
    card_id: &'a str,
) -> CustomResult<controller::DeleteCardResp, errors::VaultError> {
    db.delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(controller::DeleteCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
    })
}
#[instrument(skip_all)]
pub async fn mock_delete_card<'a>(
    db: &dyn state::PaymentMethodsStorageInterface,
    card_id: &'a str,
) -> CustomResult<controller::DeleteCardResponse, errors::VaultError> {
    let locker_mock_up = db
        .delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(controller::DeleteCardResponse {
        card_id: Some(locker_mock_up.card_id),
        external_id: Some(locker_mock_up.external_id),
        card_isin: None,
        status: "Ok".to_string(),
    })
}
pub async fn mk_delete_card_request_hs(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<Request, errors::VaultError> {
    let merchant_customer_id = customer_id.to_owned();
    let card_req_body = settings::CardReqBody {
        merchant_id: merchant_id.to_owned(),
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = card_req_body
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload =
        mk_basilisk_req(jwekey, &jws, api_enums::LockerChoice::HyperswitchCardVault).await?;

    let mut url = locker.host.to_owned();
    url.push_str("/cards/delete");
    let mut request = Request::new(Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::X_TENANT_ID,
        tenant_id.get_string_repr().to_owned().into(),
    );
    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }

    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

// Need to fix this once we start moving to v2 completion
#[cfg(feature = "v2")]
pub async fn mk_delete_card_request_hs_by_id(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    id: &String,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
    tenant_id: id_type::TenantId,
    request_id: Option<RequestId>,
) -> CustomResult<services::Request, errors::VaultError> {
    let merchant_customer_id = id.to_owned();
    let card_req_body = CardReqBodyV2 {
        merchant_id: merchant_id.to_owned(),
        merchant_customer_id,
        card_reference: card_reference.to_owned(),
    };
    let payload = card_req_body
        .encode_to_vec()
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncodingFailed)?;

    let jwe_payload =
        mk_basilisk_req(jwekey, &jws, api_enums::LockerChoice::HyperswitchCardVault).await?;

    let mut url = locker.host.to_owned();
    url.push_str("/cards/delete");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/json".into());
    request.add_header(
        headers::X_TENANT_ID,
        tenant_id.get_string_repr().to_owned().into(),
    );
    if let Some(req_id) = request_id {
        request.add_header(headers::X_REQUEST_ID, req_id.to_string().into());
    }

    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}

pub async fn get_decrypted_response_payload(
    jwekey: &settings::Jwekey,
    jwe_body: encryption::JweBody,
    locker_choice: Option<api_enums::LockerChoice>,
    decryption_scheme: settings::DecryptionScheme,
) -> CustomResult<String, errors::VaultError> {
    let target_locker = locker_choice.unwrap_or(api_enums::LockerChoice::HyperswitchCardVault);

    let public_key = match target_locker {
        api_enums::LockerChoice::HyperswitchCardVault => {
            jwekey.vault_encryption_key.peek().as_bytes()
        }
    };

    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jwt = encryption::get_dotted_jwe(jwe_body);
    let alg = match decryption_scheme {
        settings::DecryptionScheme::RsaOaep => jwe::RSA_OAEP,
        settings::DecryptionScheme::RsaOaep256 => jwe::RSA_OAEP_256,
    };

    let jwe_decrypted = encryption::decrypt_jwe(
        &jwt,
        encryption::KeyIdCheck::SkipKeyIdCheck,
        private_key,
        alg,
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Jwe Decryption failed for JweBody for vault")?;

    let jws = jwe_decrypted
        .parse_struct("JwsBody")
        .change_context(errors::VaultError::ResponseDeserializationFailed)?;
    let jws_body = encryption::get_dotted_jws(jws);

    encryption::verify_sign(jws_body, public_key)
        .change_context(errors::VaultError::SaveCardFailed)
        .attach_printable("Jws Decryption failed for JwsBody for vault")
}

pub async fn mk_basilisk_req(
    jwekey: &settings::Jwekey,
    jws: &str,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<encryption::JweBody, errors::VaultError> {
    let jws_payload: Vec<&str> = jws.split('.').collect();

    let generate_jws_body = |payload: Vec<&str>| -> Option<encryption::JwsBody> {
        Some(encryption::JwsBody {
            header: payload.first()?.to_string(),
            payload: payload.get(1)?.to_string(),
            signature: payload.get(2)?.to_string(),
        })
    };

    let jws_body = generate_jws_body(jws_payload).ok_or(errors::VaultError::SaveCardFailed)?;

    let payload = jws_body
        .encode_to_vec()
        .change_context(errors::VaultError::SaveCardFailed)?;

    let public_key = match locker_choice {
        api_enums::LockerChoice::HyperswitchCardVault => {
            jwekey.vault_encryption_key.peek().as_bytes()
        }
    };

    let jwe_encrypted = encryption::encrypt_jwe(
        &payload,
        public_key,
        encryption::EncryptionAlgorithm::A256GCM,
        None,
    )
    .await
    .change_context(errors::VaultError::SaveCardFailed)
    .attach_printable("Error on jwe encrypt")?;
    let jwe_payload: Vec<&str> = jwe_encrypted.split('.').collect();

    let generate_jwe_body = |payload: Vec<&str>| -> Option<encryption::JweBody> {
        Some(encryption::JweBody {
            header: payload.first()?.to_string(),
            iv: payload.get(2)?.to_string(),
            encrypted_payload: payload.get(3)?.to_string(),
            tag: payload.get(4)?.to_string(),
            encrypted_key: payload.get(1)?.to_string(),
        })
    };

    let jwe_body = generate_jwe_body(jwe_payload).ok_or(errors::VaultError::SaveCardFailed)?;

    Ok(jwe_body)
}

#[cfg(feature = "v1")]
pub fn get_card_detail(
    pm: &domain::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let last4_digits = card_number.clone().get_last4();
    //fetch form card bin

    let card_detail = api::CardDetailFromLocker {
        scheme: pm.scheme.to_owned(),
        issuer_country: pm.issuer_country.clone(),
        last4_digits: Some(last4_digits),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_token: None,
        card_fingerprint: None,
        card_holder_name: response.name_on_card,
        nick_name: response.nick_name.map(Secret::new),
        card_isin: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    Ok(card_detail)
}

#[cfg(feature = "v2")]
pub fn get_card_detail(
    _pm: &domain::PaymentMethod,
    response: Card,
) -> CustomResult<api::CardDetailFromLocker, errors::VaultError> {
    let card_number = response.card_number;
    let last4_digits = card_number.clone().get_last4();
    //fetch form card bin

    let card_detail = api::CardDetailFromLocker {
        issuer_country: None,
        last4_digits: Some(last4_digits),
        card_number: Some(card_number),
        expiry_month: Some(response.card_exp_month),
        expiry_year: Some(response.card_exp_year),
        card_fingerprint: None,
        card_holder_name: response.name_on_card,
        nick_name: response.nick_name.map(Secret::new),
        card_isin: None,
        card_issuer: None,
        card_network: None,
        card_type: None,
        saved_to_locker: true,
    };
    Ok(card_detail)
}
