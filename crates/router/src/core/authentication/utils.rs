use common_enums::DecoupledAuthenticationType;
use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;

use super::types::AuthenticationData;
use crate::{
    consts,
    core::{
        errors::{ApiErrorResponse, ConnectorErrorExt, StorageErrorExt},
        payments,
    },
    errors::RouterResult,
    routes::AppState,
    services::{self, execute_connector_processing_step},
    types::{
        self as router_types,
        api::{self, ConnectorCallType},
        authentication::{AuthNFlowType, AuthenticationResponseData},
        storage,
        transformers::ForeignFrom,
        RouterData,
    },
};
pub fn is_separate_authn_supported_connector(connector: router_types::Connector) -> bool {
    #[cfg(feature = "dummy_connector")]
    match connector {
        api_models::enums::Connector::DummyConnector1
        | api_models::enums::Connector::DummyConnector2
        | api_models::enums::Connector::DummyConnector3
        | api_models::enums::Connector::DummyConnector4
        | api_models::enums::Connector::DummyConnector5
        | api_models::enums::Connector::DummyConnector6
        | api_models::enums::Connector::DummyConnector7
        | api_models::enums::Connector::Aci
        | api_models::enums::Connector::Adyen
        | api_models::enums::Connector::Airwallex
        | api_models::enums::Connector::Authorizedotnet
        | api_models::enums::Connector::Bambora
        | api_models::enums::Connector::Bankofamerica
        | api_models::enums::Connector::Bitpay
        | api_models::enums::Connector::Bluesnap
        | api_models::enums::Connector::Boku
        | api_models::enums::Connector::Braintree
        | api_models::enums::Connector::Cashtocode
        | api_models::enums::Connector::Coinbase
        | api_models::enums::Connector::Cryptopay
        | api_models::enums::Connector::Dlocal
        | api_models::enums::Connector::Fiserv
        | api_models::enums::Connector::Forte
        | api_models::enums::Connector::Globalpay
        | api_models::enums::Connector::Globepay
        | api_models::enums::Connector::Gocardless
        | api_models::enums::Connector::Helcim
        | api_models::enums::Connector::Iatapay
        | api_models::enums::Connector::Klarna
        | api_models::enums::Connector::Mollie
        | api_models::enums::Connector::Multisafepay
        | api_models::enums::Connector::Nexinets
        | api_models::enums::Connector::Nmi
        | api_models::enums::Connector::Nuvei
        | api_models::enums::Connector::Opennode
        | api_models::enums::Connector::Payme
        | api_models::enums::Connector::Paypal
        | api_models::enums::Connector::Payu
        | api_models::enums::Connector::Placetopay
        | api_models::enums::Connector::Powertranz
        | api_models::enums::Connector::Prophetpay
        | api_models::enums::Connector::Rapyd
        | api_models::enums::Connector::Shift4
        | api_models::enums::Connector::Square
        | api_models::enums::Connector::Stax
        | api_models::enums::Connector::Trustpay
        | api_models::enums::Connector::Tsys
        | api_models::enums::Connector::Volt
        | api_models::enums::Connector::Wise
        | api_models::enums::Connector::Worldline
        | api_models::enums::Connector::Worldpay
        | api_models::enums::Connector::Zen
        | api_models::enums::Connector::Signifyd
        | api_models::enums::Connector::Plaid
        | api_models::enums::Connector::Riskified
        | api_models::enums::Connector::Threedsecureio
        | api_models::enums::Connector::Cybersource
        | api_models::enums::Connector::Noon
        | api_models::enums::Connector::Stripe => false,
        api_models::enums::Connector::Checkout => true,
    }
    #[cfg(not(feature = "dummy_connector"))]
    match connector {
        api_models::enums::Connector::Aci
        | api_models::enums::Connector::Adyen
        | api_models::enums::Connector::Airwallex
        | api_models::enums::Connector::Authorizedotnet
        | api_models::enums::Connector::Bambora
        | api_models::enums::Connector::Bankofamerica
        | api_models::enums::Connector::Bitpay
        | api_models::enums::Connector::Bluesnap
        | api_models::enums::Connector::Boku
        | api_models::enums::Connector::Braintree
        | api_models::enums::Connector::Cashtocode
        | api_models::enums::Connector::Coinbase
        | api_models::enums::Connector::Cryptopay
        | api_models::enums::Connector::Dlocal
        | api_models::enums::Connector::Fiserv
        | api_models::enums::Connector::Forte
        | api_models::enums::Connector::Globalpay
        | api_models::enums::Connector::Globepay
        | api_models::enums::Connector::Gocardless
        | api_models::enums::Connector::Helcim
        | api_models::enums::Connector::Iatapay
        | api_models::enums::Connector::Klarna
        | api_models::enums::Connector::Mollie
        | api_models::enums::Connector::Multisafepay
        | api_models::enums::Connector::Nexinets
        | api_models::enums::Connector::Nmi
        | api_models::enums::Connector::Nuvei
        | api_models::enums::Connector::Opennode
        | api_models::enums::Connector::Payme
        | api_models::enums::Connector::Paypal
        | api_models::enums::Connector::Payu
        | api_models::enums::Connector::Placetopay
        | api_models::enums::Connector::Powertranz
        | api_models::enums::Connector::Prophetpay
        | api_models::enums::Connector::Rapyd
        | api_models::enums::Connector::Shift4
        | api_models::enums::Connector::Square
        | api_models::enums::Connector::Stax
        | api_models::enums::Connector::Trustpay
        | api_models::enums::Connector::Tsys
        | api_models::enums::Connector::Volt
        | api_models::enums::Connector::Wise
        | api_models::enums::Connector::Worldline
        | api_models::enums::Connector::Worldpay
        | api_models::enums::Connector::Zen
        | api_models::enums::Connector::Signifyd
        | api_models::enums::Connector::Plaid
        | api_models::enums::Connector::Riskified
        | api_models::enums::Connector::Threedsecureio
        | api_models::enums::Connector::Cybersource
        | api_models::enums::Connector::Noon
        | api_models::enums::Connector::Stripe => false,
        api_models::enums::Connector::Checkout => true,
    }
}

pub fn get_connector_name_if_separate_authn_supported(
    connector_call_type: &ConnectorCallType,
) -> Option<String> {
    match connector_call_type {
        ConnectorCallType::PreDetermined(connector_data) => {
            if is_separate_authn_supported_connector(connector_data.connector_name) {
                Some(connector_data.connector_name.to_string())
            } else {
                None
            }
        }
        ConnectorCallType::Retryable(connectors) => connectors.first().and_then(|connector_data| {
            if is_separate_authn_supported_connector(connector_data.connector_name) {
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
) -> RouterResult<(storage::Authentication, AuthenticationData)> {
    let mut authentication_data = authentication
        .authentication_data
        .as_ref()
        .map(|authentication_data| {
            authentication_data
                .to_owned()
                .parse_value::<AuthenticationData>("AuthenticationData")
                .change_context(ApiErrorResponse::InternalServerError)
        })
        .transpose()?
        .unwrap_or_default();

    let authentication_update = match router_data.response {
        Ok(response) => Some(match response {
            AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
                connector_metadata,
            } => {
                authentication_data.maximum_supported_version = maximum_supported_3ds_version;
                authentication_data.threeds_server_transaction_id = threeds_server_transaction_id;
                authentication_data
                    .three_ds_method_data
                    .three_ds_method_data = three_ds_method_data;
                authentication_data
                    .three_ds_method_data
                    .three_ds_method_data_submission = three_ds_method_url.is_some();
                authentication_data.three_ds_method_data.three_ds_method_url = three_ds_method_url;
                authentication_data.message_version = message_version;
                authentication_data.acquirer_details = acquirer_details;

                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: Some(connector_authentication_id),
                    payment_method_id: token.map(|token| format!("eph_{}", token)),
                    authentication_type: None,
                    authentication_status: Some(common_enums::AuthenticationStatus::Started),
                    authentication_lifecycle_status: None,
                    connector_metadata,
                }
            }
            AuthenticationResponseData::AuthNResponse {
                authn_flow_type,
                cavv,
                trans_status,
            } => {
                authentication_data.authn_flow_type = Some(authn_flow_type.clone());
                authentication_data.cavv = cavv.or(authentication_data.cavv);
                authentication_data.trans_status = trans_status.clone();
                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: None,
                    payment_method_id: None,
                    authentication_type: Some(match authn_flow_type {
                        AuthNFlowType::Challenge { .. } => DecoupledAuthenticationType::Challenge,
                        AuthNFlowType::Frictionless => DecoupledAuthenticationType::Frictionless,
                    }),
                    authentication_status: Some(common_enums::AuthenticationStatus::foreign_from(
                        trans_status,
                    )),
                    authentication_lifecycle_status: None,
                    connector_metadata: None,
                }
            }
            AuthenticationResponseData::PostAuthNResponse {
                trans_status,
                authentication_value,
                eci,
            } => {
                authentication_data.cavv = authentication_value;
                authentication_data.eci = eci;
                authentication_data.trans_status = trans_status.clone();
                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: None,
                    payment_method_id: None,
                    authentication_type: None,
                    authentication_status: Some(common_enums::AuthenticationStatus::foreign_from(
                        trans_status,
                    )),
                    authentication_lifecycle_status: None,
                    connector_metadata: None,
                }
            }
        }),
        Err(error) => Some(storage::AuthenticationUpdate::ErrorUpdate {
            connector_authentication_id: error.connector_transaction_id,
            authentication_status: common_enums::AuthenticationStatus::Failed,
            error_message: Some(error.message),
            error_code: Some(error.code),
        }),
    };
    let authentication_result = if let Some(authentication_update) = authentication_update {
        state
            .store
            .update_authentication_by_merchant_id_authentication_id(
                authentication,
                authentication_update,
            )
            .await
            .change_context(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while updating authentication")
    } else {
        Ok(authentication)
    };
    authentication_result.map(|authentication| (authentication, authentication_data))
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
        authentication_data: None,
        payment_method_id: "".into(),
        authentication_type: None,
        authentication_status: common_enums::AuthenticationStatus::Started,
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus::Unused,
        error_message: None,
        error_code: None,
        connector_metadata: None,
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
