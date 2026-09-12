use async_trait::async_trait;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;

use super::{ConstructFlowSpecificData, FeatureFrm};
use crate::{
    core::{
        errors::{ConnectorErrorExt, RouterResult},
        fraud_check::types::PayoutFrmData,
        payments::{self, helpers},
    },
    errors, services,
    services::connector_integration_interface::{ConnectorEnum, ConnectorIntegrationEnum},
    types::{
        api::fraud_check::{self as frm_api, FraudCheckConnectorData},
        domain,
        fraud_check::{FraudCheckPayoutData, FraudCheckResponseData, FrmPayoutRouterData},
        storage::enums as storage_enums,
        ConnectorAuthType, MerchantRecipientData, ResponseId, RouterData,
    },
    SessionState,
};

#[async_trait]
impl ConstructFlowSpecificData<frm_api::PoFrm, FraudCheckPayoutData, FraudCheckResponseData>
    for PayoutFrmData
{
    async fn construct_router_data<'a>(
        &self,
        state: &SessionState,
        connector_id: &str,
        processor: &domain::Processor,
        merchant_connector_account: &helpers::MerchantConnectorAccountType,
        _merchant_recipient_data: Option<MerchantRecipientData>,
        header_payload: Option<hyperswitch_domain_models::payments::HeaderPayload>,
        _payment_method: Option<common_enums::PaymentMethod>,
        _payment_method_type: Option<common_enums::PaymentMethodType>,
    ) -> RouterResult<RouterData<frm_api::PoFrm, FraudCheckPayoutData, FraudCheckResponseData>>
    {
        let connector_auth_type: ConnectorAuthType = merchant_connector_account
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: "ConnectorAuthType".to_string(),
            })?;

        let payment_method_type = self.payout_method_data.as_ref().map(From::from).or(self
            .payout_attempt
            .additional_payout_method_data
            .as_ref()
            .map(From::from));

        let billing_address = self.billing_address.clone();
        let address = hyperswitch_domain_models::payment_address::PaymentAddress::new(
            None,
            billing_address,
            None,
            None,
        );

        Ok(RouterData {
            flow: std::marker::PhantomData,
            merchant_id: processor.get_account().get_id().clone(),
            customer_id: self.customer_details.as_ref().map(|c| c.get_id().clone()),
            connector_customer: None,
            connector: connector_id.to_string(),
            payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("frm")
                .get_string_repr()
                .to_owned(),
            attempt_id: self.payout_attempt.payout_attempt_id.clone(),
            tenant_id: state.tenant.tenant_id.clone(),
            status: storage_enums::AttemptStatus::Pending,
            payment_method: storage_enums::PaymentMethod::default(),
            payment_method_type,
            connector_auth_type,
            description: None,
            address,
            auth_type: storage_enums::AuthenticationType::default(),
            connector_meta_data: None,
            connector_wallets_details: None,
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: FraudCheckPayoutData {
                amount: self.amount,
                currency: self.currency,
            },
            response: Ok(FraudCheckResponseData::TransactionResponse {
                resource_id: ResponseId::NoResponseId,
                status: storage_enums::FraudCheckStatus::Pending,
                connector_metadata: None,
                reason: None,
                score: None,
            }),
            connector_request_reference_id: uuid::Uuid::new_v4().to_string(),
            payout_method_data: None,
            quote_id: None,
            test_mode: merchant_connector_account.is_test_mode_on(),
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payout_id: Some(self.payout_attempt.payout_id.get_string_repr().to_owned()),
            connector_response: None,
            payment_method_status: None,
            minor_amount_captured: None,
            minor_amount_capturable: None,
            authorized_amount: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload,
            connector_mandate_request_reference_id: None,
            l2_l3_data: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            customer_document_details: None,
            customer_date_of_birth: None,
            feature_data: None,
            sender_payment_instrument_id: None,
            connector_returned_payment_method_details: None,
        })
    }
}

#[async_trait]
impl FeatureFrm<frm_api::PoFrm, FraudCheckPayoutData> for FrmPayoutRouterData {
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

async fn decide_frm_flow(
    router_data: &mut FrmPayoutRouterData,
    state: &SessionState,
    connector: &FraudCheckConnectorData,
    call_connector_action: payments::CallConnectorAction,
    _platform: &domain::Platform,
) -> RouterResult<FrmPayoutRouterData> {
    let connector_integration: services::BoxedFrmConnectorIntegrationInterface<
        frm_api::PoFrm,
        FraudCheckPayoutData,
        FraudCheckResponseData,
    > = match &connector.connector {
        ConnectorEnum::Old(connector) => Box::new(ConnectorIntegrationEnum::Old(
            connector.get_connector_integration(),
        )),
        ConnectorEnum::New(_) => {
            return Err(error_stack::report!(
                errors::ApiErrorResponse::InternalServerError
            ))
            .attach_printable("Payout FRM requires a legacy connector integration")
        }
    };

    services::execute_connector_processing_step(
        state,
        connector_integration,
        router_data,
        call_connector_action,
        None,
        None,
    )
    .await
    .to_payment_failed_response()
}
