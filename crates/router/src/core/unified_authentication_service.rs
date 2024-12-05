pub mod transformers;
pub mod types;
pub mod utils;

use diesel_models::authentication::Authentication;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_request_types::unified_authentication_service::{
        UasAuthenticationResponseData, UasPostAuthenticationRequestData,
        UasPreAuthenticationRequestData,
    },
};

use super::{errors::RouterResult, payments::helpers::MerchantConnectorAccountType};
use crate::{
    core::{
        payments::PaymentData,
        unified_authentication_service::types::{
            ClickToPay, UnifiedAuthenticationService, UNIFIED_AUTHENTICATION_SERVICE,
        },
    },
    db::domain,
    routes::SessionState,
    types::api,
};

#[cfg(feature = "v1")]
#[async_trait::async_trait]
impl<F: Clone + Send> UnifiedAuthenticationService<F> for ClickToPay {
    async fn pre_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        business_profile: &domain::Profile,
        payment_data: &mut PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        connector_name: &str,
    ) -> RouterResult<Authentication> {
        let pre_authentication_data =
            UasPreAuthenticationRequestData::try_from(payment_data.clone())?;
        let payment_method = payment_data.payment_attempt.payment_method.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method",
            },
        )?;

        let connector_transaction_id = merchant_connector_account
            .get_mca_id()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Error while finding mca_id from merchant_connector_account")?;

        let store_authentication_in_db = utils::create_new_authentication(
            state,
            payment_data.payment_attempt.merchant_id.clone(),
            connector_name.to_string(),
            business_profile.get_id().clone(),
            Some(payment_data.payment_intent.get_id().clone()),
            connector_transaction_id,
        )
        .await?;

        let pre_auth_router_data: api::unified_authentication_service::UasPreAuthenticationRouterData = utils::construct_uas_router_data(
            connector_name.to_string(),
            payment_method,
            payment_data.payment_attempt.merchant_id.clone(),
            None,
            pre_authentication_data,
            merchant_connector_account,
            Some(store_authentication_in_db.authentication_id.clone()),
        )?;
        payment_data.payment_attempt.authentication_id =
            Some(store_authentication_in_db.authentication_id.clone());

        let response = utils::do_auth_connector_call(
            state,
            UNIFIED_AUTHENTICATION_SERVICE.to_string(),
            pre_auth_router_data,
        )
        .await?;

        let upadated_authentication =
            utils::update_trackers(state, response.clone(), store_authentication_in_db).await?;

        Ok(upadated_authentication)
    }

    async fn post_authentication(
        state: &SessionState,
        _key_store: &domain::MerchantKeyStore,
        _business_profile: &domain::Profile,
        payment_data: &mut PaymentData<F>,
        merchant_connector_account: &MerchantConnectorAccountType,
        authentication: Option<Authentication>,
        connector_name: &str,
    ) -> RouterResult<Authentication> {
        let payment_method = payment_data.payment_attempt.payment_method.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method",
            },
        )?;

        let post_authentication_data = UasPostAuthenticationRequestData;
        let authentication_id = payment_data
            .payment_attempt
            .authentication_id
            .clone()
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication id in payment attempt")?;

        let post_auth_router_data: api::unified_authentication_service::UasPostAuthenticationRouterData = utils::construct_uas_router_data(
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

        let network_token = match response.response.clone() {
            Ok(UasAuthenticationResponseData::PostAuthentication {
                authentication_details,
            }) => Some(
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
            ),
            _ => None,
        };

        payment_data.payment_method_data =
            network_token.map(domain::PaymentMethodData::NetworkToken);

        let previous_authentication_state = authentication
            .ok_or(ApiErrorResponse::InternalServerError)
            .attach_printable("Missing authentication table details after pre_authentication")?;

        let updated_authentication =
            utils::update_trackers(state, response.clone(), previous_authentication_state).await?;

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
