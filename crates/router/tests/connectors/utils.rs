use std::{fmt::Debug, marker::PhantomData, time::Duration};

use async_trait::async_trait;
use error_stack::Report;
use masking::Secret;
use router::{
    configs::settings::Settings,
    core::{errors, errors::ConnectorError, payments},
    db::StorageImpl,
    routes, services,
    types::{self, api, storage::enums, AccessToken, PaymentAddress, RouterData},
};
use wiremock::{Mock, MockServer};

pub trait Connector {
    fn get_data(&self) -> types::api::ConnectorData;
    fn get_auth_token(&self) -> types::ConnectorAuthType;
    fn get_name(&self) -> String;
    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        None
    }
    /// interval in seconds to be followed when making the subsequent request whenever needed
    fn get_request_interval(&self) -> u64 {
        5
    }
}

#[derive(Debug, Default, Clone)]
pub struct PaymentInfo {
    pub address: Option<PaymentAddress>,
    pub auth_type: Option<enums::AuthenticationType>,
    pub access_token: Option<AccessToken>,
    pub router_return_url: Option<String>,
    pub connector_meta_data: Option<serde_json::Value>,
}

#[async_trait]
pub trait ConnectorActions: Connector {
    async fn authorize_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(storage_models::enums::CaptureMethod::Manual),
                ..(payment_data.unwrap_or(PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        let state =
            routes::AppState::with_storage(Settings::new().unwrap(), StorageImpl::PostgresqlTest)
                .await;
        integration.execute_pretasks(&mut request, &state).await?;
        call_connector(request, integration).await
    }

    async fn make_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(storage_models::enums::CaptureMethod::Automatic),
                ..(payment_data.unwrap_or(PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        let state =
            routes::AppState::with_storage(Settings::new().unwrap(), StorageImpl::PostgresqlTest)
                .await;
        integration.execute_pretasks(&mut request, &state).await?;
        call_connector(request, integration).await
    }

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
        call_connector(request, integration).await
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
        call_connector(request, integration).await
    }

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
        call_connector(request, integration).await
    }

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

    async fn refund_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            types::RefundsData {
                connector_transaction_id: transaction_id,
                ..payment_data.unwrap_or(PaymentRefundType::default().0)
            },
            payment_info,
        );
        call_connector(request, integration).await
    }

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

    async fn sync_refund(
        &self,
        refund_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundSyncRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| types::RefundsData {
                amount: 1000,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: "".to_string(),
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
                connector_refund_id: Some(refund_id),
            }),
            payment_info,
        );
        call_connector(request, integration).await
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

    fn generate_data<Flow, Req: From<Req>, Res>(
        &self,
        req: Req,
        info: Option<PaymentInfo>,
    ) -> RouterData<Flow, Req, Res> {
        RouterData {
            flow: PhantomData,
            merchant_id: self.get_name(),
            connector: self.get_name(),
            payment_id: uuid::Uuid::new_v4().to_string(),
            attempt_id: uuid::Uuid::new_v4().to_string(),
            status: enums::AttemptStatus::default(),
            router_return_url: info.clone().and_then(|a| a.router_return_url),
            complete_authorize_url: None,
            auth_type: info
                .clone()
                .map_or(enums::AuthenticationType::NoThreeDs, |a| {
                    a.auth_type
                        .map_or(enums::AuthenticationType::NoThreeDs, |a| a)
                }),
            payment_method: enums::PaymentMethod::Card,
            connector_auth_type: self.get_auth_token(),
            description: Some("This is a test".to_string()),
            return_url: None,
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
            access_token: info.and_then(|a| a.access_token),
            session_token: None,
            reference_id: None,
        }
    }

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
            Err(_) => None,
        }
    }
}

async fn call_connector<
    T: Debug + Clone + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    request: RouterData<T, Req, Resp>,
    integration: services::BoxedConnectorIntegration<'_, T, Req, Resp>,
) -> Result<RouterData<T, Req, Resp>, Report<ConnectorError>> {
    let conf = Settings::new().unwrap();
    let state = routes::AppState::with_storage(conf, StorageImpl::PostgresqlTest).await;
    services::api::execute_connector_processing_step(
        &state,
        integration,
        &request,
        payments::CallConnectorAction::Trigger,
    )
    .await
}

pub struct MockConfig {
    pub address: Option<String>,
    pub mocks: Vec<Mock>,
}

#[async_trait]
pub trait LocalMock {
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

impl Default for CCardType {
    fn default() -> Self {
        Self(api::Card {
            card_number: Secret::new("4200000000000000".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_cvc: Secret::new("999".to_string()),
            card_issuer: None,
            card_network: None,
        })
    }
}

impl Default for PaymentAuthorizeType {
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
            email: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            payment_experience: None,
            payment_method_type: None,
        };
        Self(data)
    }
}

impl Default for PaymentCaptureType {
    fn default() -> Self {
        Self(types::PaymentsCaptureData {
            amount_to_capture: Some(100),
            currency: enums::Currency::USD,
            connector_transaction_id: "".to_string(),
            amount: 100,
        })
    }
}

impl Default for PaymentCancelType {
    fn default() -> Self {
        Self(types::PaymentsCancelData {
            cancellation_reason: Some("requested_by_customer".to_string()),
            connector_transaction_id: "".to_string(),
            ..Default::default()
        })
    }
}

impl Default for BrowserInfoType {
    fn default() -> Self {
        let data = types::BrowserInformation {
            user_agent: "".to_string(),
            accept_header: "".to_string(),
            language: "nl-NL".to_string(),
            color_depth: 24,
            screen_height: 723,
            screen_width: 1536,
            time_zone: 0,
            java_enabled: true,
            java_script_enabled: true,
            ip_address: Some("127.0.0.1".parse().unwrap()),
        };
        Self(data)
    }
}

impl Default for PaymentSyncType {
    fn default() -> Self {
        let data = types::PaymentsSyncData {
            connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                "12345".to_string(),
            ),
            encoded_data: None,
            capture_method: None,
            connector_meta: None,
        };
        Self(data)
    }
}

impl Default for PaymentRefundType {
    fn default() -> Self {
        let data = types::RefundsData {
            amount: 100,
            currency: enums::Currency::USD,
            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 100,
            connector_metadata: None,
            reason: Some("Customer returned product".to_string()),
            connector_refund_id: None,
        };
        Self(data)
    }
}

pub fn get_connector_transaction_id(
    response: Result<types::PaymentsResponseData, types::ErrorResponse>,
) -> Option<String> {
    match response {
        Ok(types::PaymentsResponseData::TransactionResponse { resource_id, .. }) => {
            resource_id.get_connector_transaction_id().ok()
        }
        Ok(types::PaymentsResponseData::SessionResponse { .. }) => None,
        Ok(types::PaymentsResponseData::SessionTokenResponse { .. }) => None,
        Err(_) => None,
    }
}
