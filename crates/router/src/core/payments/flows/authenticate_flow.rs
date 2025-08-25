use async_trait::async_trait;
use common_enums as enums;
use common_types::payments as common_payments_types;
use error_stack::ResultExt;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_data::RouterData,
    router_flow_types::{Authenticate, PreAuthenticate},
    router_request_types::PaymentsAuthorizeData,
    router_response_types::PaymentsResponseData,
};
use masking::{ExposeInterface, Secret};
use unified_connector_service_client::payments as payments_grpc;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
        },
        unified_connector_service::{
            build_unified_connector_service_auth_metadata,
            handle_unified_connector_service_response_for_payment_authorize,
        },
    },
    logger,
    routes::{metrics, SessionState},
    services::{self, api::ConnectorValidation},
    types::{
        self, api, domain,
        transformers::{ForeignFrom, ForeignTryFrom},
    },
    utils::OptionExt,
};

#[async_trait]
impl Feature<Authenticate, types::PaymentsAuthenticateData>
    for RouterData<Authenticate, types::PaymentsAuthenticateData, PaymentsResponseData>
{
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
        return_raw_connector_response: Option<bool>,
    ) -> RouterResult<Self> {
        todo!()
        // let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        //     Authenticate,
        //     types::PaymentsAuthenticateData,
        //     types::PaymentsResponseData,
        // > = connector.connector.get_connector_integration();

        // // if self.should_proceed_with_authorize() {
        // //     self.decide_authentication_type();
        // //     logger::debug!(auth_type=?self.auth_type);
        // let mut auth_router_data = services::execute_connector_processing_step(
        //     state,
        //     connector_integration,
        //     &self,
        //     call_connector_action.clone(),
        //     connector_request,
        //     return_raw_connector_response,
        // )
        // .await
        // .to_payment_failed_response()?;

        // // Initiating Integrity check
        // let integrity_result = helpers::check_integrity_based_on_flow(
        //     &auth_router_data.request,
        //     &auth_router_data.response,
        // );
        // auth_router_data.integrity_check = integrity_result;
        // metrics::PAYMENT_COUNT.add(1, &[]); // Move outside of the if block

        // match auth_router_data.response.clone() {
        //     Err(_) => Ok(auth_router_data),
        //     Ok(authorize_response) => {
        //         // Check if the Capture API should be called based on the connector and other parameters
        //         // if super::should_initiate_capture_flow(
        //         //     &connector.connector_name,
        //         //     self.request.customer_acceptance,
        //         //     self.request.capture_method,
        //         //     self.request.setup_future_usage,
        //         //     auth_router_data.status,
        //         // ) {
        //         //     auth_router_data = Box::pin(process_capture_flow(
        //         //         auth_router_data,
        //         //         authorize_response,
        //         //         state,
        //         //         connector,
        //         //         call_connector_action.clone(),
        //         //         business_profile,
        //         //         header_payload,
        //         //     ))
        //         //     .await?;
        //         // }
        //         Ok(auth_router_data)
        //     }
        // }
        // // } else {
        // //     Ok(self.clone())
        // // }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_context: &domain::MerchantContext,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_context, self, creds_identifier)
            .await
    }

    // async fn add_session_token<'a>(
    //     self,
    //     state: &SessionState,
    //     connector: &api::ConnectorData,
    // ) -> RouterResult<Self>
    // where
    //     Self: Sized,
    // {
    //     Ok(router_data)
    // }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        todo!()
        // let request = self.request.clone();
        // tokenization::add_payment_method_token(
        //     state,
        //     connector,
        //     tokenization_action,
        //     self,
        //     types::PaymentMethodTokenizationData::try_from(request)?,
        //     should_continue_payment,
        // )
        // .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        todo!()
        // match call_connector_action {
        //     payments::CallConnectorAction::Trigger => {
        //         connector
        //             .connector
        //             .validate_connector_against_payment_request(
        //                 self.request.capture_method,
        //                 self.payment_method,
        //                 self.request.payment_method_type,
        //             )
        //             .to_payment_failed_response()?;

        //         // Check if the connector supports mandate payment
        //         // if the payment_method_type does not support mandate for the given connector, downgrade the setup future usage to on session
        //         if self.request.setup_future_usage
        //             == Some(diesel_models::enums::FutureUsage::OffSession)
        //             && !self
        //                 .request
        //                 .payment_method_type
        //                 .and_then(|payment_method_type| {
        //                     state
        //                         .conf
        //                         .mandates
        //                         .supported_payment_methods
        //                         .0
        //                         .get(&enums::PaymentMethod::from(payment_method_type))
        //                         .and_then(|supported_pm_for_mandates| {
        //                             supported_pm_for_mandates.0.get(&payment_method_type).map(
        //                                 |supported_connector_for_mandates| {
        //                                     supported_connector_for_mandates
        //                                         .connector_list
        //                                         .contains(&connector.connector_name)
        //                                 },
        //                             )
        //                         })
        //                 })
        //                 .unwrap_or(false)
        //         {
        //             // downgrade the setup future usage to on session
        //             self.request.setup_future_usage =
        //                 Some(diesel_models::enums::FutureUsage::OnSession);
        //         };

        //         if crate::connector::utils::PaymentsAuthorizeRequestData::is_customer_initiated_mandate_payment(
        //             &self.request,
        //         ) {
        //             connector
        //                 .connector
        //                 .validate_mandate_payment(
        //                     self.request.payment_method_type,
        //                     self.request.payment_method_data.clone(),
        //                 )
        //                 .to_payment_failed_response()?;
        //         };

        //         let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        //             api::Authorize,
        //             types::PaymentsAuthorizeData,
        //             types::PaymentsResponseData,
        //         > = connector.connector.get_connector_integration();

        //         metrics::EXECUTE_PRETASK_COUNT.add(
        //             1,
        //             router_env::metric_attributes!(
        //                 ("connector", connector.connector_name.to_string()),
        //                 ("flow", format!("{:?}", api::Authorize)),
        //             ),
        //         );

        //         logger::debug!(completed_pre_tasks=?true);

        //         // if self.should_proceed_with_authorize() {
        //         //     self.decide_authentication_type();
        //         //     logger::debug!(auth_type=?self.auth_type);

        //         //     Ok((
        //         //         connector_integration
        //         //             .build_request(self, &state.conf.connectors)
        //         //             .to_payment_failed_response()?,
        //         //         true,
        //         //     ))
        //         // } else {
        //         Ok((None, false))
        //         // }
        //     }
        //     _ => Ok((None, true)),
        // }
    }

    async fn create_order_at_connector(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        should_continue_payment: bool,
    ) -> RouterResult<Option<types::CreateOrderResult>> {
        todo!()
        // if connector
        //     .connector_name
        //     .requires_order_creation_before_payment(self.payment_method)
        //     && should_continue_payment
        // {
        //     let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        //         api::CreateOrder,
        //         types::CreateOrderRequestData,
        //         types::PaymentsResponseData,
        //     > = connector.connector.get_connector_integration();

        //     let request_data = types::CreateOrderRequestData::try_from(self.request.clone())?;

        //     let response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
        //         Err(types::ErrorResponse::default());

        //     let createorder_router_data =
        //         helpers::router_data_type_conversion::<_, api::CreateOrder, _, _, _, _>(
        //             self.clone(),
        //             request_data,
        //             response_data,
        //         );

        //     let resp = services::execute_connector_processing_step(
        //         state,
        //         connector_integration,
        //         &createorder_router_data,
        //         payments::CallConnectorAction::Trigger,
        //         None,
        //         None,
        //     )
        //     .await
        //     .to_payment_failed_response()?;

        //     let create_order_resp = match resp.response {
        //         Ok(res) => {
        //             if let types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id } =
        //                 res
        //             {
        //                 Ok(order_id)
        //             } else {
        //                 Err(error_stack::report!(ApiErrorResponse::InternalServerError)
        //                     .attach_printable(format!(
        //                         "Unexpected response format from connector: {res:?}",
        //                     )))?
        //             }
        //         }
        //         Err(error) => Err(error),
        //     };

        //     Ok(Some(types::CreateOrderResult {
        //         create_order_result: create_order_resp,
        //     }))
        // } else {
        //     // If the connector does not require order creation, return None
        //     Ok(None)
        // }
    }

    fn update_router_data_with_create_order_response(
        &mut self,
        create_order_result: types::CreateOrderResult,
    ) {
        todo!()
        // match create_order_result.create_order_result {
        //     Ok(order_id) => {
        //         self.request.order_id = Some(order_id.clone()); // ? why this is assigned here and ucs also wants this to populate data
        //         self.response =
        //             Ok(types::PaymentsResponseData::PaymentsCreateOrderResponse { order_id });
        //     }
        //     Err(err) => {
        //         self.response = Err(err.clone());
        //     }
        // }
    }

    async fn call_unified_connector_service<'a>(
        &mut self,
        state: &SessionState,
        #[cfg(feature = "v1")] merchant_connector_account: helpers::MerchantConnectorAccountType,
        #[cfg(feature = "v2")]
        merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
        merchant_context: &domain::MerchantContext,
    ) -> RouterResult<()> {
        todo!()
        // let client = state
        //     .grpc_client
        //     .unified_connector_service_client
        //     .clone()
        //     .ok_or(ApiErrorResponse::InternalServerError)
        //     .attach_printable("Failed to fetch Unified Connector Service client")?;

        // let payment_authorize_request =
        //     payments_grpc::PaymentServiceAuthorizeRequest::foreign_try_from(self)
        //         .change_context(ApiErrorResponse::InternalServerError)
        //         .attach_printable("Failed to construct Payment Authorize Request")?;

        // let connector_auth_metadata = build_unified_connector_service_auth_metadata(
        //     merchant_connector_account,
        //     merchant_context,
        // )
        // .change_context(ApiErrorResponse::InternalServerError)
        // .attach_printable("Failed to construct request metadata")?;

        // let response = client
        //     .payment_authorize(
        //         payment_authorize_request,
        //         connector_auth_metadata,
        //         state.get_grpc_headers(),
        //     )
        //     .await
        //     .change_context(ApiErrorResponse::InternalServerError)
        //     .attach_printable("Failed to authorize payment")?;

        // let payment_authorize_response = response.into_inner();

        // let (status, router_data_response) =
        //     handle_unified_connector_service_response_for_payment_authorize(
        //         payment_authorize_response.clone(),
        //     )
        //     .change_context(ApiErrorResponse::InternalServerError)
        //     .attach_printable("Failed to deserialize UCS response")?;

        // self.status = status;
        // self.response = router_data_response;
        // self.raw_connector_response = payment_authorize_response
        //     .raw_connector_response
        //     .map(Secret::new);

        // Ok(())
        // call_ucs_for_authenticate
    }
}

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        hyperswitch_domain_models::router_flow_types::Authenticate,
        types::PaymentsAuthenticateData,
        types::PaymentsResponseData,
    > for PaymentConfirmData<hyperswitch_domain_models::router_flow_types::Authenticate>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_context: &domain::MerchantContext,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            hyperswitch_domain_models::router_flow_types::Authenticate,
            types::PaymentsAuthenticateData,
            types::PaymentsResponseData,
        >,
    > {
        todo!()
        // Box::pin(transformers::construct_payment_router_data_for_authorize(
        //     state,
        //     self.clone(),
        //     connector_id,
        //     merchant_context,
        //     customer,
        //     merchant_connector_account,
        //     merchant_recipient_data,
        //     header_payload,
        // ))
        // .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        state: &SessionState,
        merchant_context: &domain::MerchantContext,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        let is_open_banking = &self
            .payment_attempt
            .get_payment_method()
            .get_required_value("PaymentMethod")?
            .eq(&enums::PaymentMethod::OpenBanking);

        if *is_open_banking {
            payments::get_merchant_bank_data_for_open_banking_connectors(
                merchant_connector_account,
                merchant_context,
                connector,
                state,
            )
            .await
        } else {
            Ok(None)
        }
    }
}
