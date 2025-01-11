pub mod transformers;
pub mod types;
pub mod utils;

use api_models::payments::CtpServiceDetails;
use diesel_models::authentication::{Authentication, AuthenticationNew};
use error_stack::ResultExt;
use hyperswitch_connectors::connectors::unified_authentication_service::transformers::WebhookResponse;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_request_types::unified_authentication_service::{
        UasAuthenticationResponseData, UasConfirmationRequestData,
        UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
    },
};
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;
use masking::ExposeInterface;

use super::errors::RouterResult;
use crate::{
    core::{
        errors::utils::StorageErrorExt,
        payments::PaymentData,
        unified_authentication_service::types::{
            ClickToPay, UnifiedAuthenticationService, UNIFIED_AUTHENTICATION_SERVICE,
        },
    },
    db::domain,
    routes::SessionState,
    types::domain::MerchantConnectorAccount,
};

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl<F: Clone + Sync> UnifiedAuthenticationService<F> for ClickToPay {
    async fn pre_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccount,
        connector_name: &str,
        authentication_id: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<()> {
        let pre_authentication_data =
            UasPreAuthenticationRequestData::try_from(payment_data.clone())?;

        let pre_auth_router_data: hyperswitch_domain_models::types::UasPreAuthenticationRouterData =
            utils::construct_uas_router_data(
                connector_name.to_string(),
                payment_method,
                payment_data.payment_attempt.merchant_id.clone(),
                None,
                pre_authentication_data,
                merchant_connector_account,
                Some(authentication_id.to_owned()),
            )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        )
        .await?;

        Ok(())
    }

    async fn post_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccount,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<hyperswitch_domain_models::types::UasPostAuthenticationRouterData> {
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let post_authentication_data = UasPostAuthenticationRequestData {};

        let post_auth_router_data: hyperswitch_domain_models::types::UasPostAuthenticationRouterData = utils::construct_uas_router_data(
            connector_name.to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            post_authentication_data,
            merchant_connector_account,
            Some(authentication_id.clone()),
        )?;

        let response = utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            post_auth_router_data,
        )
        .await?;

        Ok(response)
    }

    async fn confirmation(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccount,
        connector_name: &str,
        payment_method: common_enums::PaymentMethod,
    ) -> RouterResult<()> {
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let authentication_confirmation_data =
            UasConfirmationRequestData::try_from(payment_data.clone())?;

        let authentication_confirmation_router_data : hyperswitch_domain_models::types::UasAuthenticationConfirmationRouterData = utils::construct_uas_router_data(
            connector_name.to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            authentication_confirmation_data,
            merchant_connector_account,
            Some(authentication_id.clone()),
        )?;

        utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            authentication_confirmation_router_data,
        )
        .await?;

        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn create_new_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    profile_id: common_utils::id_type::ProfileId,
    payment_id: Option<common_utils::id_type::PaymentId>,
    merchant_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    authentication_id: &str,
    service_details: Option<CtpServiceDetails>,
    authentication_status: common_enums::AuthenticationStatus,
    network_token: Option<hyperswitch_domain_models::payment_method_data::NetworkTokenData>,
) -> RouterResult<Authentication> {
    let service_details_value = service_details
        .map(serde_json::to_value)
        .transpose()
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable(
            "unable to parse service details into json value while inserting to DB",
        )?;
    let new_authorization = AuthenticationNew {
        authentication_id: authentication_id.to_owned(),
        merchant_id,
        authentication_connector,
        connector_authentication_id: None,
        payment_method_id: "".to_string(),
        authentication_type: None,
        authentication_status,
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
        error_message: None,
        error_code: None,
        connector_metadata: None,
        maximum_supported_version: None,
        threeds_server_transaction_id: None,
        cavv: network_token
            .clone()
            .and_then(|data| data.token_cryptogram.map(|cavv| cavv.expose())),
        authentication_flow_type: None,
        message_version: None,
        eci: network_token.and_then(|data| data.eci),
        trans_status: None,
        acquirer_bin: None,
        acquirer_merchant_id: None,
        three_ds_method_data: None,
        three_ds_method_url: None,
        acs_url: None,
        challenge_request: None,
        acs_reference_number: None,
        acs_trans_id: None,
        acs_signed_content: None,
        profile_id,
        payment_id,
        merchant_connector_id,
        ds_trans_id: None,
        directory_server_id: None,
        acquirer_country_code: None,
        service_details: service_details_value,
    };
    state
        .store
        .insert_authentication(new_authorization)
        .await
        .to_duplicate_response(ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Authentication with authentication_id {} already exists",
                authentication_id
            ),
        })
}

pub async fn process_incoming_webhook(
    state: &SessionState,
    incoming_webhook_request: &IncomingWebhookRequestDetails<'_>,
    connector_name: &str,
) -> RouterResult<Vec<u8>> {
    let webhook_data = transformers::get_webhook_request_data_for_uas(incoming_webhook_request);

    let webhook_router_data: hyperswitch_domain_models::types::UasProcessWebhookRouterData =
        utils::construct_uas_webhook_router_data(connector_name.to_string(), webhook_data)?;

    let response = utils::do_auth_connector_call(
        state,
        UNIFIED_AUTHENTICATION_SERVICE.to_string(),
        webhook_router_data,
    )
    .await?;

    let response_body = match response.response {
        Ok(resp) => match resp {
            UasAuthenticationResponseData::Webhook {
                trans_status,
                authentication_value,
                eci,
                three_ds_server_transaction_id,
            } => Ok(WebhookResponse {
                trans_status,
                authentication_value,
                eci,
                three_ds_server_transaction_id,
            }),
            _ => Err(ApiErrorResponse::WebhookProcessingFailure),
        },
        Err(err) => Err(ApiErrorResponse::WebhookProcessingFailure),
    }?;

    let serialized =
        serde_json::to_vec(&response_body).change_context(ApiErrorResponse::InternalServerError)?;

    Ok(serialized)
}
