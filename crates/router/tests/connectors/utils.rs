use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use error_stack::Report;
use masking::Secret;
use router::{
    core::{errors::ConnectorError, payments},
    db::StorageImpl,
    routes, services,
    types::{self, api, storage::enums, PaymentAddress},
};
use wiremock::{Mock, MockServer};

pub trait Connector {
    fn get_data(&self) -> types::api::ConnectorData;
    fn get_auth_token(&self) -> types::ConnectorAuthType;
    fn get_name(&self) -> String;
    fn get_connector_meta(&self) -> Option<serde_json::Value> {
        None
    }
}

#[derive(Debug, Default, Clone)]
pub struct PaymentInfo {
    pub address: Option<PaymentAddress>,
    pub auth_type: Option<enums::AuthenticationType>,
}

#[async_trait]
pub trait ConnectorActions: Connector {
    async fn authorize_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| types::PaymentsAuthorizeData {
                capture_method: Some(storage_models::enums::CaptureMethod::Manual),
                ..PaymentAuthorizeType::default().0
            }),
            payment_info,
        );
        call_connector(request, integration).await
    }
    async fn make_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| PaymentAuthorizeType::default().0),
            payment_info,
        );
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

    async fn capture_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::PaymentsCaptureData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCaptureRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or(types::PaymentsCaptureData {
                amount_to_capture: Some(100),
                currency: enums::Currency::USD,
                connector_transaction_id: transaction_id,
                amount: 100,
            }),
            payment_info,
        );
        call_connector(request, integration).await
    }

    async fn void_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::PaymentsCancelData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsCancelRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or(types::PaymentsCancelData {
                connector_transaction_id: transaction_id,
                cancellation_reason: Some("Test cancellation".to_string()),
            }),
            payment_info,
        );
        call_connector(request, integration).await
    }

    async fn refund_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundExecuteRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| types::RefundsData {
                amount: 100,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: transaction_id,
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
            }),
            payment_info,
        );
        call_connector(request, integration).await
    }

    async fn sync_refund(
        &self,
        transaction_id: String,
        payment_data: Option<types::RefundsData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::RefundSyncRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let request = self.generate_data(
            payment_data.unwrap_or_else(|| types::RefundsData {
                amount: 1000,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                connector_transaction_id: transaction_id,
                refund_amount: 100,
                connector_metadata: None,
                reason: None,
            }),
            payment_info,
        );
        call_connector(request, integration).await
    }

    fn generate_data<Flow, Req: From<Req>, Res>(
        &self,
        req: Req,
        info: Option<PaymentInfo>,
    ) -> types::RouterData<Flow, Req, Res> {
        types::RouterData {
            flow: PhantomData,
            merchant_id: self.get_name(),
            connector: self.get_name(),
            payment_id: uuid::Uuid::new_v4().to_string(),
            attempt_id: Some(uuid::Uuid::new_v4().to_string()),
            status: enums::AttemptStatus::default(),
            router_return_url: None,
            auth_type: info
                .clone()
                .map_or(enums::AuthenticationType::NoThreeDs, |a| {
                    a.auth_type
                        .map_or(enums::AuthenticationType::NoThreeDs, |a| a)
                }),
            payment_method: enums::PaymentMethodType::Card,
            connector_auth_type: self.get_auth_token(),
            description: Some("This is a test".to_string()),
            return_url: None,
            request: req,
            response: Err(types::ErrorResponse::default()),
            payment_method_id: None,
            address: info.map_or(PaymentAddress::default(), |a| a.address.unwrap()),
            connector_meta_data: self.get_connector_meta(),
            amount_captured: None,
        }
    }
}

async fn call_connector<
    T: Debug + Clone + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    request: types::RouterData<T, Req, Resp>,
    integration: services::BoxedConnectorIntegration<'_, T, Req, Resp>,
) -> Result<types::RouterData<T, Req, Resp>, Report<ConnectorError>> {
    use router::configs::settings::Settings;
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
pub struct PaymentSyncType(pub types::PaymentsSyncData);
pub struct PaymentRefundType(pub types::RefundsData);
pub struct CCardType(pub api::CCard);
pub struct BrowserInfoType(pub types::BrowserInformation);

impl Default for CCardType {
    fn default() -> Self {
        Self(api::CCard {
            card_number: Secret::new("4200000000000000".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_cvc: Secret::new("999".to_string()),
        })
    }
}

impl Default for PaymentAuthorizeType {
    fn default() -> Self {
        let data = types::PaymentsAuthorizeData {
            payment_method_data: types::api::PaymentMethod::Card(CCardType::default().0),
            amount: 100,
            currency: enums::Currency::USD,
            confirm: true,
            statement_descriptor_suffix: None,
            capture_method: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            browser_info: Some(BrowserInfoType::default().0),
            order_details: None,
            email: None,
        };
        Self(data)
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
        };
        Self(data)
    }
}

impl Default for PaymentRefundType {
    fn default() -> Self {
        let data = types::RefundsData {
            amount: 1000,
            currency: enums::Currency::USD,
            refund_id: uuid::Uuid::new_v4().to_string(),
            connector_transaction_id: String::new(),
            refund_amount: 100,
            connector_metadata: None,
            reason: None,
        };
        Self(data)
    }
}

pub fn get_connector_transaction_id(
    response: types::PaymentsAuthorizeRouterData,
) -> Option<String> {
    match response.response {
        Ok(types::PaymentsResponseData::TransactionResponse { resource_id, .. }) => {
            resource_id.get_connector_transaction_id().ok()
        }
        Ok(types::PaymentsResponseData::SessionResponse { .. }) => None,
        Err(_) => None,
    }
}
