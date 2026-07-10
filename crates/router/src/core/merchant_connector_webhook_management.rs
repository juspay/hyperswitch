mod transformers;
use api_models::merchant_connector_webhook_management::ConnectorWebhookRegisterRequest as ApiConnectorWebhookRegisterRequest;
use common_utils::id_type;
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    merchant_connector_account::MerchantConnectorAccountUpdate,
    router_request_types::{
        merchant_connector_webhook_management::{
            ConnectorWebhookGenerateSecretRequest, ConnectorWebhookRegisterRequest,
        },
        CurrentFlowInfo,
    },
    router_response_types::merchant_connector_webhook_management::{
        ConnectorWebhookGenerateSecretResponse, ConnectorWebhookRegisterResponse,
    },
};
use hyperswitch_interfaces::api::{ConnectorSpecifications, ConnectorValidation};
use transformers as configure_connector_webhook_flow;

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResponse, StorageErrorExt},
        payments::helpers,
        utils as core_utils,
    },
    routes::{metrics, SessionState},
    services::{
        self,
        api::{self as service_api},
    },
    types::{self, api, domain},
};

fn to_error_response<E: std::fmt::Display>(err: E) -> types::ErrorResponse {
    router_env::logger::error!(error=%err, "Webhook access token error");
    types::ErrorResponse {
        code: "WEBHOOK_ACCESS_TOKEN_ERROR".to_string(),
        message: "Failed to obtain access token for webhook registration".to_string(),
        status_code: 500,
        attempt_status: None,
        connector_transaction_id: None,
        connector_response_reference_id: None,
        reason: Some(err.to_string()),
        network_advice_code: None,
        network_decline_code: None,
        network_error_message: None,
        connector_metadata: None,
    }
}

async fn fetch_access_token_for_webhook(
    state: &SessionState,
    connector_data: &api::ConnectorData,
    router_data: &types::RouterData<
        api::ConnectorWebhookRegister,
        ConnectorWebhookRegisterRequest,
        ConnectorWebhookRegisterResponse,
    >,
    current_flow_info: Option<CurrentFlowInfo>,
) -> Result<Option<types::AccessToken>, types::ErrorResponse> {
    if !connector_data
        .connector_name
        .supports_access_token(router_data.payment_method)
    {
        return Ok(None);
    }

    let db = state.store.as_ref();
    let merchant_connector_id_or_connector_name = connector_data
        .merchant_connector_id
        .clone()
        .map(|mca_id| mca_id.get_string_repr().to_string())
        .unwrap_or(connector_data.connector_name.to_string());

    let key = connector_data
        .connector
        .get_access_token_key(
            &router_data.merchant_id,
            merchant_connector_id_or_connector_name.clone(),
            current_flow_info,
            router_data.payment_method_type,
            Some(false),
        )
        .map_err(to_error_response)?;

    router_env::logger::debug!("Fetching access token from Redis using key: {key}");

    let cached_token = db
        .get_access_token(key.clone())
        .await
        .map_err(to_error_response)?;

    match cached_token {
        Some(token) => Ok(Some(token)),
        None => {
            metrics::ACCESS_TOKEN_CACHE_MISS.add(
                1,
                router_env::metric_attributes!((
                    "connector",
                    connector_data.connector_name.to_string()
                )),
            );

            let refresh_token_request_data = types::AccessTokenRequestData::try_from((
                router_data.connector_auth_type.clone(),
                None,
                None,
            ))
            .map_err(to_error_response)?;

            let refresh_token_router_data =
                helpers::router_data_type_conversion::<_, api::AccessTokenAuth, _, _, _, _>(
                    router_data.clone(),
                    refresh_token_request_data,
                    Err(types::ErrorResponse::default()),
                );

            let access_token_connector_integration: services::BoxedAccessTokenConnectorIntegrationInterface<
                api::AccessTokenAuth,
                types::AccessTokenRequestData,
                types::AccessToken,
            > = connector_data.connector.get_connector_integration();

            let token_router_data = services::execute_connector_processing_step(
                state,
                access_token_connector_integration,
                &refresh_token_router_data,
                common_enums::CallConnectorAction::Trigger,
                None,
                None,
            )
            .await
            .map_err(to_error_response)?;

            let token = token_router_data.response.map_err(|err| {
                router_env::logger::error!(
                    error=?err,
                    connector=%connector_data.connector_name,
                    "Access token response contained an error"
                );
                err
            })?;

            let modified_token = types::AccessToken {
                expires: token
                    .expires
                    .saturating_sub(consts::REDUCE_ACCESS_TOKEN_EXPIRY_TIME.into()),
                ..token
            };

            if let Err(access_token_set_error) = db
                .set_access_token(key.clone(), modified_token.clone())
                .await
            {
                router_env::logger::error!(
                    access_token_set_error=?access_token_set_error,
                    "Failed to cache access token — proceeding anyway"
                );
            }

            Ok(Some(modified_token))
        }
    }
}

#[cfg(feature = "v1")]
pub async fn register_connector_webhook(
    state: SessionState,
    merchant_id: &id_type::MerchantId,
    profile_id: Option<id_type::ProfileId>,
    merchant_connector_id: &id_type::MerchantConnectorAccountId,
    req: ApiConnectorWebhookRegisterRequest,
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
        &state.conf.connectors,
    )
    .await?;

    let scope = req.scope.as_ref().ok_or_else(|| {
        Report::new(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("webhook registration scope is missing after request deserialization")
    })?;

    let registration_plan = connector_data
        .connector
        .get_webhook_registration_plan(scope, &state.conf.connectors)
        .to_webhook_configuration_failed_response()?;

    let scope_type = configure_connector_webhook_flow::determine_scope_type(scope);
    let requested = configure_connector_webhook_flow::extract_requested_identifiers(scope);

    let mut results = Vec::new();
    let mut registration_entries = Vec::new();
    let mut metadata_patches = Vec::new();
    let mut first_successful_webhook_id = None;

    for (identifier, base_url) in registration_plan {
        router_env::logger::info!(
            flow = "ConnectorWebhookRegister",
            connector = %connector_data.connector_name,
            scope = ?identifier,
            "Initiating connector webhook registration"
        );

        let merchant_connector_id = mca.merchant_connector_id.get_string_repr();
        let webhook_url =
            helpers::create_webhook_url(&state.base_url, &mca.merchant_id, merchant_connector_id);
        let scoped_request = ConnectorWebhookRegisterRequest {
            scope: identifier.clone(),
            base_url: base_url
                .parse::<url::Url>()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid base_url in webhook registration plan")?,
            webhook_url: webhook_url
                .parse::<url::Url>()
                .map(hyperswitch_masking::Secret::new)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Invalid webhook_url for connector registration")?,
        };

        let connector_integration: services::BoxedConnectorWebhookConfigurationInterface<
            api::ConnectorWebhookRegister,
            ConnectorWebhookRegisterRequest,
            ConnectorWebhookRegisterResponse,
        > = connector_data.connector.get_connector_integration();

        let mut router_data =
            configure_connector_webhook_flow::construct_webhook_register_router_data(
                &state,
                &mca,
                scoped_request,
            )
            .await?;

        let current_flow_info = Some(CurrentFlowInfo::ConnectorWebhookRegister {
            request_data: Box::new(router_data.request.clone()),
        });

        match fetch_access_token_for_webhook(
            &state,
            &connector_data,
            &router_data,
            current_flow_info,
        )
        .await
        {
            Ok(Some(token)) => router_data.access_token = Some(token),
            Ok(None) => {}
            Err(err) => {
                results.push(
                    api_models::merchant_connector_webhook_management::WebhookRegistrationResult {
                        identifier: identifier.clone(),
                        status: common_enums::WebhookRegistrationStatus::Failure,
                        connector_webhook_id: None,
                        error: Some(
                            api_models::merchant_connector_webhook_management::WebhookRegistrationError {
                                code: err.code,
                                message: err.message,
                            },
                        ),
                    },
                );
                continue;
            }
        }

        let response = match services::execute_connector_processing_step(
            &state,
            connector_integration,
            &router_data,
            common_enums::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        {
            Ok(resp) => resp,
            Err(err) => {
                router_env::logger::error!(
                    error=?err,
                    connector=%connector_data.connector_name,
                    scope=?identifier,
                    "Connector webhook registration call failed; continuing with next item"
                );
                results.push(
                    api_models::merchant_connector_webhook_management::WebhookRegistrationResult {
                        identifier: identifier.clone(),
                        status: common_enums::WebhookRegistrationStatus::Failure,
                        connector_webhook_id: None,
                        error: Some(
                            api_models::merchant_connector_webhook_management::WebhookRegistrationError {
                                code: "CONNECTOR_EXECUTION_ERROR".to_string(),
                                message: err.to_string(),
                            },
                        ),
                    },
                );
                continue;
            }
        };

        let result = match response.response {
            Ok(success) => {
                if let Some(metadata) = success.metadata {
                    metadata_patches.push(metadata);
                }

                api_models::merchant_connector_webhook_management::WebhookRegistrationResult {
                    identifier: identifier.clone(),
                    status: success.status,
                    connector_webhook_id: success.connector_webhook_id,
                    error: None,
                }
            }
            Err(err) => api_models::merchant_connector_webhook_management::WebhookRegistrationResult {
                identifier: identifier.clone(),
                status: common_enums::WebhookRegistrationStatus::Failure,
                connector_webhook_id: None,
                error: Some(api_models::merchant_connector_webhook_management::WebhookRegistrationError {
                    code: err.code,
                    message: err.message,
                }),
            },
        };

        if let Some(ref webhook_id) = result.connector_webhook_id {
            if first_successful_webhook_id.is_none() {
                first_successful_webhook_id = Some(webhook_id.clone());
            }
            registration_entries.push((webhook_id.clone(), identifier.clone()));
        }

        results.push(result);
    }

    // Run the GenerateSecret flow only when the connector requires it (e.g. Adyen) AND the
    // register step returned a connector_webhook_id to operate on. A failure here does not
    // fail registration — register success is still surfaced and the secret-generation error
    // is reported alongside it in the response.
    let generate_secret_response = if let Some(connector_webhook_id) = connector_data
        .connector
        .requires_webhook_secret_generation()
        .then_some(first_successful_webhook_id)
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
            &mca,
            registration_entries,
            generated_secret,
            metadata_patches,
        )?;

    let should_update_db = matches!(
        mca_update,
        MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_registration_details: Some(_),
            ..
        } | MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            connector_webhook_details: Some(_),
            ..
        } | MerchantConnectorAccountUpdate::ConnectorWebhookRegisterationUpdate {
            metadata: Some(_),
            ..
        }
    );

    if should_update_db {
        db.update_merchant_connector_account(mca.clone(), mca_update.into(), &key_store)
            .await
            .change_context(
                errors::ApiErrorResponse::DuplicateMerchantConnectorAccount {
                    profile_id: profile_id.clone(),
                    connector_label: connector_name.to_owned(),
                },
            )
            .attach_printable_lazy(|| {
                format!(
                    "Failed while updating MerchantConnectorAccount: id: {merchant_connector_id:?}",
                )
            })?;
    }

    let response =
        configure_connector_webhook_flow::construct_connector_webhook_registration_response(
            results,
            scope_type,
            requested,
            generate_secret_response.as_ref(),
            req.event_type.is_some(),
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
