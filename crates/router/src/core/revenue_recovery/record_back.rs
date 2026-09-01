//! Recording money that has gone back to the customer onto the billing connector.
//!
//! When a revenue recovery payment succeeds we tell the billing connector the invoice is
//! paid. If that money later goes back — a lost dispute, or a refund the merchant issued —
//! the billing connector must be told too, or its invoice stays marked paid and the
//! merchant's books are wrong.
//!
//! Both callers record against the *invoice*, not the payment transaction: the
//! transaction-level endpoint can only refund a payment's unapplied balance, which is zero
//! once the payment has paid an invoice.

use std::{marker::PhantomData, str::FromStr};

use common_utils::{ext_traits::ValueExt, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payments::{payment_attempt::PaymentAttempt, PaymentIntent},
    router_request_types::revenue_recovery::RecordBackPaymentMethod,
};
use router_env::logger;

use crate::{
    core::{
        errors::{self, RouterResult},
        payments,
    },
    routes::SessionState,
    services,
    types::{
        api::{ConnectorData, GetToken},
        domain,
    },
};

/// Whether a record-back actually reached the billing connector.
///
/// Kept distinct from `Result` so that the "nothing to do" paths — a payment that never
/// went through revenue recovery, or a connector with no offline-refund API — are not
/// counted as successes by the caller's metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordBackOutcome {
    /// The request was sent and the billing connector accepted it.
    Recorded,
    /// No request was sent.
    Skipped,
}

/// Record an amount back to the billing connector as an offline refund.
///
/// Skips silently when the attempt carries no billing connector transaction id — the
/// ordinary case for a payment that never went through revenue recovery — or when the
/// billing connector is not configured as supporting the call.
///
/// The caller passes the payment intent it already holds, so nothing is re-fetched.
pub async fn record_back_to_billing_connector(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_attempt: &PaymentAttempt,
    payment_intent: &PaymentIntent,
    amount: MinorUnit,
    payment_method: RecordBackPaymentMethod,
    comment: Option<String>,
) -> RouterResult<RecordBackOutcome> {
    let Some(billing_connector_transaction_id) = payment_attempt
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.revenue_recovery.as_ref())
        .and_then(|recovery| recovery.billing_connector_transaction_id.clone())
    else {
        // Not a recovery payment. Silent by design; this is the common case.
        return Ok(RecordBackOutcome::Skipped);
    };

    let recovery_metadata = payment_intent
        .feature_metadata
        .as_ref()
        .and_then(|metadata| metadata.payment_revenue_recovery_metadata.as_ref())
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("no recovery metadata on the payment intent")?;

    // The money moved on the payment connector's account; the record-back goes to the
    // billing connector, which is named on the intent's recovery metadata.
    let billing_mca = state
        .store
        .find_merchant_connector_account_by_id(&recovery_metadata.billing_connector_id, key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("billing merchant connector account not found")?;

    // Only some billing connectors expose an offline-refund API. Calling one that does not
    // would fail on the default unimplemented flow impl, so the supported set is configured
    // rather than inferred.
    if !state
        .conf
        .billing_connectors_dispute_record_back
        .billing_connectors_which_requires_dispute_record_back_call
        .contains(&billing_mca.connector_name)
    {
        logger::debug!(
            billing_connector = %billing_mca.connector_name,
            "billing connector is not configured for record back; skipping"
        );
        return Ok(RecordBackOutcome::Skipped);
    }

    let merchant_reference_id = payment_intent
        .merchant_reference_id
        .clone()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("no merchant reference id on the payment intent to record against")?;

    let connector_data = ConnectorData::get_connector_by_name(
        &state.conf.connectors,
        &billing_mca.connector_name.to_string(),
        GetToken::Connector,
        Some(billing_mca.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("invalid connector name on the billing merchant connector account")?;

    let connector_integration: services::BoxedRevenueRecoveryDisputeRecordBackInterface<
        hyperswitch_domain_models::router_flow_types::DisputeRecordBack,
        hyperswitch_domain_models::router_request_types::revenue_recovery::DisputeRecordBackRequest,
        hyperswitch_domain_models::router_response_types::revenue_recovery::DisputeRecordBackResponse,
    > = connector_data.connector.get_connector_integration();

    let router_data = construct_router_data(
        state,
        &billing_mca,
        &merchant_reference_id,
        &billing_connector_transaction_id,
        amount,
        payment_method,
        comment,
    )?;

    let response = services::execute_connector_processing_step(
        state,
        connector_integration,
        &router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("failed while recording back to the billing connector")?;

    response
        .response
        .map(|_| RecordBackOutcome::Recorded)
        .map_err(|error| {
            logger::error!(?error, "billing connector rejected the record back");
            error_stack::report!(errors::ApiErrorResponse::InternalServerError)
        })
}

#[allow(clippy::too_many_arguments)]
fn construct_router_data(
    state: &SessionState,
    billing_mca: &domain::MerchantConnectorAccount,
    merchant_reference_id: &common_utils::id_type::PaymentReferenceId,
    billing_connector_transaction_id: &str,
    amount: MinorUnit,
    payment_method: RecordBackPaymentMethod,
    comment: Option<String>,
) -> RouterResult<hyperswitch_domain_models::types::DisputeRecordBackRouterData> {
    let auth_type: crate::types::ConnectorAuthType =
        payments::helpers::MerchantConnectorAccountType::DbVal(Box::new(billing_mca.clone()))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to parse the billing connector auth type")?;

    let connector_name = billing_mca.get_connector_name_as_string();
    let connector = common_enums::connector_enums::Connector::from_str(connector_name.as_str())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("cannot resolve the connector from the connector name")?;

    let connector_params =
        hyperswitch_domain_models::connector_endpoints::Connectors::get_connector_params(
            &state.conf.connectors,
            connector,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(format!("no connector params for {connector} in this flow"))?;

    let router_data = hyperswitch_domain_models::router_data_v2::RouterDataV2 {
        flow: PhantomData::<hyperswitch_domain_models::router_flow_types::DisputeRecordBack>,
        tenant_id: state.tenant.tenant_id.clone(),
        resource_common_data:
            hyperswitch_domain_models::router_data_v2::flow_common_types::DisputeRecordBackData {
                connector_meta_data: billing_mca.metadata.clone(),
            },
        connector_auth_type: auth_type,
        request:
            hyperswitch_domain_models::router_request_types::revenue_recovery::DisputeRecordBackRequest {
                merchant_reference_id: merchant_reference_id.clone(),
                billing_connector_transaction_id: billing_connector_transaction_id.to_string(),
                payment_method,
                amount,
                refund_date: common_utils::date_time::now(),
                comment,
                connector_params,
            },
        response: Err(crate::types::ErrorResponse::default()),
    };

    <hyperswitch_domain_models::router_data_v2::flow_common_types::DisputeRecordBackData
        as hyperswitch_interfaces::connector_integration_interface::RouterDataConversion<_, _, _>>
        ::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("cannot construct the record back router data")
}
