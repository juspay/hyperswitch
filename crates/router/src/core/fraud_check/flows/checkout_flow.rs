use async_trait::async_trait;
use common_utils::{ext_traits::ValueExt, pii::Email};
use error_stack::ResultExt;
use masking::ExposeInterface;

use super::{ConstructFlowSpecificData, FeatureFrm};
use crate::{
    connector::utils::PaymentsAttemptData,
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        fraud_check::types::FrmData,
        payments::{self, helpers},
    },
    errors, services,
    types::{
        api::fraud_check::{self as frm_api, FraudCheckConnectorData},
        domain,
        fraud_check::{FraudCheckCheckoutData, FraudCheckResponseData, FrmCheckoutRouterData},
        storage::enums as storage_enums,
        BrowserInformation, ConnectorAuthType, ResponseId, RouterData,
    },
    AppState,
};

#[async_trait]
impl ConstructFlowSpecificData<frm_api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>
    for FrmData
{
    async fn construct_router_data<'a>(
        &self,
        _state: &AppState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<RouterData<frm_api::Checkout, FraudCheckCheckoutData, FraudCheckResponseData>>
    {
        let status = storage_enums::AttemptStatus::Pending;

        let auth_type: ConnectorAuthType = merchant_connector_account
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: "ConnectorAuthType".to_string(),
            })?;

        let browser_info: Option<BrowserInformation> = self.payment_attempt.get_browser_info().ok();
        let customer_id = customer.to_owned().map(|customer| customer.customer_id);

        let router_data = RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id,
            connector: connector_id.to_string(),
            payment_id: self.payment_intent.payment_id.clone(),
            attempt_id: self.payment_attempt.attempt_id.clone(),
            status,
            payment_method: self
                .payment_attempt
                .payment_method
                .ok_or(errors::ApiErrorResponse::PaymentMethodNotFound)?,
            connector_auth_type: auth_type,
            description: None,
            return_url: None,
            payment_method_id: None,
            address: self.address.clone(),
            auth_type: storage_enums::AuthenticationType::NoThreeDs,
            connector_meta_data: None,
            amount_captured: None,
            request: FraudCheckCheckoutData {
                amount: self.payment_attempt.amount,
                order_details: self.order_details.clone(),
                currency: self.payment_attempt.currency,
                browser_info,
                payment_method_data: self
                    .payment_attempt
                    .payment_method_data
                    .as_ref()
                    .map(|pm_data| {
                        pm_data
                            .clone()
                            .parse_value::<api_models::payments::AdditionalPaymentData>(
                                "AdditionalPaymentData",
                            )
                    })
                    .transpose()
                    .unwrap_or_default(),
                email: customer.clone().and_then(|customer_data| {
                    customer_data
                        .email
                        .and_then(|email| Email::try_from(email.into_inner().expose()).ok())
                }),
                gateway: self.payment_attempt.connector.clone(),
            }, // self.order_details
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId("".to_string()),
                connector_metadata: None,
                status: storage_enums::FraudCheckStatus::Pending,
                score: None,
                reason: None,
            }),
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            connector_customer: None,
            preprocessing_id: None,
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            test_mode: None,
            recurring_mandate_payment_data: None,
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            payment_method_balance: None,
            connector_http_status_code: None,
            external_latency: None,
            connector_api_version: None,
            apple_pay_flow: None,
            frm_metadata: self.frm_metadata.clone(),
            refund_id: None,
            dispute_id: None,
        };

        Ok(router_data)
    }
}

#[async_trait]
impl FeatureFrm<frm_api::Checkout, FraudCheckCheckoutData> for FrmCheckoutRouterData {
    async fn decide_frm_flows<'a>(
        mut self,
        state: &AppState,
        connector: &FraudCheckConnectorData,
        call_connector_action: payments::CallConnectorAction,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<Self> {
        decide_frm_flow(
            &mut self,
            state,
            connector,
            call_connector_action,
            merchant_account,
        )
        .await
    }
}

pub async fn decide_frm_flow<'a, 'b>(
    router_data: &'b mut FrmCheckoutRouterData,
    state: &'a AppState,
    connector: &FraudCheckConnectorData,
    call_connector_action: payments::CallConnectorAction,
    _merchant_account: &domain::MerchantAccount,
) -> RouterResult<FrmCheckoutRouterData> {
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        frm_api::Checkout,
        FraudCheckCheckoutData,
        FraudCheckResponseData,
    > = connector.connector.get_connector_integration();
    let resp = services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        call_connector_action,
        None,
    )
    .await
    .to_payment_failed_response()?;

    Ok(resp)
}
