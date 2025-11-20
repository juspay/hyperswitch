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
        ConnectorAuthType, MerchantRecipientData, ResponseId, RouterData,
    },
    utils, SessionState,
};

#[async_trait]
impl ConstructFlowSpecificData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>
    for FrmData
{
    #[cfg(feature = "v2")]
    async fn construct_router_data<'a>(
        &self,
        _state: &SessionState,
        _connector_id: &str,
        _platform: &domain::Platform,
        _customer: &Option<domain::Customer>,
        _merchant_connector_account: &domain::MerchantConnectorAccountTypeDetails,
        _merchant_recipient_data: Option<MerchantRecipientData>,
        _header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
    ) -> RouterResult<RouterData<RecordReturn, FraudCheckRecordReturnData, FraudCheckResponseData>>
    {
        todo!()
    }

    #[cfg(feature = "v1")]
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        platform: &domain::Platform,
        customer: &Option<domain::Customer>,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _merchant_recipient_data: Option<MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
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
            merchant_id: platform.get_processor().get_account().get_id().clone(),
            tenant_id: state.tenant.tenant_id.clone(),
            customer_id,
            connector: connector_id.to_string(),
            payment_id: self.payment_intent.payment_id.get_string_repr().to_owned(),
            attempt_id: self.payment_attempt.attempt_id.clone(),
            status,
            payment_method: utils::OptionExt::get_required_value(
                self.payment_attempt.payment_method,
                "payment_method",
            )?,
            payment_method_type: self.payment_attempt.payment_method_type,
            connector_auth_type: auth_type,
            description: None,
            address: self.address.clone(),
            auth_type: storage_enums::AuthenticationType::NoThreeDs,
            connector_meta_data: None,
            connector_wallets_details: None,
            amount_captured: None,
            minor_amount_captured: None,
            request: FraudCheckRecordReturnData {
                amount: self
                    .payment_attempt
                    .net_amount
                    .get_total_amount()
                    .get_amount_as_i64(),
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
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
            authorized_amount: None,
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
        platform: &domain::Platform,
    ) -> RouterResult<Self> {
        decide_frm_flow(&mut self, state, connector, call_connector_action, platform).await
    }
}

pub async fn decide_frm_flow(
    router_data: &mut FrmRecordReturnRouterData,
    state: &SessionState,
    connector: &FraudCheckConnectorData,
    call_connector_action: payments::CallConnectorAction,
    _platform: &domain::Platform,
) -> RouterResult<FrmRecordReturnRouterData> {
    let connector_integration: services::BoxedFrmConnectorIntegrationInterface<
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
        None,
    )
    .await
    .to_payment_failed_response()?;

    Ok(resp)
}
