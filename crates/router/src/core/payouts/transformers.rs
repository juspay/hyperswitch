use api_models::payouts::PayoutAttemptResponse;

use crate::types::{
    api, domain,
    storage::{self},
    transformers::ForeignFrom,
};

impl ForeignFrom<(&storage::Payouts, &storage::PayoutAttempt)> for PayoutAttemptResponse {
    fn foreign_from(item: (&storage::Payouts, &storage::PayoutAttempt)) -> Self {
        let (payout, payout_attempt) = item;
        Self {
            attempt_id: payout_attempt.payout_attempt_id.clone(),
            status: payout_attempt.status,
            amount: payout.amount,
            currency: Some(payout.destination_currency),
            connector: payout_attempt.connector.clone(),
            error_code: payout_attempt.error_code.clone(),
            error_message: payout_attempt.error_message.clone(),
            payment_method: Some(payout.payout_type),
            payout_method_type: None,
            connector_transaction_id: Some(payout_attempt.connector_payout_id.clone()),
            cancellation_reason: None,
            unified_code: None,
            unified_message: None,
        }
    }
}

impl ForeignFrom<(storage::Payouts, storage::PayoutAttempt, domain::Customer)>
    for api::PayoutCreateResponse
{
    fn foreign_from(item: (storage::Payouts, storage::PayoutAttempt, domain::Customer)) -> Self {
        let (payout, payout_attempt, customer) = item;
        let attempts = vec![PayoutAttemptResponse::foreign_from((
            &payout,
            &payout_attempt,
        ))];
        Self {
            payout_id: payout.payout_id,
            merchant_id: payout.merchant_id,
            amount: payout.amount,
            currency: payout.destination_currency,
            connector: payout_attempt.connector,
            payout_type: payout.payout_type,
            customer_id: customer.customer_id,
            auto_fulfill: payout.auto_fulfill,
            email: customer.email,
            name: customer.name,
            phone: customer.phone,
            phone_country_code: customer.phone_country_code,
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
            attempts: Some(attempts),
            billing: None,
            client_secret: None,
        }
    }
}
