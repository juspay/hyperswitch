use actix_web::web;
use api_models::verifications::{self, ApplepayMerchantResponse};
use common_utils::errors::CustomResult;
use error_stack::{Report, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;

use crate::{
    core::errors::{self, api_error_response, utils::StorageErrorExt},
    headers, logger,
    routes::AppState,
    services, types,
    types::storage,
    utils,
};

const APPLEPAY_INTERNAL_MERCHANT_NAME: &str = "Applepay_merchant";

pub async fn verify_merchant_creds_for_applepay(
    state: &AppState,
    _req: &actix_web::HttpRequest,
    body: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    kms_config: &kms::KmsConfig,
    merchant_id: String,
) -> CustomResult<
    services::ApplicationResponse<ApplepayMerchantResponse>,
    api_error_response::ApiErrorResponse,
> {
    let encrypted_merchant_identifier = &state
        .conf
        .applepay_merchant_configs
        .common_merchant_identifier;
    let encrypted_cert = &state.conf.applepay_merchant_configs.merchant_cert;
    let encrypted_key = &state.conf.applepay_merchant_configs.merchant_cert_key;
    let applepay_endpoint = &state.conf.applepay_merchant_configs.applepay_endpoint;
    let applepay_internal_merchant_identifier = kms::get_kms_client(kms_config)
        .await
        .decrypt(encrypted_merchant_identifier)
        .await
        .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;

    let cert_data = kms::get_kms_client(kms_config)
        .await
        .decrypt(encrypted_cert)
        .await
        .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;

    let key_data = kms::get_kms_client(kms_config)
        .await
        .decrypt(encrypted_key)
        .await
        .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;

    let request_body = verifications::ApplepayMerchantVerificationConfigs {
        domain_names: body.domain_names.clone(),
        encrypt_to: applepay_internal_merchant_identifier.to_string(),
        partner_internal_merchant_identifier: applepay_internal_merchant_identifier.to_string(),
        partner_merchant_name: APPLEPAY_INTERNAL_MERCHANT_NAME.to_string(),
    };

    let applepay_req = types::RequestBody::log_and_get_request_body(
        &request_body,
        utils::Encode::<verifications::ApplepayMerchantVerificationRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encode ApplePay session request to a string of json")?;

    let apple_pay_merch_verification_req = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(applepay_endpoint)
        .attach_default_headers()
        .headers(vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string().into(),
        )])
        .body(Some(applepay_req))
        .add_certificate(Some(cert_data))
        .add_certificate_key(Some(key_data))
        .build();

    let response = services::call_connector_api(state, apple_pay_merch_verification_req).await;
    log_applepay_verification_response_if_error(&response);

    let applepay_response =
        response.change_context(api_error_response::ApiErrorResponse::InternalServerError)?;

    // Error is already logged
    Ok(match applepay_response {
        Ok(_) => {
            check_existence_and_add_domain_to_db(
                state,
                merchant_id,
                body.merchant_connector_account_id.clone(),
                body.domain_names.clone(),
            )
            .await
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;
            services::api::ApplicationResponse::Json(ApplepayMerchantResponse {
                status_code: 200,
                status_message: "Applepay verification Completed".to_string(),
            })
        }
        Err(error) => {
            logger::error!(?error);
            services::api::ApplicationResponse::Json(ApplepayMerchantResponse {
                status_code: 200,
                status_message: "Applepay verification Failed".to_string(),
            })
        }
    })
}
async fn check_existence_and_add_domain_to_db(
    state: &AppState,
    merchant_id: String,
    merchant_connector_id: String,
    domain_from_req: Vec<String>,
) -> CustomResult<Vec<String>, errors::ApiErrorResponse> {
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_connector_account = state
        .store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut already_verified_domains = merchant_connector_account
        .applepay_verified_domains
        .clone()
        .unwrap_or_default();

    let mut new_verified_domains: Vec<String> = domain_from_req
        .into_iter()
        .filter(|req_domain| !already_verified_domains.contains(req_domain))
        .collect();

    already_verified_domains.append(&mut new_verified_domains);
    let updated_mca = storage::MerchantConnectorAccountUpdate::Update {
        merchant_id: None,
        connector_type: None,
        connector_name: None,
        connector_account_details: None,
        test_mode: None,
        disabled: None,
        merchant_connector_id: None,
        payment_methods_enabled: None,
        metadata: None,
        frm_configs: None,
        connector_webhook_details: None,
        applepay_verified_domains: Some(already_verified_domains.clone()),
    };
    state
        .store
        .update_merchant_connector_account(
            merchant_connector_account,
            updated_mca.into(),
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!("Failed while updating MerchantConnectorAccount: id: {merchant_connector_id}")
        })?;

    Ok(already_verified_domains.clone())
}

fn log_applepay_verification_response_if_error(
    response: &Result<Result<types::Response, types::Response>, Report<errors::ApiClientError>>,
) {
    if let Err(error) = response.as_ref() {
        logger::error!(?error);
    };
    response
        .as_ref()
        .ok()
        .map(|res| res.as_ref().map_err(|error| logger::error!(?error)));
}
