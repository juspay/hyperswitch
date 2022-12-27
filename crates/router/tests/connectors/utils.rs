use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use masking::Secret;
use router::{
    core::payments,
    db::StorageImpl,
    routes, services,
    types::{self, api, storage::enums, PaymentAddress},
};

pub trait Connector {
    fn get_data(&self) -> types::api::ConnectorData;
    fn get_auth_token(&self) -> types::ConnectorAuthType;
    fn get_name(&self) -> String;
}

#[async_trait]
pub trait ConnectorActions: Connector {
    async fn authorize_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
    ) -> types::PaymentsAuthorizeRouterData {
        let integration = self.get_data().connector.get_connector_integration();
        let request = generate_data(
            self.get_name(),
            self.get_auth_token(),
            payment_data.unwrap_or_else(|| types::PaymentsAuthorizeData {
                capture_method: Some(storage_models::enums::CaptureMethod::Manual),
                ..PaymentAuthorizeType::default().0
            }),
        );
        call_connector(request, integration).await
    }
    async fn make_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
    ) -> types::PaymentsAuthorizeRouterData {
        let integration = self.get_data().connector.get_connector_integration();
        let request = generate_data(
            self.get_name(),
            self.get_auth_token(),
            payment_data.unwrap_or_else(|| PaymentAuthorizeType::default().0),
        );
        call_connector(request, integration).await
    }
    async fn capture_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::PaymentsCaptureData>,
    ) -> types::PaymentsCaptureRouterData {
        let integration = self.get_data().connector.get_connector_integration();
        let request = generate_data(
            self.get_name(),
            self.get_auth_token(),
            payment_data.unwrap_or(types::PaymentsCaptureData {
                amount_to_capture: Some(100),
                connector_transaction_id: transaction_id,
            }),
        );
        call_connector(request, integration).await
    }
    async fn refund_payment(
        &self,
        transaction_id: String,
        payment_data: Option<types::RefundsData>,
    ) -> types::RefundExecuteRouterData {
        let integration = self.get_data().connector.get_connector_integration();
        let request = generate_data(
            self.get_name(),
            self.get_auth_token(),
            payment_data.unwrap_or_else(|| types::RefundsData {
                amount: 100,
                currency: enums::Currency::USD,
                refund_id: uuid::Uuid::new_v4().to_string(),
                payment_method_data: types::api::PaymentMethod::Card(CCardType::default().0),
                connector_transaction_id: transaction_id,
                refund_amount: 100,
            }),
        );
        call_connector(request, integration).await
    }
}

async fn call_connector<
    T: Debug + Clone + 'static,
    Req: Debug + Clone + 'static,
    Resp: Debug + Clone + 'static,
>(
    request: types::RouterData<T, Req, Resp>,
    integration: services::BoxedConnectorIntegration<'_, T, Req, Resp>,
) -> types::RouterData<T, Req, Resp> {
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
    .unwrap()
}

pub struct PaymentAuthorizeType(pub types::PaymentsAuthorizeData);
pub struct PaymentRefundType(pub types::RefundsData);
pub struct CCardType(pub api::CCard);

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
            browser_info: None,
            order_details: None,
            email: None,
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
            payment_method_data: types::api::PaymentMethod::Card(CCardType::default().0),
            connector_transaction_id: String::new(),
            refund_amount: 100,
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

fn generate_data<Flow, Req: From<Req>, Res>(
    connector: String,
    connector_auth_type: types::ConnectorAuthType,
    req: Req,
) -> types::RouterData<Flow, Req, Res> {
    types::RouterData {
        flow: PhantomData,
        merchant_id: connector.clone(),
        connector,
        payment_id: uuid::Uuid::new_v4().to_string(),
        status: enums::AttemptStatus::default(),
        orca_return_url: None,
        auth_type: enums::AuthenticationType::NoThreeDs,
        payment_method: enums::PaymentMethodType::Card,
        connector_auth_type,
        description: Some("This is a test".to_string()),
        return_url: None,
        request: req,
        response: Err(types::ErrorResponse::default()),
        payment_method_id: None,
        address: PaymentAddress::default(),
        connector_meta_data: None,
        amount_captured: None,
    }
}
