pub mod utils;
use actix_web::web;
use api_models::verifications::{self, ApplepayMerchantResponse};
use common_utils::{errors::CustomResult, ext_traits::Encode};
use error_stack::ResultExt;
#[cfg(feature = "kms")]
use external_services::kms;

use crate::{
    core::errors::{self, api_error_response},
    headers, logger,
    routes::AppState,
    services, types,
};

const APPLEPAY_INTERNAL_MERCHANT_NAME: &str = "Applepay_merchant";

pub async fn verify_merchant_creds_for_applepay(
    state: AppState,
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
        Encode::<verifications::ApplepayMerchantVerificationRequest>::encode_to_string_of_json,
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

    let response = services::call_connector_api(&state, apple_pay_merch_verification_req).await;
    utils::log_applepay_verification_response_if_error(&response);

    let applepay_response =
        response.change_context(api_error_response::ApiErrorResponse::InternalServerError)?;

    // Error is already logged
    match applepay_response {
        Ok(_) => {
            utils::check_existence_and_add_domain_to_db(
                &state,
                merchant_id,
                body.merchant_connector_account_id.clone(),
                body.domain_names.clone(),
            )
            .await
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;
            Ok(services::api::ApplicationResponse::Json(
                ApplepayMerchantResponse {
                    status_message: "Applepay verification Completed".to_string(),
                },
            ))
        }
        Err(error) => {
            logger::error!(?error);
            Err(api_error_response::ApiErrorResponse::InvalidRequestData {
                message: "Applepay verification Failed".to_string(),
            }
            .into())
        }
    }
}

pub async fn get_verified_apple_domains_with_mid_mca_id(
    state: AppState,
    merchant_id: String,
    merchant_connector_id: String,
) -> CustomResult<
    services::ApplicationResponse<api_models::verifications::ApplepayVerifiedDomainsResponse>,
    api_error_response::ApiErrorResponse,
> {
    let db = state.store.as_ref();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .change_context(api_error_response::ApiErrorResponse::MerchantAccountNotFound)?;

    let verified_domains = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id.as_str(),
            merchant_connector_id.as_str(),
            &key_store,
        )
        .await
        .change_context(api_error_response::ApiErrorResponse::ResourceIdNotFound)?
        .applepay_verified_domains
        .unwrap_or_default();

    Ok(services::api::ApplicationResponse::Json(
        api_models::verifications::ApplepayVerifiedDomainsResponse { verified_domains },
    ))
}
