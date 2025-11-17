use std::fmt::Debug;

use crate::{
    metrics,
    configs::settings,
    controller,
    core::{errors,transformers},
    headers,
    helpers::domain,
    state,
};
use masking::ExposeOptionInterface;
use hyperswitch_domain_models::locker_mock_up;
use time::Duration;
use scheduler::workflows::storage as sch_storage;
use scheduler::errors::ProcessTrackerError;
use api_models::{
    enums as api_enums,
    payment_methods::{self as api, Card},
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
    consts, encryption,
    generate_id, id_type, type_name,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::ext_traits::OptionExt;
use hyperswitch_interfaces::api_client;
use josekit::jwe;
use masking::{PeekInterface, Secret};
use router_env::logger;
use router_env::{instrument, tracing, RequestId};

const PAYMENT_METHOD_STATUS_UPDATE_TASK: &str = "PAYMENT_METHOD_STATUS_UPDATE";
const PAYMENT_METHOD_STATUS_TAG: &str = "PAYMENT_METHOD_STATUS";

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct PaymentMethodStatusTrackingData {
    pub payment_method_id: String,
    pub prev_status: common_enums::PaymentMethodStatus,
    pub curr_status: common_enums::PaymentMethodStatus,
    pub merchant_id: id_type::MerchantId,
}

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

pub async fn get_card_from_locker(
    state: &state::PaymentMethodsState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &str,
) -> errors::PmResult<Card> {
    metrics::GET_FROM_LOCKER.add(1, &[]);

    let get_card_from_rs_locker_resp = common_utils::metrics::utils::record_operation_time(
        async {
            get_card_from_hs_locker(
                state,
                customer_id,
                merchant_id,
                card_reference,
                api_enums::LockerChoice::HyperswitchCardVault,
            )
            .await
            .map_err(|err| match err.current_context() {
                errors::VaultError::FetchCardFailed => {
                    err.change_context(errors::ApiErrorResponse::GenericNotFoundError {
                        message: "Card not found in vault".to_string(),
                    })
                }
                _ => err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error getting card from card vault"),
            })
            .inspect_err(|_| {
                metrics::CARD_LOCKER_FAILURES.add(
                    1,
                    router_env::metric_attributes!(("locker", "rust"), ("operation", "get")),
                );
            })
        },
        &metrics::CARD_GET_TIME,
        router_env::metric_attributes!(("locker", "rust")),
    )
    .await?;

    logger::debug!("card retrieved from rust locker");
    Ok(get_card_from_rs_locker_resp)
}

#[instrument(skip_all)]
pub async fn get_card_from_hs_locker<'a>(
    state: &'a state::PaymentMethodsState,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    card_reference: &'a str,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<Card, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    if !locker.mock_locker {
        let request = transformers::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            card_reference,
            Some(locker_choice),
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)
        .attach_printable("Making get card request failed")?;
        let get_card_resp = call_locker_api::<transformers::RetrieveCardResp>(
            state,
            request,
            "get_card_from_locker",
            Some(locker_choice),
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;

        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchCardFailed)?;
        retrieve_card_resp
            .card
            .get_required_value("Card")
            .change_context(errors::VaultError::FetchCardFailed)
    } else {
        let (get_card_resp, _) = mock_get_card(&*state.store, card_reference).await?;
        transformers::mk_get_card_response(get_card_resp)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
    }
}

#[instrument(skip_all)]
pub async fn mock_get_card<'a>(
    db: &dyn state::PaymentMethodsStorageInterface,
    card_id: &'a str,
) -> CustomResult<(transformers::GetCardResponse, Option<String>), errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    let add_card_response = transformers::AddCardResponse {
        card_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        card_fingerprint: locker_mock_up.card_fingerprint.into(),
        card_global_fingerprint: locker_mock_up.card_global_fingerprint.into(),
        merchant_id: Some(locker_mock_up.merchant_id),
        card_number: cards::CardNumber::try_from(locker_mock_up.card_number)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Invalid card number format from the mock locker")
            .map(Some)?,
        card_exp_year: Some(locker_mock_up.card_exp_year.into()),
        card_exp_month: Some(locker_mock_up.card_exp_month.into()),
        name_on_card: locker_mock_up.name_on_card.map(|card| card.into()),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
    };
    Ok((
        transformers::GetCardResponse {
            card: add_card_response,
        },
        locker_mock_up.card_cvc,
    ))
}

#[cfg(feature = "v1")]
pub async fn add_payment_method_status_update_task(
    db: &dyn state::PaymentMethodsStorageInterface,
    payment_method: &domain::PaymentMethod,
    prev_status: common_enums::PaymentMethodStatus,
    curr_status: common_enums::PaymentMethodStatus,
    merchant_id: &id_type::MerchantId,
) -> Result<(), ProcessTrackerError> {
    let created_at = payment_method.created_at;
    let schedule_time =
        created_at.saturating_add(Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

    let tracking_data = PaymentMethodStatusTrackingData {
        payment_method_id: payment_method.get_id().clone(),
        prev_status,
        curr_status,
        merchant_id: merchant_id.to_owned(),
    };

    let runner = sch_storage::ProcessTrackerRunner::PaymentMethodStatusUpdateWorkflow;
    let task = PAYMENT_METHOD_STATUS_UPDATE_TASK;
    let tag = [PAYMENT_METHOD_STATUS_TAG];

    let process_tracker_id = generate_task_id_for_payment_method_status_update_workflow(
        payment_method.get_id().as_str(),
        runner,
        task,
    );
    let process_tracker_entry = sch_storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct PAYMENT_METHOD_STATUS_UPDATE process tracker task")?;

    db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting PAYMENT_METHOD_STATUS_UPDATE reminder to process_tracker for payment_method_id: {}",
                payment_method.get_id().clone()
            )
        })?;

    Ok(())
}

fn generate_task_id_for_payment_method_status_update_workflow(
    key_id: &str,
    runner: sch_storage::ProcessTrackerRunner,
    task: &str,
) -> String {
    format!("{runner}_{task}_{key_id}")
}

#[instrument(skip_all)]
pub async fn add_card_to_hs_locker(
    state: &state::PaymentMethodsState,
    payload: &transformers::StoreLockerReq,
    customer_id: &id_type::CustomerId,
    locker_choice: api_enums::LockerChoice,
) -> CustomResult<transformers::StoreCardRespPayload, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let db = &*state.store;
    let stored_card_response = if !locker.mock_locker {
        let request = transformers::mk_add_locker_request_hs(
            jwekey,
            locker,
            payload,
            locker_choice,
            state.tenant.tenant_id.clone(),
            state.request_id.clone(),
        )
        .await?;
        call_locker_api::<transformers::StoreCardResp>(
            state,
            request,
            "add_card_to_hs_locker",
            Some(locker_choice),
        )
        .await
        .change_context(errors::VaultError::SaveCardFailed)?
    } else {
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_call_to_locker_hs(db, &card_id, payload, None, None, Some(customer_id)).await?
    };

    let stored_card = stored_card_response
        .payload
        .get_required_value("StoreCardRespPayload")
        .change_context(errors::VaultError::SaveCardFailed)?;
    Ok(stored_card)
}

///Mock api for local testing
pub async fn mock_call_to_locker_hs(
    db: &dyn state::PaymentMethodsStorageInterface,
    card_id: &str,
    payload: &transformers::StoreLockerReq,
    card_cvc: Option<String>,
    payment_method_id: Option<String>,
    customer_id: Option<&id_type::CustomerId>,
) -> CustomResult<transformers::StoreCardResp, errors::VaultError> {
    let mut locker_mock_up = locker_mock_up::LockerMockUpNew {
        card_id: card_id.to_string(),
        external_id: uuid::Uuid::new_v4().to_string(),
        card_fingerprint: uuid::Uuid::new_v4().to_string(),
        card_global_fingerprint: uuid::Uuid::new_v4().to_string(),
        merchant_id: id_type::MerchantId::default(),
        card_number: "4111111111111111".to_string(),
        card_exp_year: "2099".to_string(),
        card_exp_month: "12".to_string(),
        card_cvc,
        payment_method_id,
        customer_id: customer_id.map(ToOwned::to_owned),
        name_on_card: None,
        nickname: None,
        enc_card_data: None,
    };
    locker_mock_up = match payload {
        transformers::StoreLockerReq::LockerCard(store_card_req) => locker_mock_up::LockerMockUpNew {
            merchant_id: store_card_req.merchant_id.to_owned(),
            card_number: store_card_req.card.card_number.peek().to_string(),
            card_exp_year: store_card_req.card.card_exp_year.peek().to_string(),
            card_exp_month: store_card_req.card.card_exp_month.peek().to_string(),
            name_on_card: store_card_req.card.name_on_card.to_owned().expose_option(),
            nickname: store_card_req.card.nick_name.to_owned(),
            ..locker_mock_up
        },
        transformers::StoreLockerReq::LockerGeneric(store_generic_req) => {
            locker_mock_up::LockerMockUpNew {
                merchant_id: store_generic_req.merchant_id.to_owned(),
                enc_card_data: Some(store_generic_req.enc_data.to_owned()),
                ..locker_mock_up
            }
        }
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::VaultError::SaveCardFailed)?;
    let payload = transformers::StoreCardRespPayload {
        card_reference: response.card_id,
        duplication_check: None,
    };
    Ok(transformers::StoreCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
        payload: Some(payload),
    })
}
