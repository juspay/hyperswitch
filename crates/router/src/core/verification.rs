pub mod utils;
use api_models::verifications::{self, ApplepayMerchantResponse};
use common_utils::{errors::CustomResult, request::RequestContent};
use error_stack::ResultExt;
use masking::ExposeInterface;

use crate::{core::errors, headers, logger, routes::SessionState, services};

const APPLEPAY_INTERNAL_MERCHANT_NAME: &str = "Applepay_merchant";

pub async fn verify_merchant_creds_for_applepay(
    state: SessionState,
    body: verifications::ApplepayMerchantVerificationRequest,
    merchant_id: common_utils::id_type::MerchantId,
    profile_id: Option<common_utils::id_type::ProfileId>,
) -> CustomResult<services::ApplicationResponse<ApplepayMerchantResponse>, errors::ApiErrorResponse>
{
    let applepay_merchant_configs = state.conf.applepay_merchant_configs.get_inner();

    let applepay_internal_merchant_identifier = applepay_merchant_configs
        .common_merchant_identifier
        .clone()
        .expose();
    let cert_data = applepay_merchant_configs.merchant_cert.clone();
    let key_data = applepay_merchant_configs.merchant_cert_key.clone();
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
        .add_certificate(Some(cert_data))
        .add_certificate_key(Some(key_data))
        .build();

    let response = services::call_connector_api(
        &state,
        apple_pay_merch_verification_req,
        "verify_merchant_creds_for_applepay",
    )
    .await;
    utils::log_applepay_verification_response_if_error(&response);

    let applepay_response =
        response.change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Error is already logged
    match applepay_response {
        Ok(_) => {
            utils::check_existence_and_add_domain_to_db(
                &state,
                merchant_id,
                profile_id,
                body.merchant_connector_account_id.clone(),
                body.domain_names.clone(),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            Ok(services::api::ApplicationResponse::Json(
                ApplepayMerchantResponse {
                    status_message: "Applepay verification Completed".to_string(),
                },
            ))
        }
        Err(error) => {
            logger::error!(?error);
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Applepay verification Failed".to_string(),
            }
            .into())
        }
    }
}

pub async fn get_verified_apple_domains_with_mid_mca_id(
    state: SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
) -> CustomResult<
    services::ApplicationResponse<verifications::ApplepayVerifiedDomainsResponse>,
    errors::ApiErrorResponse,
> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &merchant_id,
            &db.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    #[cfg(feature = "v1")]
    let verified_domains = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            key_manager_state,
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::ResourceIdNotFound)?
        .applepay_verified_domains
        .unwrap_or_default();

    #[cfg(feature = "v2")]
    let verified_domains = {
        let _ = merchant_connector_id;
        let _ = key_store;
        todo!()
    };

    Ok(services::api::ApplicationResponse::Json(
        verifications::ApplepayVerifiedDomainsResponse { verified_domains },
    ))
}
