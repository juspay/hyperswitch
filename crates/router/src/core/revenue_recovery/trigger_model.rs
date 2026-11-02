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

/// Whether a recovery job stops instead of running. The profile alone decides who retries, and
/// under external triggers that is the merchant, so a job left waiting from before the profile
/// changed must not charge. A status check still runs, as its charge was already made.
pub(crate) fn should_stop_retry_job(
    trigger_model: common_enums::RevenueRecoveryTriggerModel,
    job_name: Option<&str>,
) -> bool {
    trigger_model == common_enums::RevenueRecoveryTriggerModel::ExternalTriggers
        && matches!(
            job_name,
            Some(super::CALCULATE_WORKFLOW | super::EXECUTE_WORKFLOW)
        )
}
