mod transformers;
use common_utils::id_type;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_connector_account::MerchantConnectorAccountUpdate,
    router_request_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecretRequest, ConnectorWebhookRegisterRequest,
    },
    router_response_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecretResponse, ConnectorWebhookRegisterResponse,
    },
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
    types::{api, domain},
};

#[cfg(feature = "v1")]
pub async fn register_connector_webhook(
    state: SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: Option<id_type::ProfileId>,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
    req: api_models::merchant_connector_webhook_management::ConnectorWebhookRegisterRequest,
) -> RouterResponse<
    api_models::merchant_connector_webhook_management::RegisterConnectorWebhookResponse,
> {
    let db = state.store.as_ref();
    let key_store = db
        .get_merchant_key_store_by_merchant_id(merchant_id, &db.get_master_key().to_vec().into())
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

    let connector_data = api::ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &connector_name,
        api::GetToken::Connector,
        Some(mca.merchant_connector_id.clone()),
    )?;

    configure_connector_webhook_flow::validate_webhook_registration_request(
        &connector_data,
        req.clone(),
    )
    .await?;

    let register_integration: services::BoxedConnectorWebhookConfigurationInterface<
        api::ConnectorWebhookRegister,
        ConnectorWebhookRegisterRequest,
        ConnectorWebhookRegisterResponse,
    > = connector_data.connector.get_connector_integration();

    let register_router_data =
        configure_connector_webhook_flow::construct_webhook_register_router_data(&state, &mca, req)
            .await?;

    let register_router_data = services::execute_connector_processing_step(
        &state,
        register_integration,
        &register_router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_webhook_configuration_failed_response()
    .attach_printable("Failed while calling register webhook connector api")?;

    let register_webhook_response = register_router_data.response.as_ref().map_err(|err| {
        errors::ApiErrorResponse::ExternalConnectorError {
            code: err.code.clone(),
            message: err.message.clone(),
            connector: connector_name.clone(),
            status_code: err.status_code,
            reason: err.reason.clone(),
        }
    })?;

    // Run the GenerateSecret flow only when the connector requires it (e.g. Adyen) AND the
    // register step returned a connector_webhook_id to operate on. A failure here does not
    // fail registration — register success is still surfaced and the secret-generation error
    // is reported alongside it in the response.
    let generate_secret_response = if let Some(connector_webhook_id) = connector_data
        .connector
        .requires_webhook_secret_generation()
        .then(|| register_webhook_response.connector_webhook_id.clone())
        .flatten()
    {
        Some(
            generate_connector_webhook_secret(&state, &connector_data, &mca, connector_webhook_id)
                .await?,
        )
    } else {
        None
    };

    let generated_secret = generate_secret_response
        .as_ref()
        .and_then(|resp| resp.secret.clone());

    let mca_update =
        configure_connector_webhook_flow::construct_connector_webhook_registration_details(
            register_webhook_response,
            &mca,
            &register_router_data.request,
            generated_secret,
        )?;

    let should_update_db = matches!(
        mca_update,
        MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_registration_details: Some(_),
            ..
        } | MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_details: Some(_),
            ..
        }
    );

    if should_update_db {
        db.update_merchant_connector_account(mca.clone(), mca_update.into(), &key_store)
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
            &register_router_data.request,
            generate_secret_response.as_ref(),
        )?;

    Ok(service_api::ApplicationResponse::Json(response))
}

/// Runs the GenerateSecret connector call. Returns the success payload or a synthesized failure
/// payload when the connector call itself returned a non-network error. Callers MUST check
/// [`requires_webhook_secret_generation`] and ensure a `connector_webhook_id` is available
/// before invoking this.
#[cfg(feature = "v1")]
async fn generate_connector_webhook_secret(
    state: &SessionState,
    connector_data: &api::ConnectorData,
    mca: &domain::MerchantConnectorAccount,
    connector_webhook_id: String,
) -> errors::RouterResult<ConnectorWebhookGenerateSecretResponse> {
    let generate_secret_integration: services::BoxedConnectorWebhookConfigurationInterface<
        api::ConnectorWebhookGenerateSecret,
        ConnectorWebhookGenerateSecretRequest,
        ConnectorWebhookGenerateSecretResponse,
    > = connector_data.connector.get_connector_integration();

    let generate_secret_router_data =
        configure_connector_webhook_flow::construct_generate_secret_router_data(
            state,
            mca,
            connector_webhook_id,
        )
        .await?;

    let generate_secret_router_data = services::execute_connector_processing_step(
        state,
        generate_secret_integration,
        &generate_secret_router_data,
        common_enums::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_webhook_configuration_failed_response()
    .attach_printable("Failed while calling generate secret connector api")?;

    Ok(match generate_secret_router_data.response {
        Ok(success) => success,
        Err(err) => ConnectorWebhookGenerateSecretResponse {
            secret: None,
            status: common_enums::WebhookSecretGenerationStatus::Failure,
            error_code: Some(err.code),
            error_message: Some(err.message),
        },
    })
}

#[cfg(feature = "v1")]
pub async fn fetch_connector_webhook(
    state: SessionState,
    merchant_id: id_type::MerchantId,
    profile_id: Option<id_type::ProfileId>,
    merchant_connector_id: id_type::MerchantConnectorAccountId,
) -> RouterResponse<api_models::merchant_connector_webhook_management::ConnectorWebhookListResponse>
{
    let store = state.store.as_ref();
    let key_store = store
        .get_merchant_key_store_by_merchant_id(
            &merchant_id,
            &store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let mca = store
        .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
            &merchant_id,
            &merchant_connector_id,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_connector_id.get_string_repr().to_string(),
        })?;

    let connector_webook_data =
        configure_connector_webhook_flow::get_connector_webhook_list_response(
            &mca.connector_webhook_registration_details,
        )?;

    core_utils::validate_profile_id_from_auth_layer(profile_id, &mca)?;

    Ok(service_api::ApplicationResponse::Json(
        api_models::merchant_connector_webhook_management::ConnectorWebhookListResponse {
            connector: mca.connector_name.clone(),
            webhooks: connector_webook_data,
        },
    ))
}
