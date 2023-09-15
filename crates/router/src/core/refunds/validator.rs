use common_utils::pii;
use error_stack::{report, IntoReport, ResultExt};
use router_env::{instrument, tracing};
use time::PrimitiveDateTime;

use crate::{
    core::{
        errors::{self, CustomResult, RouterResult},
        utils as core_utils,
    },
    db::StorageInterface,
    logger,
    types::{
        api::refunds,
        domain,
        storage::{self, enums},
    },
    utils::{self, OptionExt},
};

// Limit constraints for refunds list flow
pub const LOWER_LIMIT: i64 = 1;
pub const UPPER_LIMIT: i64 = 100;
pub const DEFAULT_LIMIT: i64 = 10;

pub struct ValidateRefundResult {
    pub refund_id: String,
    pub refund_amount: i64,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub reason: Option<String>,
    pub refund_type: refunds::RefundType,
    pub connecter_transaction_id: String,
    pub currency: enums::Currency,
    pub connector: String,
}

#[async_trait::async_trait]
pub trait ValidateRefundRequest {
    async fn validate_request(
        self,
        state: &crate::AppState,
        payment_intent: &storage::PaymentIntent,
        payment_attempt: &storage::PaymentAttempt,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<ValidateRefundResult>;
}

#[async_trait::async_trait]
impl ValidateRefundRequest for refunds::RefundRequest {
    async fn validate_request(
        self,
        state: &crate::AppState,
        payment_intent: &storage::PaymentIntent,
        payment_attempt: &storage::PaymentAttempt,
        merchant_account: &domain::MerchantAccount,
    ) -> RouterResult<ValidateRefundResult> {
        utils::when(
            payment_intent.status != enums::IntentStatus::Succeeded,
            || {
                Err(report!(errors::ApiErrorResponse::PaymentNotSucceeded)
                    .attach_printable("unable to refund for a unsuccessful payment intent"))
            },
        )?;

        let amount = self.amount.unwrap_or(
            payment_intent
                .amount_captured
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .into_report()
                .attach_printable("amount captured is none in a successful payment")?,
        );

        utils::when(amount <= 0, || {
            Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "amount".to_string(),
                expected_format: "positive integer".to_string()
            })
            .attach_printable("amount less than or equal to zero"))
        })?;

        let predicate = self
            .merchant_id
            .as_ref()
            .map(|merchant_id| merchant_id != &merchant_account.merchant_id);

        let currency = payment_attempt.currency.get_required_value("currency")?;
        let connecter_transaction_id = payment_attempt.clone().connector_transaction_id.ok_or_else(|| {
                report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Transaction in invalid. Missing field \"connector_transaction_id\" in payment_attempt.")
            })?;

        utils::when(predicate.unwrap_or(false), || {
            Err(report!(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string()
            })
            .attach_printable("invalid merchant_id in request"))
        })?;

        validate_payment_order_age(&payment_intent.created_at, state.conf.refund.max_age)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "created_at".to_string(),
                expected_format: format!(
                    "created_at not older than {} days",
                    state.conf.refund.max_age,
                ),
            })?;

        let connector = payment_attempt
            .connector
            .clone()
            .ok_or(errors::ApiErrorResponse::InternalServerError)
            .into_report()
            .attach_printable("No connector populated in payment attempt")?;

        let refund_id = core_utils::get_or_generate_id("refund_id", &self.refund_id, "ref")?;

        Ok(ValidateRefundResult {
            refund_id,
            refund_amount: amount,
            currency,
            connecter_transaction_id,
            connector,
            metadata: self.metadata,
            reason: self.reason,
            refund_type: self.refund_type.unwrap_or_default(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RefundValidationError {
    #[error("The payment attempt was not successful")]
    UnsuccessfulPaymentAttempt,
    #[error("The refund amount exceeds the payment amount")]
    RefundAmountExceedsPaymentAmount,
    #[error("The order has expired")]
    OrderExpired,
    #[error("The maximum refund count for this payment attempt")]
    MaxRefundCountReached,
    #[error("There is already another refund request for this payment attempt")]
    DuplicateRefund,
}

#[instrument(skip_all)]
pub fn validate_success_transaction(
    transaction: &storage::PaymentAttempt,
) -> CustomResult<(), RefundValidationError> {
    if transaction.status != enums::AttemptStatus::Charged {
        Err(report!(RefundValidationError::UnsuccessfulPaymentAttempt))?
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn validate_refund_amount(
    payment_attempt_amount: i64, // &storage::PaymentAttempt,
    all_refunds: &[storage::Refund],
    refund_amount: i64,
) -> CustomResult<(), RefundValidationError> {
    let total_refunded_amount: i64 = all_refunds
        .iter()
        .filter_map(|refund| {
            if refund.refund_status != enums::RefundStatus::Failure
                && refund.refund_status != enums::RefundStatus::TransactionFailure
            {
                Some(refund.refund_amount)
            } else {
                None
            }
        })
        .sum();

    utils::when(
        refund_amount > (payment_attempt_amount - total_refunded_amount),
        || {
            Err(report!(
                RefundValidationError::RefundAmountExceedsPaymentAmount
            ))
        },
    )
}

#[instrument(skip_all)]
pub fn validate_payment_order_age(
    created_at: &PrimitiveDateTime,
    refund_max_age: i64,
) -> CustomResult<(), RefundValidationError> {
    let current_time = common_utils::date_time::now();

    utils::when(
        (current_time - *created_at).whole_days() > refund_max_age,
        || Err(report!(RefundValidationError::OrderExpired)),
    )
}

#[instrument(skip_all)]
pub fn validate_maximum_refund_against_payment_attempt(
    all_refunds: &[storage::Refund],
    refund_max_attempts: usize,
) -> CustomResult<(), RefundValidationError> {
    utils::when(all_refunds.len() > refund_max_attempts, || {
        Err(report!(RefundValidationError::MaxRefundCountReached))
    })
}

#[instrument(skip(db))]
pub async fn validate_uniqueness_of_refund_id_against_merchant_id(
    db: &dyn StorageInterface,
    payment_id: &str,
    merchant_id: &str,
    refund_id: &str,
    storage_scheme: enums::MerchantStorageScheme,
) -> RouterResult<Option<storage::Refund>> {
    let refund = db
        .find_refund_by_merchant_id_refund_id(merchant_id, refund_id, storage_scheme)
        .await;
    logger::debug!(?refund);
    match refund {
        Err(err) => {
            if err.current_context().is_db_not_found() {
                // Empty vec should be returned by query in case of no results, this check exists just
                // to be on the safer side. Fixed this, now vector is not returned but should check the flow in detail later.
                Ok(None)
            } else {
                Err(err
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed while finding refund, database error"))
            }
        }

        Ok(refund) => {
            if refund.payment_id == payment_id {
                Ok(Some(refund))
            } else {
                Ok(None)
            }
        }
    }
}

pub fn validate_refund_list(limit: Option<i64>) -> CustomResult<i64, errors::ApiErrorResponse> {
    match limit {
        Some(limit_val) => {
            if !(LOWER_LIMIT..=UPPER_LIMIT).contains(&limit_val) {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "limit should be in between 1 and 100".to_string(),
                }
                .into())
            } else {
                Ok(limit_val)
            }
        }
        None => Ok(DEFAULT_LIMIT),
    }
}

pub fn validate_for_valid_refunds(
    payment_attempt: &data_models::payments::payment_attempt::PaymentAttempt,
    connector: api_models::enums::Connector,
) -> RouterResult<()> {
    let payment_method = payment_attempt
        .payment_method
        .as_ref()
        .get_required_value("payment_method")?;

    match payment_method {
        diesel_models::enums::PaymentMethod::PayLater
        | diesel_models::enums::PaymentMethod::Wallet => {
            let payment_method_type = payment_attempt
                .payment_method_type
                .get_required_value("payment_method_type")?;

            utils::when(
                matches!(
                    (connector, payment_method_type),
                    (
                        api_models::enums::Connector::Braintree,
                        diesel_models::enums::PaymentMethodType::Paypal,
                    ) | (
                        api_models::enums::Connector::Klarna,
                        diesel_models::enums::PaymentMethodType::Klarna
                    )
                ),
                || {
                    Err(errors::ApiErrorResponse::RefundNotPossible {
                        connector: connector.to_string(),
                    })
                },
            )
            .into_report()
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod refund_validate_tests {
    use api_models::{
        enums::{CountryAlpha2, Currency, IntentStatus},
        refunds::RefundRequest,
    };
    use data_models::payments::payment_intent::{PaymentIntentInterface, PaymentIntentNew};
    use time::macros::datetime;

    use super::*;
    use crate::{
        configs::settings::Settings,
        db::{
            merchant_account::MerchantAccountInterface,
            merchant_key_store::MerchantKeyStoreInterface,
            payment_attempt::PaymentAttemptInterface, MasterKeyInterface, MockDb,
        },
        routes, services,
        types::{
            domain::{self, MerchantAccount},
            storage::{enums::MerchantStorageScheme, PaymentAttemptNew},
        },
    };

    fn make_payment_attempt(merchant_id: String, payment_id: String) -> PaymentAttemptNew {
        PaymentAttemptNew {
            payment_id,
            connector: Some("stripe".to_string()),
            merchant_id,
            currency: Some(Currency::USD),
            ..PaymentAttemptNew::default()
        }
    }
    fn make_payment_intent(merchant_id: String, payment_id: String) -> PaymentIntentNew {
        PaymentIntentNew {
            payment_id,
            merchant_id,
            status: IntentStatus::Succeeded,
            amount: 20,
            currency: Some(Currency::USD),
            amount_captured: Some(20),
            customer_id: None,
            description: None,
            return_url: None,
            metadata: None,
            connector_id: None,
            shipping_address_id: None,
            billing_address_id: None,
            statement_descriptor_name: None,
            statement_descriptor_suffix: None,
            created_at: None,
            modified_at: None,
            last_synced: None,
            setup_future_usage: None,
            off_session: None,
            client_secret: None,
            active_attempt_id: "active_123".to_string(),
            business_country: CountryAlpha2::US,
            business_label: "bus".to_string(),
            order_details: None,
            allowed_payment_method_types: None,
            connector_metadata: None,
            feature_metadata: None,
            attempt_count: 1,
            profile_id: None,
        }
    }

    fn make_merchant_account(merchant_id: String) -> MerchantAccount {
        MerchantAccount {
            id: Some(0),
            merchant_id,
            return_url: None,
            enable_payment_response_hash: false,
            payment_response_hash_key: None,
            redirect_to_merchant_with_http_post: false,
            merchant_name: None,
            merchant_details: None,
            webhook_details: None,
            sub_merchants_enabled: None,
            parent_merchant_id: None,
            publishable_key: None,
            storage_scheme: MerchantStorageScheme::PostgresOnly,
            locker_id: None,
            metadata: None,
            routing_algorithm: None,
            primary_business_details: Default::default(),
            frm_routing_algorithm: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
            intent_fulfillment_time: None,
            payout_routing_algorithm: None,
            organization_id: None,
            is_recon_enabled: false,
            default_profile: None,
        }
    }

    #[allow(clippy::unwrap_used)]
    async fn make_merchant_key_store(
        merchant_id: String,
        master_key: &[u8],
    ) -> domain::MerchantKeyStore {
        domain::MerchantKeyStore {
            merchant_id,
            key: domain::types::encrypt(
                services::generate_aes256_key().unwrap().to_vec().into(),
                master_key,
            )
            .await
            .unwrap(),
            created_at: datetime!(2023-02-01 0:00),
        }
    }

    fn make_refund_request(payment_id: String, merchant_id: String) -> RefundRequest {
        RefundRequest {
            payment_id,
            merchant_id: Some(merchant_id),
            amount: Some(20),
            ..RefundRequest::default()
        }
    }

    #[allow(clippy::unwrap_used)]
    #[tokio::test]
    async fn validate_request_ok() {
        let mockdb = MockDb::new(&Default::default()).await;
        let master_key = mockdb.get_master_key();

        let (tx, _) = tokio::sync::oneshot::channel();
        let state = routes::AppState::new(Settings::default(), tx).await;

        let merchant_id = "merchant_123";
        let payment_id = "payment_123";

        let merchant_account = make_merchant_account(merchant_id.to_string());

        let key_store = mockdb
            .insert_merchant_key_store(
                make_merchant_key_store(merchant_id.to_string(), master_key).await,
                &master_key.to_vec().into(),
            )
            .await
            .unwrap();

        let merchant_account = mockdb
            .insert_merchant(merchant_account, &key_store)
            .await
            .unwrap();

        let payment_intent = make_payment_intent(merchant_id.to_string(), payment_id.to_string());
        let payment_intent = mockdb
            .insert_payment_intent(payment_intent, MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        let payment_attempt = make_payment_attempt(merchant_id.to_string(), payment_id.to_string());
        let mut payment_attempt = mockdb
            .insert_payment_attempt(payment_attempt, MerchantStorageScheme::PostgresOnly)
            .await
            .unwrap();

        payment_attempt.connector_transaction_id = Some("connector_123".to_string());

        let refund_request = make_refund_request(payment_id.to_string(), merchant_id.to_string());

        assert!(refund_request
            .validate_request(&state, &payment_intent, &payment_attempt, &merchant_account)
            .await
            .is_ok());
    }
}
