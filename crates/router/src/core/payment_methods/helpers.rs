use crate::configs::settings;
use api_models::enums;
pub fn is_conector_update_possible(
    supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
    payment_method: Option<&enums::PaymentMethod>,
    payment_method_type: Option<&enums::PaymentMethodType>,
    connector: enums::Connector,
) -> bool {
    payment_method
        .map(|payment_method| {
            if payment_method == &enums::PaymentMethod::Card {
                supported_payment_methods_for_mandate
                    .0
                    .get(payment_method)
                    .map(|payment_method_type_hm| {
                        let pm_credit = payment_method_type_hm
                            .0
                            .get(&enums::PaymentMethodType::Credit)
                            .map(|conn| conn.connector_list.clone())
                            .unwrap_or_default();
                        let pm_debit = payment_method_type_hm
                            .0
                            .get(&enums::PaymentMethodType::Debit)
                            .map(|conn| conn.connector_list.clone())
                            .unwrap_or_default();
                        &pm_credit | &pm_debit
                    })
                    .map(|supported_connectors| supported_connectors.contains(&connector))
                    .unwrap_or(false)
            } else if let Some(payment_method_type) = payment_method_type {
                supported_payment_methods_for_mandate
                    .0
                    .get(payment_method)
                    .and_then(|payment_method_type_hm| {
                        payment_method_type_hm.0.get(payment_method_type)
                    })
                    .map(|supported_connectors| {
                        supported_connectors.connector_list.contains(&connector)
                    })
                    .unwrap_or(false)
            } else {
                false
            }
        })
        .unwrap_or(false)
}
