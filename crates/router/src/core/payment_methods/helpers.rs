use crate::types::api::enums as api_enums;
use api_models::payment_methods as pm_models;

pub fn validate_payment_method_data_against_payment_method(
    payment_method: api_enums::PaymentMethod,
    payment_method_data: pm_models::PaymentMethodCreateData,
) -> bool {
    match payment_method {
        api_enums::PaymentMethod::Card => matches!(
            payment_method_data,
            pm_models::PaymentMethodCreateData::Card(_)
        ),
        _ => false,
    }
}
