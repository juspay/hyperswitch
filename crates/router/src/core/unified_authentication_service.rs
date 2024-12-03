use std::marker::PhantomData;

use common_enums::PaymentMethod;
use common_utils::{
    ext_traits::ValueExt,
    types::{AmountConvertor, FloatMajorUnitForConnector},
};
use diesel_models::authentication::{Authentication, AuthenticationNew, AuthenticationUpdate};
use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    payment_address::PaymentAddress,
    router_data::{ConnectorAuthType, ErrorResponse},
    router_data_v2::UasFlowData,
    router_request_types::unified_authentication_service::{
        ServiceDetails, ServiceSessionIds, TransactionDetails, UasAuthenticationResponseData,
        UasPostAuthenticationRequestData, UasPreAuthenticationRequestData,
    },
};

use super::{
    errors::{ConnectorErrorExt, RouterResult, StorageErrorExt},
    payments,
    payments::helpers::MerchantConnectorAccountType,
};
use crate::{
    consts,
    core::payments::PaymentData,
    db::domain,
    routes::SessionState,
    services,
    services::execute_connector_processing_step,
    types::{api, RouterData},
};

const IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_attempt_id_in_AUTHENTICATION_flow";

const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_AUTHENTICATION_flow";

#[async_trait::async_trait]
pub trait UnifiedAuthenticationService<F: Clone> {
    async fn pre_authentication(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &mut PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
    ) -> RouterResult<Authentication>;

    async fn post_authentication(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &mut PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
        _authentication: Option<Authentication>,
    ) -> RouterResult<Authentication>;

    fn confirmation(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &mut PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
    ) -> RouterResult<()>;
}

pub struct ClickToPay;

#[async_trait::async_trait]
impl<F: Clone + Send> UnifiedAuthenticationService<F> for ClickToPay {
    async fn pre_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        business_profile: &domain::Profile,
        payment_data: &mut PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
    ) -> RouterResult<Authentication> {
        let pre_authentication_data =
            UasPreAuthenticationRequestData::try_from(payment_data.clone())?;
        let payment_method = payment_data.payment_attempt.payment_method.clone().unwrap();
        let store_authentication_in_db: Authentication = create_new_authentication(
            state,
            payment_data.payment_attempt.merchant_id.clone(),
            "ctp_mastercard".to_string(),
            business_profile.get_id().clone(),
            Some(payment_data.payment_intent.get_id().clone()),
            merchant_connector_account.get_mca_id(),
        )
        .await?;
        let pre_auth_router_data: api::unified_authentication_service::UasPreAuthenticationRouterData = construct_uas_router_data(
            "ctp_mastercard".to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            pre_authentication_data,
            merchant_connector_account,
            Some(store_authentication_in_db.authentication_id.clone()),
        )?;
        payment_data.payment_attempt.authentication_id =
            Some(store_authentication_in_db.authentication_id.clone());

        let response = do_auth_connector_call(
            state,
            "unified_authentication_service".to_string(),
            pre_auth_router_data,
        )
        .await?;

        let upadated_authentication =
            update_trackers(state, response.clone(), store_authentication_in_db).await?;

        Ok(upadated_authentication)
    }

    async fn post_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &mut PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        authentication: Option<Authentication>,
    ) -> RouterResult<Authentication> {
        let payment_method = payment_data.payment_attempt.payment_method.clone().unwrap();
        let post_authentication_data = UasPostAuthenticationRequestData;
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let post_auth_router_data: api::unified_authentication_service::UasPostAuthenticationRouterData = construct_uas_router_data(
            "ctp_mastercard".to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            post_authentication_data,
            merchant_connector_account,
            Some(authentication_id.clone()),
        ).unwrap();

        let response = do_auth_connector_call(
            state,
            "unified_authentication_service".to_string(),
            post_auth_router_data,
        )
        .await?;

        let network_token = if let Ok(UasAuthenticationResponseData::PostAuthentication {
            authentication_details,
        }) = response.response.clone()
        {
            Some(
                hyperswitch_domain_models::payment_method_data::NetworkTokenData {
                    token_number: authentication_details.token_details.payment_token,
                    token_exp_month: authentication_details.token_details.token_expiration_month,
                    token_exp_year: authentication_details.token_details.token_expiration_year,
                    token_cryptogram: None,
                    card_issuer: None,
                    card_network: None,
                    card_type: None,
                    card_issuing_country: None,
                    bank_code: None,
                    nick_name: None,
                },
            )
        } else {
            None
        };

        payment_data.payment_method_data =
            network_token.map(|token| domain::PaymentMethodData::NetworkToken(token));

        let previous_authentication_state = authentication
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication table details after pre_authentication")?;

        let updated_authentication =
            update_trackers(state, response.clone(), previous_authentication_state).await?;

        Ok(updated_authentication)
    }

    fn confirmation(
        _state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        _payment_data: &mut PaymentData<F>,
        _merchant_connector_account: &MerchantConnectorAccountType,
    ) -> RouterResult<()> {
        Ok(())
    }
}

impl<F: Clone> TryFrom<PaymentData<F>> for UasPreAuthenticationRequestData {
    type Error = Report<ApiErrorResponse>;
    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let service_details = ServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: None,
                correlation_id: None,
                x_src_flow_id: None,
            }),
        };
        let required_conversion = FloatMajorUnitForConnector;

        let amount = required_conversion
            .convert(
                payment_data.payment_attempt.net_amount.get_order_amount(),
                payment_data.payment_attempt.currency.unwrap(),
            )
            .unwrap();
        let transaction_details = TransactionDetails {
            amount,
            currency: payment_data.payment_attempt.currency.unwrap(),
        };

        Ok(UasPreAuthenticationRequestData {
            service_details: Some(service_details),
            transaction_details: Some(transaction_details),
        })
    }
}

pub fn construct_uas_router_data<F: Clone, Req, Res>(
    authentication_connector_name: String,
    payment_method: PaymentMethod,
    merchant_id: common_utils::id_type::MerchantId,
    address: Option<PaymentAddress>,
    request_data: Req,
    merchant_connector_account: &MerchantConnectorAccountType,
    authentication_id: Option<String>,
) -> RouterResult<RouterData<F, Req, Res>> {
    let test_mode: Option<bool> = merchant_connector_account.is_test_mode_on();
    let auth_type: ConnectorAuthType = merchant_connector_account
        .get_connector_account_details()
        .parse_value("ConnectorAuthType")
        .change_context(ApiErrorResponse::InternalServerError)?;
    Ok(RouterData {
        flow: PhantomData,
        merchant_id,
        customer_id: None,
        connector_customer: None,
        connector: authentication_connector_name,
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("authentication")
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        status: common_enums::AttemptStatus::default(),
        payment_method,
        connector_auth_type: auth_type,
        description: None,
        return_url: None,
        address: address.unwrap_or_default(),
        auth_type: common_enums::AuthenticationType::default(),
        connector_meta_data: merchant_connector_account.get_metadata(),
        connector_wallets_details: merchant_connector_account.get_connector_wallets_details(),
        amount_captured: None,
        minor_amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: request_data,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_AUTHENTICATION_FLOW.to_owned(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payment_method_status: None,
        connector_response: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        authentication_id,
    })
}

pub async fn do_auth_connector_call<F, Req, Res>(
    state: &SessionState,
    authentication_connector_name: String,
    router_data: RouterData<F, Req, Res>,
) -> RouterResult<RouterData<F, Req, Res>>
where
    Req: std::fmt::Debug + Clone + 'static,
    Res: std::fmt::Debug + Clone + 'static,
    F: std::fmt::Debug + Clone + 'static,
    dyn api::Connector + Sync: services::api::ConnectorIntegration<F, Req, Res>,
    dyn api::ConnectorV2 + Sync: services::api::ConnectorIntegrationV2<F, UasFlowData, Req, Res>,
{
    let connector_data =
        api::AuthenticationConnectorData::get_connector_by_name(&authentication_connector_name)?;
    let connector_integration: services::BoxedUnifiedAuthenticationServiceInterface<F, Req, Res> =
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

pub async fn create_new_authentication(
    state: &SessionState,
    merchant_id: common_utils::id_type::MerchantId,
    authentication_connector: String,
    profile_id: common_utils::id_type::ProfileId,
    payment_id: Option<common_utils::id_type::PaymentId>,
    merchant_connector_id: Option<common_utils::id_type::MerchantConnectorAccountId>,
) -> RouterResult<Authentication> {
    let authentication_id =
        common_utils::generate_id_with_default_len(consts::AUTHENTICATION_ID_PREFIX);
    let new_authorization = AuthenticationNew {
        authentication_id: authentication_id.clone(),
        merchant_id,
        authentication_connector,
        connector_authentication_id: None,
        payment_method_id: format!("eph_"),
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
        acs_signed_content: None,
        profile_id,
        payment_id,
        merchant_connector_id: merchant_connector_id.unwrap(),
        ds_trans_id: None,
        directory_server_id: None,
        acquirer_country_code: None,
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

pub async fn update_trackers<F: Clone, Req>(
    state: &SessionState,
    router_data: RouterData<F, Req, UasAuthenticationResponseData>,
    authentication: Authentication,
) -> RouterResult<Authentication> {
    let authentication_update = match router_data.response {
        Ok(response) => match response {
            UasAuthenticationResponseData::PreAuthentication {} => {
                AuthenticationUpdate::AuthenticationStatusUpdate {
                    trans_status: common_enums::TransactionStatus::InformationOnly,
                    authentication_status: common_enums::AuthenticationStatus::Pending,
                }
            }
            UasAuthenticationResponseData::PostAuthentication {
                authentication_details,
            } => AuthenticationUpdate::PostAuthenticationUpdate {
                authentication_status: common_enums::AuthenticationStatus::Success,
                trans_status: common_enums::TransactionStatus::Success,
                authentication_value: authentication_details
                    .dynamic_data_details
                    .and_then(|data| data.dynamic_data_value),
                eci: authentication_details.eci,
            },
        },
        Err(error) => AuthenticationUpdate::ErrorUpdate {
            connector_authentication_id: error.connector_transaction_id,
            authentication_status: common_enums::AuthenticationStatus::Failed,
            error_message: error
                .reason
                .map(|reason| format!("message: {}, reason: {}", error.message, reason))
                .or(Some(error.message)),
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
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Error while updating authentication for uas")
}