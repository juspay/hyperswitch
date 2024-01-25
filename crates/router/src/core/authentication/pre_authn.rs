use cards::CardNumber;
use common_utils;
use error_stack::ResultExt;

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
    connector_account_for_3ds: String,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<()> {
    let authentication =
        create_new_authentication(state, merchant_account.merchant_id.clone()).await?;
    match authentication_flow_input {
        types::AuthenthenticationFlowInput::PaymentAuthNFlow {
            payment_data,
            should_continue_confirm_transaction,
            card_number,
        } => {
            let router_data =
                do_pre_auth_connector_call(state, card_number, connector_account_for_3ds.clone())
                    .await?;

            let (authentication, authentication_data) =
                utils::update_trackers(state, router_data, authentication).await?;
            let external_3ds_authentication_requested =
                if authentication_data.maximum_supported_version.0 == 2 {
                    *should_continue_confirm_transaction = false; // if 3ds version is >= 2
                    true
                } else {
                    false
                };
            let attempt_update = storage::PaymentAttemptUpdate::AuthenticationUpdate {
                external_3ds_authentication_requested: Some(external_3ds_authentication_requested),
                authentication_provider: Some(connector_account_for_3ds.clone()),
                authentication_id: Some(authentication.authentication_id.clone()),
                updated_by: merchant_account.storage_scheme.to_string(),
            };

            payment_data.payment_attempt = state
                .store
                .update_payment_attempt_with_attempt_id(
                    payment_data.payment_attempt.to_owned(),
                    attempt_update,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(ApiErrorResponse::PaymentNotFound)?;
            payment_data.authentication = Some((authentication, authentication_data))
        }
        types::AuthenthenticationFlowInput::PaymentMethodAuthNFlow {
            card_number,
            other_fields: _,
        } => {
            let router_data =
                do_pre_auth_connector_call(state, card_number, connector_account_for_3ds).await?;
            todo!("Some operation");
        }
    };
    Ok(())
}

// async fn payment_authentication_operations<F: Clone>(
//     state: &AppState,
//     connector: &ConnectorCallType,
//     payment_data: &mut payments::PaymentData<F>,
//     should_continue_confirm_transaction: &mut bool,
// ) -> RouterResult<()> {
//     if !should_continue_confirm_transaction {
//         return Ok(());
//     }
//     let separate_authn_supported = utils::is_separate_authn_supported(connector);
//     let separate_authn_requested = payment_data
//         .payment_attempt
//         .external_3ds_authentication_requested
//         .unwrap_or(false);
//     if separate_authn_requested && separate_authn_supported {
//         let authentication =
//             create_new_authentication(state, &payment_data.payment_attempt).await?;
//         let merchant_connector_account = todo!("Fetch MCA");

//         // call 3ds conector version call(connector_processing_step)
//         let is_3ds_version_greater_than_2 = true;
//         if is_3ds_version_greater_than_2 {
//             *should_continue_confirm_transaction = false;
//         }
//     }
//     Ok(())
// }

async fn do_pre_auth_connector_call(
    state: &AppState,
    card_holder_account_number: CardNumber,
    merchant_connector_account: String,
) -> RouterResult<RouterData<api::PreAuthN, PreAuthNRequestData, AuthenticationResponseData>> {
    let request = PreAuthNRequestData {
        card_holder_account_number,
    };
    let temp_response_data = AuthenticationResponseData::PreAuthNResponse {
        threeds_server_transaction_id: "".into(),
        maximum_supported_3ds_version: (0, 0, 0),
        connector_authentication_id: "".into(),
    };
    let router_data = utils::construct_router_data(
        None,
        None,
        None,
        None,
        request,
        temp_response_data,
        merchant_connector_account,
    )?;
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        api::PreAuthN,
        PreAuthNRequestData,
        AuthenticationResponseData,
    > = todo!();
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
        connector: "".into(),
        connector_authentication_id: None,
        authentication_data: None,
        payment_method_id: "".into(),
        authentication_type: None,
        authentication_status: common_enums::AuthenticationStatus::Started,
        lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
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
