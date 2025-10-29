use std::collections::HashMap;

use common_utils::link_utils::EnabledPaymentMethod;

#[cfg(all(feature = "v1", feature = "olap"))]
use crate::types::transformers::ForeignInto;
#[cfg(feature = "olap")]
use crate::types::{api::payments, domain, storage};
use crate::{
    settings::PayoutRequiredFields,
    types::{api, transformers::ForeignFrom},
};

#[cfg(all(feature = "v2", feature = "olap"))]
impl
    ForeignFrom<(
        storage::Payouts,
        storage::PayoutAttempt,
        Option<domain::Customer>,
        Option<payments::Address>,
    )> for api::PayoutCreateResponse
{
    fn foreign_from(
        item: (
            storage::Payouts,
            storage::PayoutAttempt,
            Option<domain::Customer>,
            Option<payments::Address>,
        ),
    ) -> Self {
        todo!()
    }
}

#[cfg(all(feature = "v1", feature = "olap"))]
impl
    ForeignFrom<(
        storage::Payouts,
        storage::PayoutAttempt,
        Option<domain::Customer>,
        Option<payments::Address>,
    )> for api::PayoutCreateResponse
{
    fn foreign_from(
        item: (
            storage::Payouts,
            storage::PayoutAttempt,
            Option<domain::Customer>,
            Option<payments::Address>,
        ),
    ) -> Self {
        let (payout, payout_attempt, customer, address) = item;
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
            merchant_order_reference_id: payout_attempt.merchant_order_reference_id.clone(),
            amount: payout.amount,
            currency: payout.destination_currency,
            connector: payout_attempt.connector,
            payout_type: payout.payout_type,
            auto_fulfill: payout.auto_fulfill,
            customer_id: customer.as_ref().map(|cust| cust.customer_id.clone()),
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
            billing: address,
            payout_method_data: payout_attempt.additional_payout_method_data.map(From::from),
            client_secret: None,
            payout_link: None,
            unified_code: attempt.unified_code.clone(),
            unified_message: attempt.unified_message.clone(),
            attempts: Some(vec![attempt]),
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
            payout_method_id: payout.payout_method_id,
        }
    }
}

#[cfg(feature = "v1")]
impl
    ForeignFrom<(
        &PayoutRequiredFields,
        Vec<EnabledPaymentMethod>,
        api::RequiredFieldsOverrideRequest,
    )> for Vec<api::PayoutEnabledPaymentMethodsInfo>
{
    fn foreign_from(
        (payout_required_fields, enabled_payout_methods, value_overrides): (
            &PayoutRequiredFields,
            Vec<EnabledPaymentMethod>,
            api::RequiredFieldsOverrideRequest,
        ),
    ) -> Self {
        let value_overrides = value_overrides.flat_struct();

        enabled_payout_methods
            .into_iter()
            .map(|enabled_payout_method| {
                let payment_method = enabled_payout_method.payment_method;
                let payment_method_types_info = enabled_payout_method
                    .payment_method_types
                    .into_iter()
                    .filter_map(|pmt| {
                        payout_required_fields
                            .0
                            .get(&payment_method)
                            .and_then(|pmt_info| {
                                pmt_info.0.get(&pmt).map(|connector_fields| {
                                    let mut required_fields = HashMap::new();

                                    for required_field_final in connector_fields.fields.values() {
                                        required_fields.extend(required_field_final.common.clone());
                                    }

                                    for (key, value) in &value_overrides {
                                        required_fields.entry(key.to_string()).and_modify(
                                            |required_field| {
                                                required_field.value =
                                                    Some(masking::Secret::new(value.to_string()));
                                            },
                                        );
                                    }
                                    api::PaymentMethodTypeInfo {
                                        payment_method_type: pmt,
                                        required_fields: if required_fields.is_empty() {
                                            None
                                        } else {
                                            Some(required_fields)
                                        },
                                    }
                                })
                            })
                            .or(Some(api::PaymentMethodTypeInfo {
                                payment_method_type: pmt,
                                required_fields: None,
                            }))
                    })
                    .collect();

                api::PayoutEnabledPaymentMethodsInfo {
                    payment_method,
                    payment_method_types_info,
                }
            })
            .collect()
    }
}
