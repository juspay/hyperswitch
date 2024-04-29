pub mod utils;
use api_models::verifications::{self, ApplepayMerchantResponse};
use common_utils::{errors::CustomResult, request::RequestContent};
use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{core::errors::api_error_response, headers, logger, pii, routes::AppState, services};

const APPLEPAY_INTERNAL_MERCHANT_NAME: &str = "Applepay_merchant";

pub async fn verify_merchant_creds_for_applepay(
    state: AppState,
    body: verifications::ApplepayMerchantVerificationRequest,
    merchant_id: String,
) -> CustomResult<
    services::ApplicationResponse<ApplepayMerchantResponse>,
    api_error_response::ApiErrorResponse,
> {
    let applepay_merchant_configs = state.conf.applepay_merchant_configs.get_inner();

    let applepay_internal_merchant_identifier = applepay_merchant_configs
        .common_merchant_identifier
        .clone()
        .expose();
    let cert_data = applepay_merchant_configs.merchant_cert.clone().expose();
    let key_data = applepay_merchant_configs.merchant_cert_key.clone().expose();
    let applepay_endpoint = &applepay_merchant_configs.applepay_endpoint;

    let request_body = verifications::ApplepayMerchantVerificationConfigs {
        domain_names: body.domain_names.clone(),
        encrypt_to: applepay_internal_merchant_identifier.clone(),
        partner_internal_merchant_identifier: applepay_internal_merchant_identifier,
        partner_merchant_name: APPLEPAY_INTERNAL_MERCHANT_NAME.to_string(),
    };

    let apple_pay_merch_verification_req = services::RequestBuilder::new()
        .method(services::Method::Post)
        .url(applepay_endpoint)
        .attach_default_headers()
        .headers(vec![(
            headers::CONTENT_TYPE.to_string(),
            "application/json".to_string().into(),
        )])
        .set_body(RequestContent::Json(Box::new(request_body)))
        .add_certificate(Some(pii::Secret::new(cert_data)))
        .add_certificate_key(Some(pii::Secret::new(key_data)))
        .build();

    let response = services::call_connector_api(
        &state,
        apple_pay_merch_verification_req,
        "verify_merchant_creds_for_applepay",
    )
    .await;
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
