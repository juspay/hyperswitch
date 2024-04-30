use async_trait::async_trait;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;

use crate::{
    connector::signifyd::transformers::RefundMethod,
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        fraud_check::{FeatureFrm, FraudCheckConnectorData, FrmData},
        payments::{self, flows::ConstructFlowSpecificData, helpers},
    },
    errors, services,
    types::{
        api::RecordReturn,
        domain,
        fraud_check::{
            FraudCheckRecordReturnData, FraudCheckResponseData, FrmRecordReturnRouterData,
        },
        storage::enums as storage_enums,
        ConnectorAuthType, ResponseId, RouterData,
    },
    utils, SessionState,
};

#[async_trait]
impl ConstructFlowSpecificData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for FrmData
{
    async fn construct_router_data<'a>(
        &self,
        _state: &SessionState,
        connector_id: &str,
        merchant_account: &domain::MerchantAccount,
        _key_store: &domain::MerchantKeyStore,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
    ) -> RouterResult<RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>>
    {
        let status = storage_enums::AttemptStatus::Pending;

        let auth_type: ConnectorAuthType = merchant_connector_account
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: "ConnectorAuthType".to_string(),
            })?;

        let customer_id = customer.to_owned().map(|customer| customer.customer_id);
        let currency = self.payment_attempt.clone().currency;
        let router_data = RouterData {
            flow: std::marker::PhantomData,
            merchant_id: merchant_account.merchant_id.clone(),
            customer_id,
            connector: connector_id.to_string(),
            payment_id: self.payment_intent.payment_id.clone(),
            attempt_id: self.payment_attempt.attempt_id.clone(),
            status,
            payment_method: utils::OptionExt::get_required_value(
                self.payment_attempt.payment_method,
                "payment_method_type",
            )?,
            connector_auth_type: auth_type,
            description: None,
            return_url: None,
            payment_method_id: None,
            address: self.address.clone(),
            auth_type: storage_enums::AuthenticationType::NoThreeDs,
            connector_meta_data: None,
            amount_captured: None,
            request: FraudCheckRecordReturnData {
                amount: self.payment_attempt.amount,
                refund_method: RefundMethod::OriginalPaymentInstrument, //we dont consume this data now in payments...hence hardcoded
                currency,
                refund_transaction_id: self.refund.clone().map(|refund| refund.refund_id),
            }, // self.order_details
            response: Ok(FraudCheckResponseData::RecordReturnResponse {
                resource_id: ResponseId::ConnectorTransactionId("".to_string()),
                connector_metadata: None,
                return_id: None,
            }),
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            connector_customer: None,
            preprocessing_id: None,
            payment_method_status: None,
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
            frm_metadata: None,
            refund_id: None,
            dispute_id: None,
            connector_response: None,
        };

        Ok(router_data)
    }
}

#[async_trait]
impl FeatureFrm<RecordReturn, FraudCheckRecordReturnData> for FrmRecordReturnRouterData {
    async fn decide_frm_flows<'a>(
        mut self,
        state: &SessionState,
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
    router_data: &'b mut FrmRecordReturnRouterData,
    state: &'a SessionState,
    connector: &FraudCheckConnectorData,
    call_connector_action: payments::CallConnectorAction,
    _merchant_account: &domain::MerchantAccount,
) -> RouterResult<FrmRecordReturnRouterData> {
    let connector_integration: services::BoxedConnectorIntegration<
        '_,
        RecordReturn,
        FraudCheckRecordReturnData,
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
