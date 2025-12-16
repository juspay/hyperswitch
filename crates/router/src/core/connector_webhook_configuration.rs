mod transformers;
use common_utils::id_type;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_request_types::configure_connector_webhook::ConnectorWebhookRegisterData,
    router_response_types::configure_connector_webhook::ConnectorWebhookRegisterResponse,
};
use transformers as configure_connector_webhook_flow;

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        utils as core_utils,
    },
    errors::utils::ConnectorErrorExt,
    routes::SessionState,
    services::{
        self,
        api::{self as service_api},
    },
    types::api,
};

#[cfg(feature = "v1")]
pub async fn register_connector_webhook(
    state: SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: Option<id_type::ProfileId>,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
    req: api_models::admin::ConnectorWebhookRegisterRequest,
) -> RouterResponse<api_models::admin::RegisterConnectorWebhookResponse> {
    let db = state.store.as_ref();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(&merchant_id, &db.get_master_key().to_vec().into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = db
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            merchant_id,
            merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.get_string_repr().to_string(),
        })?;
    core_utils::validate_profile_id_from_auth_layer(profile_id, &mca)?;
    let connector_name = mca.connector_name.clone();
    let profile_id = mca.profile_id.clone().get_string_repr().to_string();

    // validate request

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        Some(mca.merchant_connector_id.clone()),
    )?;
    let connector_integration: services::BoxedConnectorWebhookConfigurationInterface<
        api::ConnectorWebhookRegister,
        ConnectorWebhookRegisterData,
        ConnectorWebhookRegisterResponse,
    > = connector_data.connector.get_connector_integration();

    let router_data =
        configure_connector_webhook_flow::construct_webhook_register_router_data(&state, &mca, req)
            .await?;

    let response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_webhook_configuration_failed_response()
    .attach_printable("Failed while calling register webhook connector api")?;

    let register_webhook_response = response.response.as_ref().map_err(|err| {
        errors::ApiErrorResponse::ExternalConnectorError {
            code: err.code.clone(),
            message: err.message.clone(),
            connector: connector_name.clone(),
            status_code: err.status_code,
            reason: err.reason.clone(),
        }
    })?;

    let (should_update_db, connector_webhook_registration_details) =
        configure_connector_webhook_flow::construct_connector_webhook_registration_details(
            &register_webhook_response,
            &mca, &router_data.request,
        )?;

    if should_update_db {
        db.update_merchant_connector_account(mca.clone(), connector_webhook_registration_details.into(), &key_store)
            .await
            .change_context(
                errors::ApiErrorResponse::DuplicateMerchantConnectorAccount {
                    profile_id,
                    connector_label: connector_name.to_owned(),
                },
            )
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating MerchantConnectorAccount: id: {merchant_connector_id:?}",
                )
            })?;
    };


    let response =
        configure_connector_webhook_flow::construct_connector_webhook_registration_response(
            register_webhook_response,
            &router_data.request,
        )?;

    Ok(service_api::ApplicationResponse::Json(response))
}
