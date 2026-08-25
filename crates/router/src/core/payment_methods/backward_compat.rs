use common_utils::{ext_traits::StringExt, id_type};
use error_stack::ResultExt;
use hyperswitch_domain_models::platform::ProviderMerchantId;

use super::{add_payment_method_modular_backward_compat_task, utils};
use crate::{
    core::{configs::dimension_state, errors::ProcessTrackerError},
    errors as router_errors, logger,
    routes::SessionState,
    types::{domain, storage},
};

async fn trigger_payment_method_modular_backward_compat_inline(
    state: &SessionState,
    payment_method: &domain::PaymentMethod,
    merchant_id: &id_type::MerchantId,
    organization_id: &id_type::OrganizationId,
    last_modified_by: Option<String>,
) -> Result<(), ProcessTrackerError> {
    let tracking_data = storage::PaymentMethodModularCompatTrackingData {
        payment_method_id: payment_method.get_id().get_string_repr().to_owned(),
        merchant_id: merchant_id.to_owned(),
        organization_id: organization_id.clone(),
        last_modified_by,
    };

    Box::pin(
        crate::workflows::payment_method_modular_backward_compat::run_payment_method_modular_backward_compat_backfill(
            state,
            tracking_data,
            "INLINE_PM_MOD_BACK_COMPAT",
        ),
    )
    .await
}

async fn schedule_payment_method_modular_backward_compat_task_best_effort(
    state: &SessionState,
    payment_method: &domain::PaymentMethod,
    merchant_id: &id_type::MerchantId,
    organization_id: &id_type::OrganizationId,
    last_modified_by: Option<String>,
) {
    let res = add_payment_method_modular_backward_compat_task(
        &*state.store,
        payment_method,
        merchant_id,
        organization_id.clone(),
        state.conf.application_source,
        last_modified_by,
    )
    .await
    .change_context(router_errors::ApiErrorResponse::InternalServerError)
    .attach_printable(
        "Failed to add payment method modular backward compatibility task in process tracker",
    );

    if let Err(err) = res {
        logger::error!(
            ?err,
            payment_method_id = %payment_method.get_id().get_string_repr(),
            merchant_id=%merchant_id.get_string_repr(),
            "Failed to schedule modular backward compatibility PT; continuing payment method create flow"
        );
    } else {
        logger::info!(
            payment_method_id = %payment_method.get_id().get_string_repr(),
            merchant_id=%merchant_id.get_string_repr(),
            "Scheduled payment method modular backward compatibility PT"
        );
    }
}

pub(super) async fn trigger_payment_method_modular_backward_compat(
    state: &SessionState,
    payment_method: &domain::PaymentMethod,
    organization_id: &id_type::OrganizationId,
    last_modified_by: Option<String>,
) {
    let merchant_id = &payment_method.merchant_id;
    let dimensions = dimension_state::Dimensions::new()
        .with_provider_merchant_id(ProviderMerchantId::new(merchant_id.clone()))
        .with_organization_id(organization_id.clone());
    let should_trigger_backwards_compatibility_inline =
        utils::get_should_trigger_backwards_compatibility_inline(state, &dimensions, None).await;

    if should_trigger_backwards_compatibility_inline {
        let inline_result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            Box::pin(trigger_payment_method_modular_backward_compat_inline(
                state,
                payment_method,
                merchant_id,
                organization_id,
                last_modified_by.clone(),
            )),
        )
        .await;

        match inline_result {
            Ok(Ok(())) => {
                logger::info!(
                    payment_method_id = %payment_method.get_id().get_string_repr(),
                    merchant_id=%merchant_id.get_string_repr(),
                    "Completed modular backward compatibility inline"
                );
            }
            Ok(Err(err)) => {
                logger::error!(
                    ?err,
                    payment_method_id = %payment_method.get_id().get_string_repr(),
                    merchant_id=%merchant_id.get_string_repr(),
                    "Failed modular backward compatibility inline; continuing payment method create flow"
                );
            }
            Err(err) => {
                logger::error!(
                    ?err,
                    payment_method_id = %payment_method.get_id().get_string_repr(),
                    merchant_id=%merchant_id.get_string_repr(),
                    "Timed out modular backward compatibility inline; scheduling PT fallback"
                );
                schedule_payment_method_modular_backward_compat_task_best_effort(
                    state,
                    payment_method,
                    merchant_id,
                    organization_id,
                    last_modified_by,
                )
                .await;
            }
        }
    } else {
        let should_schedule_modular_backward_compat =
            utils::get_should_schedule_modular_backward_compat(state, &dimensions, None).await;

        if should_schedule_modular_backward_compat {
            schedule_payment_method_modular_backward_compat_task_best_effort(
                state,
                payment_method,
                merchant_id,
                organization_id,
                last_modified_by,
            )
            .await;
        } else {
            logger::debug!(
                payment_method_id = %payment_method.get_id().get_string_repr(),
                merchant_id=%merchant_id.get_string_repr(),
                "Skipping modular backward compatibility PT scheduling by config"
            );
        }
    }
}
