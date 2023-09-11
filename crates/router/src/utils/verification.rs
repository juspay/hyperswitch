use actix_web::web;
#[cfg(all(feature = "olap", feature = "kms"))]
use api_models::verifications::{self, ApplepayMerchantResponse};
use common_utils::errors::CustomResult;
use diesel_models::business_profile::{BusinessProfile, BusinessProfileUpdateInternal};
use error_stack::{Report, ResultExt};
#[cfg(feature = "kms")]
use external_services::kms;
use masking::Secret;

use crate::{
    core::errors::{self, api_error_response},
    headers, logger,
    routes::AppState,
    services, types, utils,
};

const APPLEPAY_INTERNAL_MERCHANT_NAME: &str = "Applepay_merchant";

pub async fn verify_merchant_creds_for_applepay(
    state: &AppState,
    _req: &actix_web::HttpRequest,
    body: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    kms_config: &kms::KmsConfig,
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

    let cert_data = Secret::new(
        kms::get_kms_client(kms_config)
            .await
            .decrypt(encrypted_cert)
            .await
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)?,
    );

    let key_data = Secret::new(
        kms::get_kms_client(kms_config)
            .await
            .decrypt(encrypted_key)
            .await
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)?,
    );

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
                body.business_profile_id.clone(),
                body.domain_names.clone(),
            )
            .await
            .change_context(api_error_response::ApiErrorResponse::InternalServerError)?;
            services::api::ApplicationResponse::Json(ApplepayMerchantResponse {
                status_code: "200".to_string(),
                status_message: "Applepay verification Completed".to_string(),
            })
        }
        Err(error) => {
            logger::error!(?error);
            services::api::ApplicationResponse::Json(ApplepayMerchantResponse {
                status_code: "200".to_string(),
                status_message: "Applepay verification Failed".to_string(),
            })
        }
    })
}

// Checks whether or not the domain verified is already present in db if not adds it
async fn check_existence_and_add_domain_to_db(
    state: &AppState,
    business_profile_id: String,
    domain_from_req: Vec<String>,
) -> CustomResult<BusinessProfile, errors::StorageError> {
    let business_profile = state
        .store
        .find_business_profile_by_profile_id(&business_profile_id)
        .await?;
    let business_profile_to_update = business_profile.clone();
    let mut already_verified_domains = business_profile
        .applepay_verified_domains
        .unwrap_or_default();

    let mut new_verified_domains: Vec<String> = domain_from_req
        .into_iter()
        .filter(|req_domain| !already_verified_domains.contains(req_domain))
        .collect();

    already_verified_domains.append(&mut new_verified_domains);

    let update_business_profile = BusinessProfileUpdateInternal {
        applepay_verified_domains: Some(already_verified_domains),
        profile_name: Some(business_profile.profile_name),
        modified_at: Some(business_profile.modified_at),
        return_url: business_profile.return_url,
        enable_payment_response_hash: Some(business_profile.enable_payment_response_hash),
        payment_response_hash_key: business_profile.payment_response_hash_key,
        redirect_to_merchant_with_http_post: Some(
            business_profile.redirect_to_merchant_with_http_post,
        ),
        webhook_details: business_profile.webhook_details,
        metadata: business_profile.metadata,
        routing_algorithm: business_profile.routing_algorithm,
        intent_fulfillment_time: business_profile.intent_fulfillment_time,
        frm_routing_algorithm: business_profile.frm_routing_algorithm,
        payout_routing_algorithm: business_profile.payout_routing_algorithm,
        is_recon_enabled: Some(business_profile.is_recon_enabled),
    };

    state
        .store
        .update_business_profile_by_profile_id(business_profile_to_update, update_business_profile)
        .await
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
