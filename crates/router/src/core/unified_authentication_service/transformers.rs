use error_stack::Report;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_request_types::unified_authentication_service::{
        CtpServiceDetails, ServiceSessionIds, TransactionDetails, UasPreAuthenticationRequestData,
    },
};

use crate::core::payments::PaymentData;

#[cfg(feature = "v1")]
impl<F: Clone + Sync> TryFrom<PaymentData<F>> for UasPreAuthenticationRequestData {
    type Error = Report<ApiErrorResponse>;
    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let service_details = CtpServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.merchant_transaction_id.clone()),
                correlation_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.correlation_id.clone()),
                x_src_flow_id: payment_data
                    .service_details
                    .as_ref()
                    .and_then(|details| details.x_src_flow_id.clone()),
            }),
        };
        let currency = payment_data.payment_attempt.currency.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            },
        )?;

        let amount = payment_data.payment_attempt.net_amount.get_order_amount();
        let transaction_details = TransactionDetails { amount, currency };

        Ok(Self {
            service_details: Some(service_details),
            transaction_details: Some(transaction_details),
        })
    }
}
