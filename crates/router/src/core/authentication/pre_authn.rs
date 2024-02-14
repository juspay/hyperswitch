use cards::CardNumber;
use common_utils;

use super::{types, utils};
use crate::{
    consts,
    core::{
        errors::{
            utils::{ConnectorErrorExt, StorageErrorExt},
            ApiErrorResponse, RouterResult,
        },
        payments,
    },
    routes::AppState,
    services::{self, execute_connector_processing_step},
    types::{
        api,
        authentication::{AuthenticationResponseData, PreAuthNRequestData},
        domain, storage, RouterData,
    },
};

pub async fn execute_pre_auth_flow<F: Clone + Send>(
    state: &AppState,
    authentication_flow_input: types::AuthenthenticationFlowInput<'_, F>,
    merchant_account: &domain::MerchantAccount,
    three_ds_connector_account: &domain::MerchantConnectorAccount,
) -> RouterResult<()> {
    let authentication =
        create_new_authentication(state, merchant_account.merchant_id.clone()).await?;
    match authentication_flow_input {
        types::AuthenthenticationFlowInput::PaymentAuthNFlow {
            payment_data,
            should_continue_confirm_transaction,
            card_number,
        } => {
            let router_data: RouterData<
                api::PreAuthentication,
                PreAuthNRequestData,
                AuthenticationResponseData,
            > = do_pre_auth_connector_call(state, card_number, three_ds_connector_account).await?;

            let (authentication, authentication_data) = utils::update_trackers(
                state,
                router_data,
                authentication,
                payment_data.token.clone(),
            )
            .await?;
            if authentication_data.is_separate_authn_required() {
                *should_continue_confirm_transaction = true;
            }
            payment_data.authentication = Some((authentication, authentication_data))
        }
        types::AuthenthenticationFlowInput::PaymentMethodAuthNFlow {
            card_number,
            other_fields: _,
        } => {
            let _router_data =
                do_pre_auth_connector_call(state, card_number, three_ds_connector_account).await?;
            // todo!("Some operation");
        }
    };
    Ok(())
}

async fn do_pre_auth_connector_call(
    state: &AppState,
    card_holder_account_number: CardNumber,
    three_ds_connector_account: &domain::MerchantConnectorAccount,
) -> RouterResult<RouterData<api::PreAuthentication, PreAuthNRequestData, AuthenticationResponseData>>
{
    let request = PreAuthNRequestData {
        card_holder_account_number,
    };
    let temp_response_data = AuthenticationResponseData::PreAuthNResponse {
        threeds_server_transaction_id: "".into(),
        maximum_supported_3ds_version: (0, 0, 0),
        authentication_connector_id: "".into(),
        three_ds_method_data: "".into(),
        three_ds_method_url: None,
        message_version: "".into(),
    };

    let router_data = utils::construct_router_data(
        None,
        None,
        None,
        None,
        request,
        temp_response_data,
        three_ds_connector_account,
    )?;
    let connector_data = api::AuthenticationConnectorData::get_connector_by_name(
        &three_ds_connector_account.connector_name,
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PreAuthentication,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > = connector_data.connector.get_connector_integration();
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

async fn create_new_authentication(
    state: &AppState,
    merchant_id: String,
) -> RouterResult<storage::Authentication> {
    let authorization_id =
        common_utils::generate_id_with_default_len(consts::AUTHENTICATION_ID_PREFIX);
    let new_authorization = storage::AuthenticationNew {
        authentication_id: authorization_id.clone(),
        merchant_id,
        authentication_connector: "".into(),
        authentication_connector_id: None,
        authentication_data: None,
        payment_method_id: "".into(),
        authentication_type: None,
        authentication_status: common_enums::AuthenticationStatus::Started,
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
    };
    state
        .store
        .insert_authentication(new_authorization)
        .await
        .to_duplicate_response(ApiErrorResponse::GenericDuplicateError {
            message: format!(
                "Authentication with authentication_id {} already exists",
                authorization_id
            ),
        })
}
