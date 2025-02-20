use async_trait::async_trait;
use common_enums as enums;
use hyperswitch_domain_models::errors::api_error_response::ApiErrorResponse;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payments::PaymentConfirmData;
use masking::ExposeInterface;

// use router_env::tracing::Instrument;
use super::{ConstructFlowSpecificData, Feature};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        mandate,
        payments::{
            self, access_token, customers, helpers, tokenization, transformers, PaymentData,
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

#[cfg(feature = "v2")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentConfirmData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &domain::MerchantConnectorAccount,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data_for_authorize(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        let payment_method = &self
            .payment_attempt
            .get_payment_method()
            .get_required_value("PaymentMethod")?;

        let data = if *payment_method == enums::PaymentMethod::OpenBanking {
            payments::get_merchant_bank_data_for_open_banking_connectors(
                merchant_connector_account,
                key_store,
                connector,
                state,
                merchant_account,
            )
            .await?
        } else {
            None
        };

        Ok(data)
    }
}

#[cfg(feature = "v1")]
#[async_trait]
impl
    ConstructFlowSpecificData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for PaymentData<api::Authorize>
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        merchant_recipient_data: Option<types::MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<
        types::RouterData<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    > {
        Box::pin(transformers::construct_payment_router_data::<
            api::Authorize,
            types::PaymentsAuthorizeData,
        >(
            state,
            self.clone(),
            connector_id,
            merchant_account,
            key_store,
            customer,
            merchant_connector_account,
            merchant_recipient_data,
            header_payload,
        ))
        .await
    }

    async fn get_merchant_recipient_data<'a>(
        &self,
        state: &SessionState,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<types::MerchantRecipientData>> {
        let payment_method = &self
            .payment_attempt
            .get_payment_method()
            .get_required_value("PaymentMethod")?;

        let data = if *payment_method == enums::PaymentMethod::OpenBanking {
            payments::get_merchant_bank_data_for_open_banking_connectors(
                merchant_connector_account,
                key_store,
                connector,
                state,
                merchant_account,
            )
            .await?
        } else {
            None
        };

        Ok(data)
    }
}

#[async_trait]
impl Feature<api::Authorize, types::PaymentsAuthorizeData> for types::PaymentsAuthorizeRouterData {
    async fn decide_flows<'a>(
        mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
        connector_request: Option<services::Request>,
        business_profile: &domain::Profile,
        header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    ) -> RouterResult<Self> {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        if self.should_proceed_with_authorize() {
            self.decide_authentication_type();
            logger::debug!(auth_type=?self.auth_type);
            let mut auth_router_data = services::execute_connector_processing_step(
                state,
                connector_integration,
                &self,
                call_connector_action.clone(),
                connector_request,
            )
            .await
            .to_payment_failed_response()?;

            // Initiating Integrity check
            let integrity_result = helpers::check_integrity_based_on_flow(
                &auth_router_data.request,
                &auth_router_data.response,
            );
            auth_router_data.integrity_check = integrity_result;
            metrics::PAYMENT_COUNT.add(1, &[]); // Move outside of the if block

            match auth_router_data.response.clone() {
                Err(_) => Ok(auth_router_data),
                Ok(authorize_response) => {
                    // Check if the Capture API should be called based on the connector and other parameters
                    if super::should_initiate_capture_flow(
                        &connector.connector_name,
                        self.request.customer_acceptance,
                        self.request.capture_method,
                        self.request.setup_future_usage,
                        auth_router_data.status,
                    ) {
                        auth_router_data = Box::pin(process_capture_flow(
                            auth_router_data,
                            authorize_response,
                            state,
                            connector,
                            call_connector_action.clone(),
                            business_profile,
                            header_payload,
                        ))
                        .await?;
                    }
                    Ok(auth_router_data)
                }
            }
        } else {
            Ok(self.clone())
        }
    }

    async fn add_access_token<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
        merchant_account: &domain::MerchantAccount,
        creds_identifier: Option<&str>,
    ) -> RouterResult<types::AddAccessTokenResult> {
        access_token::add_access_token(state, connector, merchant_account, self, creds_identifier)
            .await
    }

    async fn add_session_token<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self>
    where
        Self: Sized,
    {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::AuthorizeSessionToken,
            types::AuthorizeSessionTokenData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();
        let authorize_data = &types::PaymentsAuthorizeSessionTokenRouterData::foreign_from((
            &self,
            types::AuthorizeSessionTokenData::foreign_from(&self),
        ));
        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            authorize_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payment_failed_response()?;
        let mut router_data = self;
        router_data.session_token = resp.session_token;
        Ok(router_data)
    }

    async fn add_payment_method_token<'a>(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        tokenization_action: &payments::TokenizationAction,
        should_continue_payment: bool,
    ) -> RouterResult<types::PaymentMethodTokenResult> {
        let request = self.request.clone();
        tokenization::add_payment_method_token(
            state,
            connector,
            tokenization_action,
            self,
            types::PaymentMethodTokenizationData::try_from(request)?,
            should_continue_payment,
        )
        .await
    }

    async fn preprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_preprocessing_steps(state, &self, true, connector).await
    }

    async fn postprocessing_steps<'a>(
        self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Self> {
        authorize_postprocessing_steps(state, &self, true, connector).await
    }

    async fn create_connector_customer<'a>(
        &self,
        state: &SessionState,
        connector: &api::ConnectorData,
    ) -> RouterResult<Option<String>> {
        customers::create_connector_customer(
            state,
            connector,
            self,
            types::ConnectorCustomerData::try_from(self)?,
        )
        .await
    }

    async fn build_flow_specific_connector_request(
        &mut self,
        state: &SessionState,
        connector: &api::ConnectorData,
        call_connector_action: payments::CallConnectorAction,
    ) -> RouterResult<(Option<services::Request>, bool)> {
        match call_connector_action {
            payments::CallConnectorAction::Trigger => {
                connector
                    .connector
                    .validate_connector_against_payment_request(
                        self.request.capture_method,
                        self.payment_method,
                        self.request.payment_method_type,
                    )
                    .to_payment_failed_response()?;

                if crate::connector::utils::PaymentsAuthorizeRequestData::is_customer_initiated_mandate_payment(
                    &self.request,
                ) {
                    connector
                        .connector
                        .validate_mandate_payment(
                            self.request.payment_method_type,
                            self.request.payment_method_data.clone(),
                        )
                        .to_payment_failed_response()?;
                }

                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::Authorize,
                    types::PaymentsAuthorizeData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                metrics::EXECUTE_PRETASK_COUNT.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("flow", format!("{:?}", api::Authorize)),
                    ),
                );

                logger::debug!(completed_pre_tasks=?true);

                if self.should_proceed_with_authorize() {
                    self.decide_authentication_type();
                    logger::debug!(auth_type=?self.auth_type);

                    Ok((
                        connector_integration
                            .build_request(self, &state.conf.connectors)
                            .to_payment_failed_response()?,
                        true,
                    ))
                } else {
                    Ok((None, false))
                }
            }
            _ => Ok((None, true)),
        }
    }
}

pub trait RouterDataAuthorize {
    fn decide_authentication_type(&mut self);

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool;
}

impl RouterDataAuthorize for types::PaymentsAuthorizeRouterData {
    fn decide_authentication_type(&mut self) {
        if let hyperswitch_domain_models::payment_method_data::PaymentMethodData::Wallet(
            hyperswitch_domain_models::payment_method_data::WalletData::GooglePay(google_pay_data),
        ) = &self.request.payment_method_data
        {
            if let Some(assurance_details) = google_pay_data.info.assurance_details.as_ref() {
                // Step up the transaction to 3DS when either assurance_details.card_holder_authenticated or assurance_details.account_verified is false
                if !assurance_details.card_holder_authenticated
                    || !assurance_details.account_verified
                {
                    logger::info!("Googlepay transaction stepped up to 3DS");
                    self.auth_type = diesel_models::enums::AuthenticationType::ThreeDs;
                }
            }
        }
        if self.auth_type == diesel_models::enums::AuthenticationType::ThreeDs
            && !self.request.enrolled_for_3ds
        {
            self.auth_type = diesel_models::enums::AuthenticationType::NoThreeDs
        }
    }

    /// to decide if we need to proceed with authorize or not, Eg: If any of the pretask returns `redirection_response` then we should not proceed with authorize call
    fn should_proceed_with_authorize(&self) -> bool {
        match &self.response {
            Ok(types::PaymentsResponseData::TransactionResponse {
                redirection_data, ..
            }) => !redirection_data.is_some(),
            _ => true,
        }
    }
}

impl mandate::MandateBehaviour for types::PaymentsAuthorizeData {
    fn get_amount(&self) -> i64 {
        self.amount
    }
    fn get_mandate_id(&self) -> Option<&api_models::payments::MandateIds> {
        self.mandate_id.as_ref()
    }
    fn get_payment_method_data(&self) -> domain::payments::PaymentMethodData {
        self.payment_method_data.clone()
    }
    fn get_setup_future_usage(&self) -> Option<diesel_models::enums::FutureUsage> {
        self.setup_future_usage
    }
    fn get_setup_mandate_details(
        &self,
    ) -> Option<&hyperswitch_domain_models::mandates::MandateData> {
        self.setup_mandate_details.as_ref()
    }

    fn set_mandate_id(&mut self, new_mandate_id: Option<api_models::payments::MandateIds>) {
        self.mandate_id = new_mandate_id;
    }
    fn get_customer_acceptance(&self) -> Option<api_models::payments::CustomerAcceptance> {
        self.customer_acceptance.clone().map(From::from)
    }
}

pub async fn authorize_preprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PreProcessing,
            types::PaymentsPreProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let preprocessing_request_data =
            types::PaymentsPreProcessingData::try_from(router_data.request.to_owned())?;

        let preprocessing_response_data: Result<types::PaymentsResponseData, types::ErrorResponse> =
            Err(types::ErrorResponse::default());

        let preprocessing_router_data =
            helpers::router_data_type_conversion::<_, api::PreProcessing, _, _, _, _>(
                router_data.clone(),
                preprocessing_request_data,
                preprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &preprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payment_failed_response()?;

        metrics::PREPROCESSING_STEPS_COUNT.add(
            1,
            router_env::metric_attributes!(
                ("connector", connector.connector_name.to_string()),
                ("payment_method", router_data.payment_method.to_string()),
                (
                    "payment_method_type",
                    router_data
                        .request
                        .payment_method_type
                        .map(|inner| inner.to_string())
                        .unwrap_or("null".to_string()),
                ),
            ),
        );
        let mut authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response.clone(),
        );
        if connector.connector_name == api_models::enums::Connector::Airwallex {
            authorize_router_data.reference_id = resp.reference_id;
        } else if connector.connector_name == api_models::enums::Connector::Nuvei {
            let (enrolled_for_3ds, related_transaction_id) = match &authorize_router_data.response {
                Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse {
                    enrolled_v2,
                    related_transaction_id,
                }) => (*enrolled_v2, related_transaction_id.clone()),
                _ => (false, None),
            };
            authorize_router_data.request.enrolled_for_3ds = enrolled_for_3ds;
            authorize_router_data.request.related_transaction_id = related_transaction_id;
        } else if connector.connector_name == api_models::enums::Connector::Shift4 {
            if resp.request.enrolled_for_3ds {
                authorize_router_data.response = resp.response;
                authorize_router_data.status = resp.status;
            } else {
                authorize_router_data.request.enrolled_for_3ds = false;
            }
        }
        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

pub async fn authorize_postprocessing_steps<F: Clone>(
    state: &SessionState,
    router_data: &types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    confirm: bool,
    connector: &api::ConnectorData,
) -> RouterResult<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>> {
    if confirm {
        let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
            api::PostProcessing,
            types::PaymentsPostProcessingData,
            types::PaymentsResponseData,
        > = connector.connector.get_connector_integration();

        let postprocessing_request_data =
            types::PaymentsPostProcessingData::try_from(router_data.to_owned())?;

        let postprocessing_response_data: Result<
            types::PaymentsResponseData,
            types::ErrorResponse,
        > = Err(types::ErrorResponse::default());

        let postprocessing_router_data =
            helpers::router_data_type_conversion::<_, api::PostProcessing, _, _, _, _>(
                router_data.clone(),
                postprocessing_request_data,
                postprocessing_response_data,
            );

        let resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &postprocessing_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await
        .to_payment_failed_response()?;

        let authorize_router_data = helpers::router_data_type_conversion::<_, F, _, _, _, _>(
            resp.clone(),
            router_data.request.to_owned(),
            resp.response,
        );

        Ok(authorize_router_data)
    } else {
        Ok(router_data.clone())
    }
}

impl<F>
    ForeignTryFrom<types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>>
    for types::PaymentsCaptureData
{
    type Error = error_stack::Report<ApiErrorResponse>;

    fn foreign_try_from(
        item: types::RouterData<F, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let response = item
            .response
            .map_err(|err| ApiErrorResponse::ExternalConnectorError {
                code: err.code,
                message: err.message,
                connector: item.connector.clone(),
                status_code: err.status_code,
                reason: err.reason,
            })?;

        Ok(Self {
            amount_to_capture: item.request.amount,
            currency: item.request.currency,
            connector_transaction_id: types::PaymentsResponseData::get_connector_transaction_id(
                &response,
            )?,
            payment_amount: item.request.amount,
            multiple_capture_data: None,
            connector_meta: types::PaymentsResponseData::get_connector_metadata(&response)
                .map(|secret| secret.expose()),
            browser_info: None,
            metadata: None,
            capture_method: item.request.capture_method,
            minor_payment_amount: item.request.minor_amount,
            minor_amount_to_capture: item.request.minor_amount,
            integrity_object: None,
            webhook_url: item.request.webhook_url,
        })
    }
}

async fn process_capture_flow(
    mut router_data: types::RouterData<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    >,
    authorize_response: types::PaymentsResponseData,
    state: &SessionState,
    connector: &api::ConnectorData,
    call_connector_action: payments::CallConnectorAction,
    business_profile: &domain::Profile,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResult<
    types::RouterData<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>,
> {
    // Convert RouterData into Capture RouterData
    let capture_router_data = helpers::router_data_type_conversion(
        router_data.clone(),
        types::PaymentsCaptureData::foreign_try_from(router_data.clone())?,
        Err(types::ErrorResponse::default()),
    );

    // Call capture request
    let post_capture_router_data = super::call_capture_request(
        capture_router_data,
        state,
        connector,
        call_connector_action,
        business_profile,
        header_payload,
    )
    .await;

    // Process capture response
    let (updated_status, updated_response) =
        super::handle_post_capture_response(authorize_response, post_capture_router_data)?;
    router_data.status = updated_status;
    router_data.response = Ok(updated_response);
    Ok(router_data)
}
