use std::{fmt::Debug, marker::PhantomData, str::FromStr, time::Duration};

use async_trait::async_trait;
use common_utils::pii::Email;
use error_stack::Report;
use masking::Secret;
#[cfg(feature = "payouts")]
use router::core::utils as core_utils;
use router::{
    configs::settings::Settings,
    core::{errors, errors::ConnectorError, payments},
    db::StorageImpl,
    routes, services,
    types::{self, api, storage::enums, AccessToken, PaymentAddress, RouterData},
};
use test_utils::connector_auth::ConnectorAuthType;
use tokio::sync::oneshot;
use wiremock::{Mock, MockServer};

pub trait Connector {
    fn get_data(&self) -> types::api::ConnectorData;

    fn get_auth_token(&self) -> types::ConnectorAuthType;

    fn get_name(&self) -> String;

        /// This method returns the metadata of the connector.
    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        None
    }

    /// interval in seconds to be followed when making the subsequent request whenever needed
    fn get_request_interval(&self) -> u64 {
        5
    }

    #[cfg(feature = "payouts")]
        /// This method returns the payout data from the PayoutConnector. It returns an Option containing the PayoutConnectorData if it is available, otherwise it returns None.
    fn get_payout_data(&self) -> Option<types::api::PayoutConnectorData> {
            None
    }
}

#[derive(Debug, Default, Clone)]
pub struct PaymentInfo {
    pub address: Option<PaymentAddress>,
    pub auth_type: Option<enums::AuthenticationType>,
    pub access_token: Option<AccessToken>,
    pub connector_meta_data: Option<serde_json::Value>,
    pub return_url: Option<String>,
    pub connector_customer: Option<String>,
    pub payment_method_token: Option<String>,
    pub payout_method_data: Option<api::PayoutMethodData>,
    pub currency: Option<enums::Currency>,
    pub country: Option<enums::CountryAlpha2>,
}

#[async_trait]
pub trait ConnectorActions: Connector {
    /// For initiating payments when `CaptureMethod` is set to `Manual`
    /// This doesn't complete the transaction, `PaymentsCapture` needs to be done manually
    async fn authorize_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..(payment_data.unwrap_or(PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        integration.execute_pretasks(&mut request, &state).await?;
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously creates a connector customer with the provided payment data and payment info. It generates the necessary data, sets up a channel for communication, initializes the application state, executes pre-tasks using the connector integration, and then calls the connector to create the customer. Returns a Result containing the connector customer router data or a report of any connector errors encountered.
    async fn create_connector_customer(
        &self,
        payment_data: Option<types::ConnectorCustomerData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::ConnectorCustomerRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::ConnectorCustomerData {
                ..(payment_data.unwrap_or(CustomerType::default().0))
            },
            payment_info,
        );
        let tx: oneshot::Sender<()> = oneshot::channel().0;
    
        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        integration.execute_pretasks(&mut request, &state).await?;
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously creates a payment method token using the given payment data and payment info. This method retrieves the connector integration, generates the necessary data, creates a sender channel, initializes the application state, executes pre-tasks using the integration, and calls the connector to create the token. Returns a Result containing the tokenization router data or a report of any connector errors encountered during the process.
    async fn create_connector_pm_token(
        &self,
        payment_data: Option<types::PaymentMethodTokenizationData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::TokenizationRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentMethodTokenizationData {
                ..(payment_data.unwrap_or(TokenType::default().0))
            },
            payment_info,
        );
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        integration.execute_pretasks(&mut request, &state).await?;
        Box::pin(call_connector(request, integration)).await
    }

    /// For initiating payments when `CaptureMethod` is set to `Automatic`
    /// This does complete the transaction without user intervention to Capture the payment
    async fn make_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(diesel_models::enums::CaptureMethod::Automatic),
                ..(payment_data.unwrap_or(PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        integration.execute_pretasks(&mut request, &state).await?;
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously synchronizes a payment by calling the connector integration with the provided payment data and information. If the payment data is not provided, it defaults to the default payment sync type. Returns the result of the synchronization operation, wrapped in a Result type along with any potential errors reported as a ConnectorError.
    async fn sync_payment(
        &self,
        payment_data: Option<types::PaymentsSyncData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsSyncRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| PaymentSyncType::default().0),
            payment_info,
        );
        Box::pin(call_connector(request, integration)).await
    }

    /// will retry the psync till the given status matches or retry max 3 times
    async fn psync_retry_till_status_matches(
        &self,
        status: enums::AttemptStatus,
        payment_data: Option<types::PaymentsSyncData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsSyncRouterData, Report<ConnectorError>> {
        let max_tries = 3;
        for curr_try in 0..max_tries {
            let sync_res = self
                .sync_payment(payment_data.clone(), payment_info.clone())
                .await
                .unwrap();
            if (sync_res.status == status) || (curr_try == max_tries - 1) {
                return Ok(sync_res);
            }
            tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await;
        }
        Err(errors::ConnectorError::ProcessingStepFailed(None).into())
    }

        /// Asynchronously captures a payment with the provided transaction ID, payment data, and payment information. Returns a result containing the captured payment router data or a report of any connector errors encountered during the capture process.
    async fn capture_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::PaymentsCaptureData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCaptureRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            types::PaymentsCaptureData {
                connector_transaction_id: transaction_id,
                ..payment_data.unwrap_or(PaymentCaptureType::default().0)
            },
            payment_info,
        );
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously authorizes a payment using the provided authorize data and payment info,
    /// and then captures the payment using the returned transaction ID and capture data.
    ///
    /// # Arguments
    ///
    /// * `authorize_data` - Optional payment authorization data
    /// * `capture_data` - Optional payment capture data
    /// * `payment_info` - Optional payment information
    ///
    /// # Returns
    ///
    /// A Result containing the captured payment data or a ConnectorError report
    async fn authorize_and_capture_payment(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        capture_data: Option<types::PaymentsCaptureData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCaptureRouterData, Report<ConnectorError>> {
        let authorize_response = self
            .authorize_payment(authorize_data, payment_info.clone())
            .await
            .unwrap();
        assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
        let txn_id = get_connector_transaction_id(authorize_response.response);
        let response = self
            .capture_payment(txn_id.unwrap(), capture_data, payment_info)
            .await
            .unwrap();
        return Ok(response);
    }

        /// Asynchronously voids a payment by sending a request to the connector with the provided transaction ID, payment data, and payment information. Returns a result containing the payment cancellation router data or a connector error report.
    async fn void_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::PaymentsCancelData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCancelRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            types::PaymentsCancelData {
                connector_transaction_id: transaction_id,
                ..payment_data.unwrap_or(PaymentCancelType::default().0)
            },
            payment_info,
        );
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously authorizes a payment using the provided authorization data and voids the payment using the provided void data.
    /// 
    /// # Arguments
    /// 
    /// * `authorize_data` - Optional data required for authorizing the payment
    /// * `void_data` - Optional data required for voiding the payment
    /// * `payment_info` - Optional payment information
    /// 
    /// # Returns
    /// 
    /// Returns a `Result` containing the data of the voided payment or a `Report` of any `ConnectorError` encountered during the process.
    /// 
    async fn authorize_and_void_payment(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        void_data: Option<types::PaymentsCancelData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCancelRouterData, Report<ConnectorError>> {
        let authorize_response = self
            .authorize_payment(authorize_data, payment_info.clone())
            .await
            .unwrap();
        assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
        let txn_id = get_connector_transaction_id(authorize_response.response);
        tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await; // to avoid 404 error
        let response = self
            .void_payment(txn_id.unwrap(), void_data, payment_info)
            .await
            .unwrap();
        return Ok(response);
    }

        /// Asynchronously refunds a payment using the provided transaction ID, refund data, and payment information. Returns a Result containing the refund execution data or a ConnectorError report.
    async fn refund_payment(
        &self,
        transaction_id: String,
        refund_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            types::RefundsData {
                connector_transaction_id: transaction_id,
                ..refund_data.unwrap_or(PaymentRefundType::default().0)
            },
            payment_info,
        );
        Box::pin(call_connector(request, integration)).await
    }

        /// Asynchronously captures a payment, then attempts to refund the captured payment. 
    /// 
    /// # Arguments
    /// 
    /// * `authorize_data` - Optional payment authorization data
    /// * `capture_data` - Optional payment capture data
    /// * `refund_data` - Optional refund data
    /// * `payment_info` - Optional payment information
    /// 
    /// # Returns
    /// 
    /// A Result containing the refund execution data, or a Report of any connector errors
    /// 
    async fn capture_payment_and_refund(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        capture_data: Option<types::PaymentsCaptureData>,
        refund_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        //make a successful payment
        let response = self
            .authorize_and_capture_payment(authorize_data, capture_data, payment_info.clone())
            .await
            .unwrap();
        let txn_id = self.get_connector_transaction_id_from_capture_data(response);

        //try refund for previous payment
        tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await; // to avoid 404 error
        Ok(self
            .refund_payment(txn_id.unwrap(), refund_data, payment_info)
            .await
            .unwrap())
    }

        /// Make a payment with the provided authorize data and payment info, then attempt to refund the payment using the refund data and payment info.
    ///
    /// # Arguments
    ///
    /// * `authorize_data` - Optional payment authorization data
    /// * `refund_data` - Optional refund data
    /// * `payment_info` - Optional payment information
    ///
    /// # Returns
    ///
    /// A Result containing the refund execution data if successful, otherwise a Report of ConnectorError
    ///
    async fn make_payment_and_refund(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        refund_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        //make a successful payment
        let response = self
            .make_payment(authorize_data, payment_info.clone())
            .await
            .unwrap();

        //try refund for previous payment
        let transaction_id = get_connector_transaction_id(response.response).unwrap();
        tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await; // to avoid 404 error
        Ok(self
            .refund_payment(transaction_id, refund_data, payment_info)
            .await
            .unwrap())
    }

        /// Asynchronously authorizes, captures, and refunds a payment, returning the refund execution data.
    ///
    /// # Arguments
    ///
    /// * `authorize_data` - Optional payment authorization data.
    /// * `refund_data` - Optional refund data.
    /// * `payment_info` - Optional payment information.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the refund execution data or a `Report` of a `ConnectorError`.
    ///
    async fn auth_capture_and_refund(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        refund_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        //make a successful payment
        let response = self
            .authorize_and_capture_payment(authorize_data, None, payment_info.clone())
            .await
            .unwrap();

        //try refund for previous payment
        let transaction_id = get_connector_transaction_id(response.response).unwrap();
        tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await; // to avoid 404 error
        Ok(self
            .refund_payment(transaction_id, refund_data, payment_info)
            .await
            .unwrap())
    }

        /// Makes a payment with the provided authorize data and payment info, and then attempts to perform two refunds for the previous payment using the provided refund data and payment info.
    /// 
    /// # Arguments
    /// * `authorize_data` - Optional payment authorization data
    /// * `refund_data` - Optional refund data
    /// * `payment_info` - Optional payment information
    /// 
    async fn make_payment_and_multiple_refund(
        &self,
        authorize_data: Option<types::PaymentsAuthorizeData>,
        refund_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) {
        //make a successful payment
        let response = self
            .make_payment(authorize_data, payment_info.clone())
            .await
            .unwrap();

        //try refund for previous payment
        let transaction_id = get_connector_transaction_id(response.response).unwrap();
        for _x in 0..2 {
            tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await; // to avoid 404 error
            let refund_response = self
                .refund_payment(
                    transaction_id.clone(),
                    refund_data.clone(),
                    payment_info.clone(),
                )
                .await
                .unwrap();
            assert_eq!(
                refund_response.response.unwrap().refund_status,
                enums::RefundStatus::Success,
            );
        }
    }

        /// Asynchronously syncs a refund with the provided refund ID, payment data, and payment info. If the payment data is not provided, default refund data will be used. Returns a result with the synced refund data or a report of any connector errors.
    async fn sync_refund(
        &self,
        refund_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundSyncRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| types::RefundsData {
                payment_amount: 1000,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: "".to_string(),
                webhook_url: None,
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
                connector_refund_id: Some(refund_id),
                browser_info: None,
            }),
            payment_info,
        );
        Box::pin(call_connector(request, integration)).await
    }

    /// will retry the rsync till the given status matches or retry max 3 times
    async fn rsync_retry_till_status_matches(
        &self,
        status: enums::RefundStatus,
        refund_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundSyncRouterData, Report<ConnectorError>> {
        let max_tries = 3;
        for curr_try in 0..max_tries {
            let sync_res = self
                .sync_refund(
                    refund_id.clone(),
                    payment_data.clone(),
                    payment_info.clone(),
                )
                .await
                .unwrap();
            if (sync_res.clone().response.unwrap().refund_status == status)
                || (curr_try == max_tries - 1)
            {
                return Ok(sync_res);
            }
            tokio::time::sleep(Duration::from_secs(self.get_request_interval())).await;
        }
        Err(errors::ConnectorError::ProcessingStepFailed(None).into())
    }

    #[cfg(feature = "payouts")]
        /// Retrieves the payout request data and generates the corresponding RouterData.
    fn get_payout_request<Flow, Res>(
        &self,
        connector_payout_id: Option<String>,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> RouterData<Flow, types::PayoutsData, Res> {
        self.generate_data(
            types::PayoutsData {
                payout_id: core_utils::get_or_generate_uuid("payout_id", None)
                    .map_or("payout_3154763247".to_string(), |p| p),
                amount: 1,
                connector_payout_id,
                destination_currency: payment_info.to_owned().map_or(enums::Currency::EUR, |pi| {
                    pi.currency.map_or(enums::Currency::EUR, |c| c)
                }),
                source_currency: payment_info.to_owned().map_or(enums::Currency::EUR, |pi| {
                    pi.currency.map_or(enums::Currency::EUR, |c| c)
                }),
                entity_type: enums::PayoutEntityType::Individual,
                payout_type,
                customer_details: Some(payments::CustomerDetails {
                    customer_id: core_utils::get_or_generate_id("customer_id", &None, "cust_").ok(),
                    name: Some(Secret::new("John Doe".to_string())),
                    email: Email::from_str("john.doe@example").ok(),
                    phone: Some(Secret::new("620874518".to_string())),
                    phone_country_code: Some("+31".to_string()),
                }),
            },
            payment_info,
        )
    }

        /// Generates router data for a payment request, including information such as merchant and customer IDs, payment and attempt IDs, status, authentication type, payment method, description, return URL, request and response, address, access and session tokens, reference ID, payment method token, connector customer, preprocessing and request reference IDs, payout method data, and various other metadata. This method returns a RouterData struct containing the generated data.
    fn generate_data<Flow, Req: From<Req>, Res>(
        &self,
        req: Req,
        info: Option<PaymentInfo>,
    ) -> RouterData<Flow, Req, Res> {
        RouterData {
            flow: PhantomData,
            merchant_id: self.get_name(),
            customer_id: Some(self.get_name()),
            connector: self.get_name(),
            payment_id: uuid::Uuid::new_v4().to_string(),
            attempt_id: uuid::Uuid::new_v4().to_string(),
            status: enums::AttemptStatus::default(),
            auth_type: info
                .clone()
                .map_or(enums::AuthenticationType::NoThreeDs, |a| {
                    a.auth_type
                        .map_or(enums::AuthenticationType::NoThreeDs, |a| a)
                }),
            payment_method: enums::PaymentMethod::Card,
            connector_auth_type: self.get_auth_token(),
            description: Some("This is a test".to_string()),
            return_url: info.clone().and_then(|a| a.return_url),
            request: req,
            response: Err(types::ErrorResponse::default()),
            payment_method_id: None,
            address: info
                .clone()
                .and_then(|a| a.address)
                .or_else(|| Some(PaymentAddress::default()))
                .unwrap(),
            connector_meta_data: info
                .clone()
                .and_then(|a| a.connector_meta_data.map(masking::Secret::new)),
            amount_captured: None,
            access_token: info.clone().and_then(|a| a.access_token),
            session_token: None,
            reference_id: None,
            payment_method_token: info
                .clone()
                .and_then(|a| a.payment_method_token.map(types::PaymentMethodToken::Token)),
            connector_customer: info.clone().and_then(|a| a.connector_customer),
            recurring_mandate_payment_data: None,

            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: info.and_then(|p| p.payout_method_data),
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            payment_method_balance: None,
            connector_api_version: None,
            connector_http_status_code: None,
            apple_pay_flow: None,
            external_latency: None,
            frm_metadata: None,
        }
    }

        /// Retrieves the connector transaction ID from the capture data response, if available.
    fn get_connector_transaction_id_from_capture_data(
        &self,
        response: types::PaymentsCaptureRouterData,
    ) -> Option<String> {
        match response.response {
            Ok(types::PaymentsResponseData::TransactionResponse { resource_id, .. }) => {
                resource_id.get_connector_transaction_id().ok()
            }
            Ok(types::PaymentsResponseData::SessionResponse { .. }) => None,
            Ok(types::PaymentsResponseData::SessionTokenResponse { .. }) => None,
            Ok(types::PaymentsResponseData::TokenizationResponse { .. }) => None,
            Ok(types::PaymentsResponseData::TransactionUnresolvedResponse { .. }) => None,
            Ok(types::PaymentsResponseData::ConnectorCustomerResponse { .. }) => None,
            Ok(types::PaymentsResponseData::PreProcessingResponse { .. }) => None,
            Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. }) => None,
            Ok(types::PaymentsResponseData::MultipleCaptureResponse { .. }) => None,
            Ok(types::PaymentsResponseData::IncrementalAuthorizationResponse { .. }) => None,
            Err(_) => None,
        }
    }

    #[cfg(feature = "payouts")]
        /// This method verifies the eligibility for a payout based on the specified payout type and payment info. It obtains the connector integration, creates a payout request, and executes pre-tasks and connector processing steps to determine the eligibility for the payout. If successful, it returns the payout response data.
    async fn verify_payout_eligibility(
        &self,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::PoEligibility,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = self
            .get_payout_data()
            .ok_or(ConnectorError::FailedToObtainPreferredConnector)?
            .connector
            .get_connector_integration();
        let mut request = self.get_payout_request(None, payout_type, payment_info);
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        connector_integration
            .execute_pretasks(&mut request, &state)
            .await?;
        let res = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;
        Ok(res.response.unwrap())
    }

    #[cfg(feature = "payouts")]
        /// Asynchronously fulfills a payout request by obtaining the connector integration, creating a payout request, executing pre-tasks, and then executing the connector processing step to trigger the payout. Returns a result containing the payout response data or a report of any connector errors.
    async fn fulfill_payout(
        &self,
        connector_payout_id: Option<String>,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::PoFulfill,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = self
            .get_payout_data()
            .ok_or(ConnectorError::FailedToObtainPreferredConnector)?
            .connector
            .get_connector_integration();
        let mut request = self.get_payout_request(connector_payout_id, payout_type, payment_info);
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        connector_integration
            .execute_pretasks(&mut request, &state)
            .await?;
        let res = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;
        Ok(res.response.unwrap())
    }

    #[cfg(feature = "payouts")]
        /// Asynchronously creates a payout using the specified connector customer, payout type, and payment information. Returns a Result containing the response data for the created payout or a Report with a ConnectorError in case of failure.
    async fn create_payout(
        &self,
        connector_customer: Option<String>,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::PoCreate,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = self
            .get_payout_data()
            .ok_or(ConnectorError::FailedToObtainPreferredConnector)?
            .connector
            .get_connector_integration();
        let mut request = self.get_payout_request(None, payout_type, payment_info);
        request.connector_customer = connector_customer;
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        connector_integration
            .execute_pretasks(&mut request, &state)
            .await?;
        let res = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;
        Ok(res.response.unwrap())
    }

    #[cfg(feature = "payouts")]
        /// Asynchronously cancels a payout using the specified connector payout ID, payout type, and payment information. Returns a result containing the payout response data or a report of any connector error encountered.
    async fn cancel_payout(
        &self,
        connector_payout_id: String,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::PoCancel,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = self
            .get_payout_data()
            .ok_or(ConnectorError::FailedToObtainPreferredConnector)?
            .connector
            .get_connector_integration();
        let mut request =
            self.get_payout_request(Some(connector_payout_id), payout_type, payment_info);
        let tx: oneshot::Sender<()> = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        connector_integration
            .execute_pretasks(&mut request, &state)
            .await?;
        let res = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;
        Ok(res.response.unwrap())
    }

    #[cfg(feature = "payouts")]
        /// This method creates a new payout and fulfills it, then returns the result.
    async fn create_and_fulfill_payout(
        &self,
        connector_customer: Option<String>,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let create_res = self
            .create_payout(connector_customer, payout_type, payment_info.to_owned())
            .await?;
        assert_eq!(
            create_res.status.unwrap(),
            enums::PayoutStatus::RequiresFulfillment
        );
        let fulfill_res = self
            .fulfill_payout(
                Some(create_res.connector_payout_id),
                payout_type,
                payment_info.to_owned(),
            )
            .await?;
        Ok(fulfill_res)
    }

    #[cfg(feature = "payouts")]
        /// This method creates a payout and then immediately cancels it. It takes in the connector customer ID, payout type, and payment information, and returns a Result containing the payout response data or a ConnectorError report if an error occurs.
    async fn create_and_cancel_payout(
        &self,
        connector_customer: Option<String>,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let create_res = self
            .create_payout(connector_customer, payout_type, payment_info.to_owned())
            .await?;
        assert_eq!(
            create_res.status.unwrap(),
            enums::PayoutStatus::RequiresFulfillment
        );
        let cancel_res = self
            .cancel_payout(
                create_res.connector_payout_id,
                payout_type,
                payment_info.to_owned(),
            )
            .await?;
        Ok(cancel_res)
    }

    #[cfg(feature = "payouts")]
        /// Asynchronously creates a payout recipient using the provided payout type and payment information.
    /// Returns a Result containing the PayoutsResponseData if successful, or a Report containing a ConnectorError if an error occurs.
    async fn create_payout_recipient(
        &self,
        payout_type: enums::PayoutType,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PayoutsResponseData, Report<ConnectorError>> {
        let connector_integration: services::BoxedConnectorIntegration<
            '_,
            types::api::PoRecipient,
            types::PayoutsData,
            types::PayoutsResponseData,
        > = self
            .get_payout_data()
            .ok_or(ConnectorError::FailedToObtainPreferredConnector)?
            .connector
            .get_connector_integration();
        let mut request = self.get_payout_request(None, payout_type, payment_info);
        let tx = oneshot::channel().0;

        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        connector_integration
            .execute_pretasks(&mut request, &state)
            .await?;
        let res = services::api::execute_connector_processing_step(
            &state,
            connector_integration,
            &request,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;
        Ok(res.response.unwrap())
    }
}

/// Asynchronously calls the connector integration to process the provided request. It creates a new application state with a PostgresqlTest storage implementation and a mock API client, then executes the connector processing step using the provided integration and request. It returns the updated RouterData with the result of the processing step or a ConnectorError report if an error occurs.
async fn call_connector<
    T: Debug + Clone + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    request: RouterData<T, Req, Resp>,
    integration: services::BoxedConnectorIntegration<'_, T, Req, Resp>,
) -> Result<RouterData<T, Req, Resp>, Report<ConnectorError>> {
    let conf = Settings::new().unwrap();
    let tx: oneshot::Sender<()> = oneshot::channel().0;

    let state = routes::AppState::with_storage(
        conf,
        StorageImpl::PostgresqlTest,
        tx,
        Box::new(services::MockApiClient),
    )
    .await;
    services::api::execute_connector_processing_step(
        &state,
        integration,
        &request,
        payments::CallConnectorAction::Trigger,
        None,
    )
    .await
}

pub struct MockConfig {
    pub address: Option<String>,
    pub mocks: Vec<Mock>,
}

#[async_trait]
pub trait LocalMock {
        /// Starts a mock server with the given configuration.
    /// 
    /// # Arguments
    /// 
    /// * `config` - A `MockConfig` struct containing the server configuration and mocks to register.
    /// 
    /// # Returns
    /// 
    /// A `MockServer` instance representing the started mock server.
    /// 
    async fn start_server(&self, config: MockConfig) -> MockServer {
        let address = config
            .address
            .unwrap_or_else(|| "127.0.0.1:9090".to_string());
        let listener = std::net::TcpListener::bind(address).unwrap();
        let expected_server_address = listener
            .local_addr()
            .expect("Failed to get server address.");
        let mock_server = MockServer::builder().listener(listener).start().await;
        assert_eq!(&expected_server_address, mock_server.address());
        for mock in config.mocks {
            mock_server.register(mock).await;
        }
        mock_server
    }
}

pub struct PaymentAuthorizeType(pub types::PaymentsAuthorizeData);
pub struct PaymentCaptureType(pub types::PaymentsCaptureData);
pub struct PaymentCancelType(pub types::PaymentsCancelData);
pub struct PaymentSyncType(pub types::PaymentsSyncData);
pub struct PaymentRefundType(pub types::RefundsData);
pub struct CCardType(pub api::Card);
pub struct BrowserInfoType(pub types::BrowserInformation);
pub struct CustomerType(pub types::ConnectorCustomerData);
pub struct TokenType(pub types::PaymentMethodTokenizationData);

impl Default for CCardType {
        /// Constructs a new instance of the `Card` struct with default values for card number, expiration date, card holder name, CVC, and nick name.
    fn default() -> Self {
        Self(api::Card {
            card_number: cards::CardNumber::from_str("4200000000000000").unwrap(),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_holder_name: Some(masking::Secret::new("John Doe".to_string())),
            card_cvc: Secret::new("999".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(masking::Secret::new("nick_name".into())),
        })
    }
}

impl Default for PaymentAuthorizeType {
        /// Creates a new instance of the PaymentsAuthorizeData struct with default values
    fn default() -> Self {
        let data = types::PaymentsAuthorizeData {
            payment_method_data: types::api::PaymentMethodData::Card(CCardType::default().0),
            amount: 100,
            currency: enums::Currency::USD,
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            capture_method: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            browser_info: Some(BrowserInfoType::default().0),
            order_details: None,
            order_category: None,
            email: None,
            customer_name: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
            router_return_url: None,
            complete_authorize_url: None,
            webhook_url: None,
            customer_id: None,
            surcharge_details: None,
            request_incremental_authorization: false,
            metadata: None,
        };
        Self(data)
    }
}

impl Default for PaymentCaptureType {
        /// This method creates a new instance of the current type with default values for the fields, and initializes the PaymentsCaptureData with default values for the amount_to_capture, currency, connector_transaction_id, and payment_amount fields.
    fn default() -> Self {
        Self(types::PaymentsCaptureData {
            amount_to_capture: 100,
            currency: enums::Currency::USD,
            connector_transaction_id: "".to_string(),
            payment_amount: 100,
            ..Default::default()
        })
    }
}

impl Default for PaymentCancelType {
        /// Creates a new instance of the struct with default values for the `PaymentsCancelData` fields, and sets the `cancellation_reason` to "requested_by_customer" and `connector_transaction_id` to an empty string.
    fn default() -> Self {
        Self(types::PaymentsCancelData {
            cancellation_reason: Some("requested_by_customer".to_string()),
            connector_transaction_id: "".to_string(),
            ..Default::default()
        })
    }
}

impl Default for BrowserInfoType {
        /// Creates a new instance of the struct with default values for browser information.
    fn default() -> Self {
        let data = types::BrowserInformation {
            user_agent: Some("".to_string()),
            accept_header: Some("".to_string()),
            language: Some("nl-NL".to_string()),
            color_depth: Some(24),
            screen_height: Some(723),
            screen_width: Some(1536),
            time_zone: Some(0),
            java_enabled: Some(true),
            java_script_enabled: Some(true),
            ip_address: Some("127.0.0.1".parse().unwrap()),
        };
        Self(data)
    }
}

impl Default for PaymentSyncType {
        /// Creates a new instance of the struct with default values for the payments sync data.
    fn default() -> Self {
        let data = types::PaymentsSyncData {
            mandate_id: None,
            connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                "12345".to_string(),
            ),
            encoded_data: None,
            capture_method: None,
            sync_type: types::SyncRequestType::SinglePaymentSync,
            connector_meta: None,
        };
        Self(data)
    }
}

impl Default for PaymentRefundType {
        /// Creates a new instance of the struct with default refund data, including a payment amount of 100 USD, a generated refund ID, an empty connector transaction ID, a refund amount of 100, a reason for the refund, and other optional fields set to None.
    fn default() -> Self {
        let data = types::RefundsData {
            payment_amount: 100,
            currency: enums::Currency::USD,
            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 100,
            webhook_url: None,
            connector_metadata: None,
            reason: Some("Customer returned product".to_string()),
            connector_refund_id: None,
            browser_info: None,
        };
        Self(data)
    }
}

impl Default for CustomerType {
        /// Creates a new instance of the struct using default values for its fields.
    fn default() -> Self {
        let data = types::ConnectorCustomerData {
            payment_method_data: types::api::PaymentMethodData::Card(CCardType::default().0),
            description: None,
            email: Email::from_str("test@juspay.in").ok(),
            phone: None,
            name: None,
            preprocessing_id: None,
        };
        Self(data)
    }
}

impl Default for TokenType {
        /// Creates a default PaymentMethodTokenizationData with a default card payment method data, no browser info, an amount of 100, and a currency of USD.
    fn default() -> Self {
        let data = types::PaymentMethodTokenizationData {
            payment_method_data: types::api::PaymentMethodData::Card(CCardType::default().0),
            browser_info: None,
            amount: Some(100),
            currency: enums::Currency::USD,
        };
        Self(data)
    }
}

/// Retrieves the connector transaction ID from the given PaymentsResponseData.
/// If the response is a TransactionResponse, the connector transaction ID is extracted and returned.
/// If the response is any other type, None is returned.
pub fn get_connector_transaction_id(
    response: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> Option<String> {
    match response {
        Ok(types::PaymentsResponseData::TransactionResponse { resource_id, .. }) => {
            resource_id.get_connector_transaction_id().ok()
        }
        Ok(types::PaymentsResponseData::SessionResponse { .. }) => None,
        Ok(types::PaymentsResponseData::SessionTokenResponse { .. }) => None,
        Ok(types::PaymentsResponseData::TokenizationResponse { .. }) => None,
        Ok(types::PaymentsResponseData::TransactionUnresolvedResponse { .. }) => None,
        Ok(types::PaymentsResponseData::PreProcessingResponse { .. }) => None,
        Ok(types::PaymentsResponseData::ConnectorCustomerResponse { .. }) => None,
        Ok(types::PaymentsResponseData::ThreeDSEnrollmentResponse { .. }) => None,
        Ok(types::PaymentsResponseData::MultipleCaptureResponse { .. }) => None,
        Ok(types::PaymentsResponseData::IncrementalAuthorizationResponse { .. }) => None,
        Err(_) => None,
    }
}

/// Extracts the connector metadata from the provided PaymentsResponseData if the response is successful, otherwise returns None.
pub fn get_connector_metadata(
    response: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> Option<serde_json::Value> {
    match response {
        Ok(types::PaymentsResponseData::TransactionResponse {
            resource_id: _,
            redirection_data: _,
            mandate_reference: _,
            connector_metadata,
            network_txn_id: _,
            connector_response_reference_id: _,
            incremental_authorization_allowed: _,
        }) => connector_metadata,
        _ => None,
    }
}

/// Converts a ConnectorAuthType enum to its equivalent types::ConnectorAuthType enum.
pub fn to_connector_auth_type(auth_type: ConnectorAuthType) -> types::ConnectorAuthType {
    match auth_type {
        ConnectorAuthType::HeaderKey { api_key } => types::ConnectorAuthType::HeaderKey { api_key },
        ConnectorAuthType::BodyKey { api_key, key1 } => {
            types::ConnectorAuthType::BodyKey { api_key, key1 }
        }
        ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } => types::ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        },
        ConnectorAuthType::MultiAuthKey {
            api_key,
            key1,
            api_secret,
            key2,
        } => types::ConnectorAuthType::MultiAuthKey {
            api_key,
            key1,
            api_secret,
            key2,
        },
        _ => types::ConnectorAuthType::NoKey,
    }
}
