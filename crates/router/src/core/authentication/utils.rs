use error_stack::ResultExt;

use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, StorageErrorExt},
        payments,
    },
    errors::RouterResult,
    routes::AppState,
    services::{self, execute_connector_processing_step},
    types::{
        api::{self, ConnectorCallType},
        authentication::AuthenticationResponseData,
        storage,
        transformers::ForeignFrom,
        RouterData,
    },
};

pub fn get_connector_name_if_separate_authn_supported(
    connector_call_type: &ConnectorCallType,
) -> Option<String> {
    match connector_call_type {
        ConnectorCallType::PreDetermined(connector_data) => {
            if connector_data
                .connector_name
                .is_separate_authentication_supported()
            {
                Some(connector_data.connector_name.to_string())
            } else {
                None
            }
        }
        ConnectorCallType::Retryable(connectors) => connectors.first().and_then(|connector_data| {
            if connector_data
                .connector_name
                .is_separate_authentication_supported()
            {
                Some(connector_data.connector_name.to_string())
            } else {
                None
            }
        }),
        ConnectorCallType::SessionMultiple(_) => None,
    }
}

pub async fn update_trackers<F: Clone, Req>(
    state: &AppState,
    router_data: RouterData<F, Req, AuthenticationResponseData>,
    authentication: storage::Authentication,
    token: Option<String>,
    acquirer_details: Option<super::types::AcquirerDetails>,
) -> RouterResult<storage::Authentication> {
    let authentication_update = match router_data.response {
        Ok(response) => match response {
            AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
            } => {
                // todo!("maximum_supported_3ds_version");
                storage::AuthenticationUpdate::PreAuthenticationUpdate {
                    threeds_server_transaction_id,
                    maximum_supported_3ds_version,
                    connector_authentication_id,
                    three_ds_method_data,
                    three_ds_method_url,
                    message_version,
                    connector_metadata,
                    authentication_status: common_enums::AuthenticationStatus::Pending,
                    payment_method_id: token.map(|token| format!("eph_{}", token)),
                    acquirer_bin: acquirer_details
                        .as_ref()
                        .map(|acquirer_details| acquirer_details.acquirer_bin.clone()),
                    acquirer_merchant_id: acquirer_details
                        .map(|acquirer_details| acquirer_details.acquirer_merchant_id),
                }
            }
            AuthenticationResponseData::AuthNResponse {
                authn_flow_type,
                authentication_value,
                trans_status,
            } => {
                let authentication_status =
                    common_enums::AuthenticationStatus::foreign_from(trans_status.clone());
                storage::AuthenticationUpdate::AuthenticationUpdate {
                    authentication_value,
                    trans_status,
                    acs_url: authn_flow_type.get_acs_url(),
                    challenge_request: authn_flow_type.get_challenge_request(),
                    acs_reference_number: authn_flow_type.get_acs_reference_number(),
                    acs_trans_id: authn_flow_type.get_acs_trans_id(),
                    acs_signed_content: authn_flow_type.get_acs_signed_content(),
                    authentication_type: authn_flow_type.get_decoupled_authentication_type(),
                    authentication_status,
                }
            }
            AuthenticationResponseData::PostAuthNResponse {
                trans_status,
                authentication_value,
                eci,
            } => storage::AuthenticationUpdate::PostAuthenticationUpdate {
                authentication_status: common_enums::AuthenticationStatus::foreign_from(
                    trans_status.clone(),
                ),
                trans_status,
                authentication_value,
                eci,
            },
        },
        Err(error) => storage::AuthenticationUpdate::ErrorUpdate {
            connector_authentication_id: error.connector_transaction_id,
            authentication_status: common_enums::AuthenticationStatus::Failed,
            error_message: Some(error.message),
            error_code: Some(error.code),
        },
    };
    state
        .store
        .update_authentication_by_merchant_id_authentication_id(
            authentication,
            authentication_update,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error while updating authentication")
}

impl ForeignFrom<common_enums::AuthenticationStatus> for common_enums::AttemptStatus {
    fn foreign_from(from: common_enums::AuthenticationStatus) -> Self {
        match from {
            common_enums::AuthenticationStatus::Started
            | common_enums::AuthenticationStatus::Pending => Self::AuthenticationPending,
            common_enums::AuthenticationStatus::Success => Self::AuthenticationSuccessful,
            common_enums::AuthenticationStatus::Failed => Self::AuthenticationFailed,
        }
    }
}

pub async fn create_new_authentication(
    state: &AppState,
    merchant_id: String,
    authentication_connector: String,
) -> RouterResult<storage::Authentication> {
    let authentication_id =
        common_utils::generate_id_with_default_len(consts::AUTHENTICATION_ID_PREFIX);
    let new_authorization = storage::AuthenticationNew {
        authentication_id: authentication_id.clone(),
        merchant_id,
        authentication_connector,
        connector_authentication_id: None,
        payment_method_id: "".into(),
        authentication_type: None,
        authentication_status: common_enums::AuthenticationStatus::Started,
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
        error_message: None,
        error_code: None,
        connector_metadata: None,
        maximum_supported_version: None,
        threeds_server_transaction_id: None,
        cavv: None,
        authentication_flow_type: None,
        message_version: None,
        eci: None,
        trans_status: None,
        acquirer_bin: None,
        acquirer_merchant_id: None,
        three_ds_method_data: None,
        three_ds_method_url: None,
        acs_url: None,
        challenge_request: None,
        acs_reference_number: None,
        acs_trans_id: None,
        three_dsserver_trans_id: None,
        acs_signed_content: None,
    };
    state
        .store
        .insert_authentication(new_authorization)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Authentication with authentication_id {} already exists",
                authentication_id
            ),
        })
}

pub async fn do_auth_connector_call<F, Req, Res>(
    state: &AppState,
    authentication_connector_name: String,
    router_data: RouterData<F, Req, Res>,
) -> RouterResult<RouterData<F, Req, Res>>
where
    Req: std::fmt::Debug + Clone + 'static,
    Res: std::fmt::Debug + Clone + 'static,
    F: std::fmt::Debug + Clone + 'static,
    dyn api::Connector + Sync: services::api::ConnectorIntegration<F, Req, Res>,
{
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector_name)?;
    let connector_integration: services::BoxedConnectorIntegration<'_, F, Req, Res> =
        connector_data.connector.get_connector_integration();
    let router_data = execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
    .to_payment_failed_response()?;
    Ok(router_data)
}
