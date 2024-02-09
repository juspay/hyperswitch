use common_utils::ext_traits::{Encode, ValueExt};
use error_stack::ResultExt;

use super::types::AuthenticationData;
use crate::{
    core::errors::ApiErrorResponse,
    errors::RouterResult,
    routes::AppState,
    types::{
        self as router_types, api::ConnectorCallType, authentication::AuthenticationResponseData,
        domain, storage, storage::enums as storage_enums, transformers::ForeignFrom,
        ConnectorAuthType, PaymentAddress, RouterData,
    },
};
pub fn is_separate_authn_supported_connector(connector: router_types::Connector) -> bool {
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
        | api_models::enums::Connector::Tokenex => false,
        api_models::enums::Connector::Stripe
        | api_models::enums::Connector::Checkout
        | api_models::enums::Connector::Cybersource
        | api_models::enums::Connector::Noon => true,
    }
}

pub fn is_separate_authn_supported(connector_call_type: &ConnectorCallType) -> bool {
    match connector_call_type {
        ConnectorCallType::PreDetermined(connector_data) => {
            is_separate_authn_supported_connector(connector_data.connector_name)
        }
        ConnectorCallType::Retryable(connectors) => connectors
            .first()
            .map(|connector_data| {
                is_separate_authn_supported_connector(connector_data.connector_name)
            })
            .unwrap_or(false),
        ConnectorCallType::SessionMultiple(_) => false,
    }
}

pub fn construct_router_data<F: Clone, Req, Res>(
    payment_id: Option<String>,
    attempt_id: Option<String>,
    merchant_id: Option<String>,
    address: Option<PaymentAddress>,
    request_data: Req,
    response_data: Res,
    merchant_connector_account: &domain::MerchantConnectorAccount,
) -> RouterResult<RouterData<F, Req, Res>> {
    let auth_type: ConnectorAuthType = merchant_connector_account
        .connector_account_details
        .clone()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)?;
    let empty_string = String::new();
    Ok(RouterData {
        flow: std::marker::PhantomData,
        merchant_id: merchant_id.unwrap_or(empty_string.clone()),
        customer_id: None,
        connector_customer: None,
        connector: merchant_connector_account.connector_name.clone(),
        payment_id: payment_id.unwrap_or(empty_string.clone()),
        attempt_id: attempt_id.unwrap_or(empty_string),
        status: storage_enums::AttemptStatus::Pending,
        payment_method: common_enums::PaymentMethod::Card,
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        address: address.unwrap_or_default(),
        auth_type: storage_enums::AuthenticationType::NoThreeDs,
        connector_meta_data: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: request_data,
        response: Ok(response_data),
        payment_method_id: None,
        connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
        payout_method_data: None,
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
    })
}

pub async fn update_trackers<F: Clone, Req>(
    state: &AppState,
    router_data: RouterData<F, Req, AuthenticationResponseData>,
    authentication: storage::Authentication,
    token: Option<String>,
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

    AuthenticationData::default();
    let authentication_update = match router_data.response {
        Ok(response) => Some(match response {
            AuthenticationResponseData::PreAuthNResponse {
                threeds_server_transaction_id,
                maximum_supported_3ds_version,
                connector_authentication_id,
                three_ds_method_data,
                three_ds_method_url,
                message_version,
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

                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::<AuthenticationData>::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: Some(connector_authentication_id),
                    payment_method_id: token,
                    authentication_type: None,
                    authentication_status: Some(common_enums::AuthenticationStatus::Started),
                    lifecycle_status: None,
                }
            }
            AuthenticationResponseData::AuthNResponse { authn_flow_type } => {
                authentication_data.authn_flow_type = Some(authn_flow_type);
                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::<AuthenticationData>::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: None,
                    payment_method_id: None,
                    authentication_type: None,
                    authentication_status: None,
                    lifecycle_status: None,
                }
            }
            AuthenticationResponseData::PostAuthNResponse { cavv } => {
                authentication_data.cavv = Some(cavv);
                storage::AuthenticationUpdate::AuthenticationDataUpdate {
                    authentication_data: Some(
                        Encode::<AuthenticationData>::encode_to_value(&authentication_data)
                            .change_context(ApiErrorResponse::InternalServerError)?,
                    ),
                    connector_authentication_id: None,
                    payment_method_id: None,
                    authentication_type: None,
                    authentication_status: Some(common_enums::AuthenticationStatus::Success),
                    lifecycle_status: None,
                }
            }
        }),
        Err(_error) => Some(storage::AuthenticationUpdate::AuthenticationDataUpdate {
            authentication_data: None,
            connector_authentication_id: None,
            payment_method_id: None,
            authentication_type: None,
            authentication_status: Some(common_enums::AuthenticationStatus::Failed),
            lifecycle_status: None,
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
