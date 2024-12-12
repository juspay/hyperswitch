use error_stack::{Report, ResultExt};
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_request_types::unified_authentication_service::{
        ServiceDetails, ServiceSessionIds, TransactionDetails, UasPreAuthenticationRequestData,
    },
};

use crate::core::payments::PaymentData;

#[cfg(feature = "v1")]
impl<F: Clone + Sync> TryFrom<PaymentData<F>> for UasPreAuthenticationRequestData {
    type Error = Report<ApiErrorResponse>;
    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let service_details = ServiceDetails {
            service_session_ids: Some(ServiceSessionIds {
                merchant_transaction_id: None,
                correlation_id: None,
                x_src_flow_id: None,
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
            source_authentication_id: payment_data
                .authentication
                .ok_or(ApiErrorResponse::InternalServerError)
                .attach_printable("missing payment_data.authentication")?
                .authentication_id,
        })
    }
}
