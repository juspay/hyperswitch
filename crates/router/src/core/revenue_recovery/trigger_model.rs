use api_models::payments::RecoveryPaymentsCreate;
use common_types::payments::RecoveryConnector;
use common_utils::id_type;
use diesel_models::business_profile::RevenueRecoveryConfig;
use error_stack::report;

use crate::{
    core::errors::{self, RouterResult},
    utils::{self, OptionExt},
};

/// What a recovery API request needs under the trigger model of the profile, worked out at the very
/// start of the API. The model comes from the profile alone. The fields of the request are only
/// checked against it, and from here on each model has exactly what it needs.
#[derive(Debug)]
pub(super) enum RecoveryTrigger<'a> {
    /// Hyperswitch triggers the retries through these connector accounts.
    Internal {
        billing_merchant_connector_id: id_type::MerchantConnectorAccountId,
        payment_merchant_connector_id: id_type::MerchantConnectorAccountId,
    },
    /// The merchant triggers the retries on this processor, within these limits.
    External {
        connector: RecoveryConnector,
        retry_config: &'a RevenueRecoveryConfig,
    },
}

impl<'a> RecoveryTrigger<'a> {
    pub(super) fn from_profile_settings(
        trigger_model: common_enums::RevenueRecoveryTriggerModel,
        retry_config: Option<&'a RevenueRecoveryConfig>,
        request: &RecoveryPaymentsCreate,
    ) -> RouterResult<Self> {
        match trigger_model {
            common_enums::RevenueRecoveryTriggerModel::InternalTriggers => {
                utils::when(request.connector.is_some(), || {
                    Err(errors::ApiErrorResponse::InvalidRequestData {
                        message: "`connector` must not be sent when the `revenue_recovery_trigger_model` of the profile is `internal_triggers`".to_owned(),
                    })
                })?;

                Ok(Self::Internal {
                    billing_merchant_connector_id: request
                        .billing_merchant_connector_id
                        .clone()
                        .get_required_value("billing_merchant_connector_id")?,
                    payment_merchant_connector_id: request
                        .payment_merchant_connector_id
                        .clone()
                        .get_required_value("payment_merchant_connector_id")?,
                })
            }
            common_enums::RevenueRecoveryTriggerModel::ExternalTriggers => {
                utils::when(
                    request.billing_merchant_connector_id.is_some()
                        || request.payment_merchant_connector_id.is_some(),
                    || {
                        Err(errors::ApiErrorResponse::InvalidRequestData {
                            message: "`billing_merchant_connector_id` and `payment_merchant_connector_id` must not be sent when the `revenue_recovery_trigger_model` of the profile is `external_triggers`".to_owned(),
                        })
                    },
                )?;

                let connector = request.connector.get_required_value("connector")?;

                // Profiles are refused without it, this holds for one saved before that check.
                let retry_config = retry_config.ok_or_else(|| {
                    report!(errors::ApiErrorResponse::PreconditionFailed {
                        message: "The `revenue_recovery_config` of the profile is required when its `revenue_recovery_trigger_model` is `external_triggers`".to_owned(),
                    })
                })?;

                Ok(Self::External {
                    connector,
                    retry_config,
                })
            }
        }
    }
}

/// An invoice stays with the trigger model it was first recorded under, whatever the profile is
/// set to now. So a profile that changes models cannot have Hyperswitch and the merchant both
/// retrying the same invoice.
pub(super) fn validate_invoice_trigger_model(
    invoice_trigger_model: Option<common_enums::RevenueRecoveryTriggerModel>,
    profile_trigger_model: common_enums::RevenueRecoveryTriggerModel,
) -> RouterResult<()> {
    match invoice_trigger_model {
        Some(invoice_trigger_model) if invoice_trigger_model != profile_trigger_model => {
            Err(report!(errors::ApiErrorResponse::PreconditionFailed {
                message: format!(
                    "The invoice was first recorded under `{invoice_trigger_model}`, and the `revenue_recovery_trigger_model` of the profile is `{profile_trigger_model}`"
                ),
            }))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use common_enums::RevenueRecoveryTriggerModel::{ExternalTriggers, InternalTriggers};
    use common_types::payments::RecoveryConnector;
    use diesel_models::business_profile::RevenueRecoveryConfig;

    use super::{validate_invoice_trigger_model, RecoveryTrigger};
    use crate::core::errors::ApiErrorResponse;

    fn recovery_request(
        connector_fields: serde_json::Value,
    ) -> api_models::payments::RecoveryPaymentsCreate {
        let mut request = serde_json::json!({
            "amount_details": { "order_amount": 6540, "currency": "USD" },
            "merchant_reference_id": "invoice_mbabizu24mvu3mela5njyh",
            "transaction_status": "failure",
            "payment_method_type": "card",
            "payment_method_sub_type": "credit",
            "connector_customer_id": "cust_12345",
            "connector_transaction_id": "993672945374576J",
            "transaction_created_at": "2022-09-10T10:11:12Z",
            "billing_started_at": "2022-09-10T10:11:12Z",
            "payment_method_data": {
                "primary_processor_payment_method_token": "token_1234",
                "payment_method_metadata": {
                    "card_network": "Visa",
                    "card_type": "credit",
                    "last4": "4242",
                    "card_isin": "424242",
                    "card_extended_bin": "42424242",
                    "card_exp_month": "01",
                    "card_exp_year": "2026"
                }
            }
        });
        if let (Some(request), Some(connector_fields)) =
            (request.as_object_mut(), connector_fields.as_object())
        {
            request.extend(connector_fields.clone());
        }
        serde_json::from_value(request).expect("valid recovery request")
    }

    fn is_missing_field(error: &ApiErrorResponse, field: &str) -> bool {
        matches!(
            error,
            ApiErrorResponse::MissingRequiredField { field_name } if *field_name == field
        )
    }

    fn retry_config() -> RevenueRecoveryConfig {
        RevenueRecoveryConfig {
            max_retry_count: 15,
            billing_connector_retry_threshold: 0,
            max_hybrid_cascading_retry_count: 0,
            grace_period_days: 30,
            enable_partial_authorization: false,
            max_hard_decline_retries: 0,
        }
    }

    #[test]
    fn internal_triggers_need_the_billing_account() {
        let request = recovery_request(serde_json::json!({
            "payment_merchant_connector_id": "mca_payment",
        }));

        let error = RecoveryTrigger::from_profile_settings(InternalTriggers, None, &request)
            .expect_err("billing account is required");

        assert!(is_missing_field(
            error.current_context(),
            "billing_merchant_connector_id"
        ));
    }

    #[test]
    fn internal_triggers_need_the_payment_account() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
        }));

        let error = RecoveryTrigger::from_profile_settings(InternalTriggers, None, &request)
            .expect_err("payment account is required");

        assert!(is_missing_field(
            error.current_context(),
            "payment_merchant_connector_id"
        ));
    }

    #[test]
    fn internal_triggers_do_not_accept_a_reported_connector() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
            "payment_merchant_connector_id": "mca_payment",
            "connector": "stripe",
        }));

        let error = RecoveryTrigger::from_profile_settings(InternalTriggers, None, &request)
            .expect_err("the processor comes from the payment account");

        assert!(matches!(
            error.current_context(),
            ApiErrorResponse::InvalidRequestData { .. }
        ));
    }

    #[test]
    fn internal_triggers_use_the_accounts_of_the_request() {
        let request = recovery_request(serde_json::json!({
            "billing_merchant_connector_id": "mca_billing",
            "payment_merchant_connector_id": "mca_payment",
        }));

        let trigger = RecoveryTrigger::from_profile_settings(InternalTriggers, None, &request)
            .expect("both accounts are sent");

        let RecoveryTrigger::Internal {
            billing_merchant_connector_id,
            payment_merchant_connector_id,
        } = trigger
        else {
            panic!("internal triggers keep the accounts of the request");
        };
        assert_eq!(
            billing_merchant_connector_id.get_string_repr(),
            "mca_billing"
        );
        assert_eq!(
            payment_merchant_connector_id.get_string_repr(),
            "mca_payment"
        );
    }

    #[test]
    fn external_triggers_need_the_retry_config_of_the_profile() {
        let request = recovery_request(serde_json::json!({ "connector": "stripe" }));

        let error = RecoveryTrigger::from_profile_settings(ExternalTriggers, None, &request)
            .expect_err("the limits come from the profile");

        assert!(matches!(
            error.current_context(),
            ApiErrorResponse::PreconditionFailed { .. }
        ));
    }

    #[test]
    fn external_triggers_keep_the_retry_config_of_the_profile() {
        let request = recovery_request(serde_json::json!({ "connector": "stripe" }));
        let config = RevenueRecoveryConfig {
            max_retry_count: 7,
            ..retry_config()
        };

        let trigger =
            RecoveryTrigger::from_profile_settings(ExternalTriggers, Some(&config), &request)
                .expect("the config is on the profile");

        let RecoveryTrigger::External { retry_config, .. } = trigger else {
            panic!("external triggers keep the config of the profile");
        };
        assert_eq!(retry_config.max_retry_count, 7);
    }

    #[test]
    fn a_new_invoice_follows_the_profile() {
        for profile_trigger_model in [InternalTriggers, ExternalTriggers] {
            assert!(validate_invoice_trigger_model(None, profile_trigger_model).is_ok());
        }
    }

    #[test]
    fn an_invoice_stays_recordable_under_its_own_trigger_model() {
        for trigger_model in [InternalTriggers, ExternalTriggers] {
            assert!(validate_invoice_trigger_model(Some(trigger_model), trigger_model).is_ok());
        }
    }

    #[test]
    fn an_invoice_is_refused_after_its_profile_changes_trigger_model() {
        for (invoice_trigger_model, profile_trigger_model) in [
            (InternalTriggers, ExternalTriggers),
            (ExternalTriggers, InternalTriggers),
        ] {
            let error =
                validate_invoice_trigger_model(Some(invoice_trigger_model), profile_trigger_model)
                    .expect_err("the invoice keeps the model it was first recorded under");

            assert!(matches!(
                error.current_context(),
                ApiErrorResponse::PreconditionFailed { .. }
            ));
        }
    }

    #[test]
    fn external_triggers_need_the_connector() {
        let request = recovery_request(serde_json::json!({}));

        let error = RecoveryTrigger::from_profile_settings(
            ExternalTriggers,
            Some(&retry_config()),
            &request,
        )
        .expect_err("the connector is required");

        assert!(is_missing_field(error.current_context(), "connector"));
    }

    #[test]
    fn external_triggers_do_not_accept_connector_accounts() {
        for account_field in [
            "billing_merchant_connector_id",
            "payment_merchant_connector_id",
        ] {
            let request = recovery_request(serde_json::json!({
                "connector": "stripe",
                account_field: "mca_1234567890",
            }));

            let error = RecoveryTrigger::from_profile_settings(
                ExternalTriggers,
                Some(&retry_config()),
                &request,
            )
            .expect_err("no account is used with external triggers");

            assert!(matches!(
                error.current_context(),
                ApiErrorResponse::InvalidRequestData { .. }
            ));
        }
    }

    #[test]
    fn external_triggers_keep_every_listed_connector() {
        for (sent, connector) in [
            ("adyen", RecoveryConnector::Adyen),
            ("stripe", RecoveryConnector::Stripe),
            ("paypal", RecoveryConnector::Paypal),
            ("other", RecoveryConnector::Other),
        ] {
            let request = recovery_request(serde_json::json!({ "connector": sent }));
            let config = retry_config();

            let trigger =
                RecoveryTrigger::from_profile_settings(ExternalTriggers, Some(&config), &request)
                    .expect("every listed connector is accepted");

            assert!(matches!(
                trigger,
                RecoveryTrigger::External { connector: kept, .. } if kept == connector
            ));
        }
    }
}
