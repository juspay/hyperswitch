use crate::types::{
    api, domain, storage,
    transformers::{ForeignFrom, ForeignInto},
};

impl
    ForeignFrom<(
        storage::Payouts,
        storage::PayoutAttempt,
        Option<domain::Customer>,
    )> for api::PayoutCreateResponse
{
    fn foreign_from(
        item: (
            storage::Payouts,
            storage::PayoutAttempt,
            Option<domain::Customer>,
        ),
    ) -> Self {
        let (payout, payout_attempt, customer) = item;
        let attempt = api::PayoutAttemptResponse {
            attempt_id: payout_attempt.payout_attempt_id,
            status: payout_attempt.status,
            amount: payout.amount,
            currency: Some(payout.destination_currency),
            connector: payout_attempt.connector.clone(),
            error_code: payout_attempt.error_code.clone(),
            error_message: payout_attempt.error_message.clone(),
            payment_method: payout.payout_type,
            payout_method_type: None,
            connector_transaction_id: payout_attempt.connector_payout_id,
            cancellation_reason: None,
            unified_code: None,
            unified_message: None,
        };
        Self {
            payout_id: payout.payout_id,
            merchant_id: payout.merchant_id,
            merchant_connector_id: payout_attempt.merchant_connector_id,
            amount: payout.amount,
            currency: payout.destination_currency,
            connector: payout_attempt.connector,
            payout_type: payout.payout_type,
            auto_fulfill: payout.auto_fulfill,
            customer_id: customer.as_ref().map(|cust| cust.get_customer_id()),
            customer: customer.as_ref().map(|cust| cust.foreign_into()),
            return_url: payout.return_url,
            business_country: payout_attempt.business_country,
            business_label: payout_attempt.business_label,
            description: payout.description,
            entity_type: payout.entity_type,
            recurring: payout.recurring,
            metadata: payout.metadata,
            status: payout_attempt.status,
            error_message: payout_attempt.error_message,
            error_code: payout_attempt.error_code,
            profile_id: payout.profile_id,
            created: Some(payout.created_at),
            connector_transaction_id: attempt.connector_transaction_id.clone(),
            priority: payout.priority,
            attempts: Some(vec![attempt]),
            billing: None,
            client_secret: None,
            payout_link: None,
            email: customer
                .as_ref()
                .and_then(|customer| customer.email.clone()),
            name: customer.as_ref().and_then(|customer| customer.name.clone()),
            phone: customer
                .as_ref()
                .and_then(|customer| customer.phone.clone()),
            phone_country_code: customer
                .as_ref()
                .and_then(|customer| customer.phone_country_code.clone()),
        }
    }
}
