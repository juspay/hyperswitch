use error_stack::Report;
use hyperswitch_domain_models::{
    errors::api_error_response::ApiErrorResponse,
    router_request_types::unified_authentication_service::{
        CtpServiceDetails, ServiceSessionIds, TransactionDetails, UasConfirmationRequestData,
        UasPreAuthenticationRequestData, UasWebhookRequestData,
    },
};
use hyperswitch_interfaces::webhooks::IncomingWebhookRequestDetails;

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

#[cfg(feature = "v1")]
impl<F: Clone + Sync> TryFrom<PaymentData<F>> for UasConfirmationRequestData {
    type Error = Report<ApiErrorResponse>;
    fn try_from(payment_data: PaymentData<F>) -> Result<Self, Self::Error> {
        let currency = payment_data.payment_attempt.currency.ok_or(
            ApiErrorResponse::MissingRequiredField {
                field_name: "currency",
            },
        )?;

        let current_time = common_utils::date_time::now();

        let payment_attempt_status = payment_data.payment_attempt.status;

        let (checkout_event_status, confirmation_reason) =
            get_checkout_event_status_and_reason(payment_attempt_status);

        let ctp_details = payment_data.service_details.clone();

        Ok(Self {
            x_src_flow_id: payment_data
                .service_details
                .as_ref()
                .and_then(|details| details.x_src_flow_id.clone()),
            transaction_amount: payment_data.payment_attempt.net_amount.get_order_amount(),
            transaction_currency: currency,
            checkout_event_type: Some("01".to_string()),
            checkout_event_status: checkout_event_status.clone(),
            confirmation_status: checkout_event_status.clone(),
            confirmation_reason,
            confirmation_timestamp: Some(current_time),
            network_authorization_code: Some("01".to_string()),
            network_transaction_identifier: Some("01".to_string()),
            correlation_id: ctp_details
                .clone()
                .and_then(|details| details.correlation_id),
            merchant_transaction_id: ctp_details
                .and_then(|details| details.merchant_transaction_id),
        })
    }
}

fn get_checkout_event_status_and_reason(
    attempt_status: common_enums::AttemptStatus,
) -> (Option<String>, Option<String>) {
    match attempt_status {
        common_enums::AttemptStatus::Charged | common_enums::AttemptStatus::Authorized => (
            Some("02".to_string()),
            Some("Approval Code received".to_string()),
        ),
        _ => (
            Some("03".to_string()),
            Some("No Approval Code received".to_string()),
        ),
    }
}

pub fn get_webhook_request_data_for_uas<'a>(
    request: IncomingWebhookRequestDetails<'a>,
) -> UasWebhookRequestData {
    UasWebhookRequestData {
        body: request.body.to_vec(),
    }
}

// #[cfg(feature = "v1")]
// impl<'a> TryFrom<IncomingWebhookRequestDetails<'a>> for UasWebhookRequestData {
//     type Error = Report<ApiErrorResponse>;
//     fn try_from( incoming_webhook_request: IncomingWebhookRequestDetails<'a>,
//     ) -> Result<Self, Self::Error> {
//         Ok(Self {
//             body: incoming_webhook_request.body.to_vec()
//         })
//     }
// }
