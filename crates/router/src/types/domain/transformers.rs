use storage_models as storage;

use crate::types::{domain, transformers::Foreign};

type F<T> = Foreign<T>;

impl From<F<storage::payment_attempt::PaymentAttempt>> for F<domain::PaymentAttempt> {
    fn from(item: F<storage::payment_attempt::PaymentAttempt>) -> Self {
        let item = item.0;
        Self(domain::PaymentAttempt {
            id: item.id,
            payment_id: item.payment_id.into(),
            merchant_id: item.merchant_id,
            attempt_id: item.attempt_id.into(),
            status: item.status,
            amount: item.amount,
            currency: item.currency,
            save_to_locker: item.save_to_locker,
            connector: item.connector,
            error_message: item.error_message,
            offer_amount: item.offer_amount,
            surcharge_amount: item.surcharge_amount,
            tax_amount: item.tax_amount,
            payment_method_id: item.payment_method_id,
            payment_method: item.payment_method,
            payment_flow: item.payment_flow,
            redirect: item.redirect,
            connector_transaction_id: item.connector_transaction_id,
            capture_method: item.capture_method,
            capture_on: item.capture_on,
            confirm: item.confirm,
            authentication_type: item.authentication_type,
            created_at: item.created_at,
            modified_at: item.modified_at,
            last_synced: item.last_synced,
            cancellation_reason: item.cancellation_reason,
            amount_to_capture: item.amount_to_capture,
            mandate_id: item.mandate_id,
            browser_info: item.browser_info,
            payment_token: item.payment_token,
            error_code: item.error_code,
        })
    }
}

impl From<F<domain::PaymentAttempt>> for F<storage::payment_attempt::PaymentAttempt> {
    fn from(item: F<domain::PaymentAttempt>) -> Self {
        let item = item.0;
        Self(storage::payment_attempt::PaymentAttempt {
            id: item.id,
            payment_id: item.payment_id.into(),
            merchant_id: item.merchant_id,
            attempt_id: item.attempt_id.into(),
            status: item.status,
            amount: item.amount,
            currency: item.currency,
            save_to_locker: item.save_to_locker,
            connector: item.connector,
            error_message: item.error_message,
            offer_amount: item.offer_amount,
            surcharge_amount: item.surcharge_amount,
            tax_amount: item.tax_amount,
            payment_method_id: item.payment_method_id,
            payment_method: item.payment_method,
            payment_flow: item.payment_flow,
            redirect: item.redirect,
            connector_transaction_id: item.connector_transaction_id,
            capture_method: item.capture_method,
            capture_on: item.capture_on,
            confirm: item.confirm,
            authentication_type: item.authentication_type,
            created_at: item.created_at,
            modified_at: item.modified_at,
            last_synced: item.last_synced,
            cancellation_reason: item.cancellation_reason,
            amount_to_capture: item.amount_to_capture,
            mandate_id: item.mandate_id,
            browser_info: item.browser_info,
            payment_token: item.payment_token,
            error_code: item.error_code,
        })
    }
}
