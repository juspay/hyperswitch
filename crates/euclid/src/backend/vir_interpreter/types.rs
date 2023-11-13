use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    backend::inputs::BackendInput,
    dssa,
    types::{self, EuclidKey, EuclidValue, MetadataValue, NumValueRefinement, StrValue},
};

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
pub enum VirInterpreterError {
    #[error("Error when lowering the program: {0:?}")]
    LoweringError(dssa::types::AnalysisError),
}

pub struct Context {
    atomic_values: FxHashSet<EuclidValue>,
    numeric_values: FxHashMap<EuclidKey, EuclidValue>,
}

impl Context {
    pub fn check_presence(&self, value: &EuclidValue) -> bool {
        let key = value.get_key();

        match key.key_type() {
            types::DataType::MetadataValue => self.atomic_values.contains(value),
            types::DataType::StrValue => self.atomic_values.contains(value),
            types::DataType::EnumVariant => self.atomic_values.contains(value),
            types::DataType::Number => {
                let ctx_num_value = self
                    .numeric_values
                    .get(&key)
                    .and_then(|value| value.get_num_value());

                value.get_num_value().zip(ctx_num_value).map_or(
                    false,
                    |(program_value, ctx_value)| {
                        let program_num = program_value.number;
                        let ctx_num = ctx_value.number;

                        match &program_value.refinement {
                            None => program_num == ctx_num,
                            Some(NumValueRefinement::NotEqual) => ctx_num != program_num,
                            Some(NumValueRefinement::GreaterThan) => ctx_num > program_num,
                            Some(NumValueRefinement::GreaterThanEqual) => ctx_num >= program_num,
                            Some(NumValueRefinement::LessThanEqual) => ctx_num <= program_num,
                            Some(NumValueRefinement::LessThan) => ctx_num < program_num,
                        }
                    },
                )
            }
        }
    }

    pub fn from_input(input: BackendInput) -> Self {
        let payment = input.payment;
        let payment_method = input.payment_method;
        let meta_data = input.metadata;
        let payment_mandate = input.mandate;

        let mut enum_values: FxHashSet<EuclidValue> =
            FxHashSet::from_iter([EuclidValue::PaymentCurrency(payment.currency)]);

        if let Some(pm) = payment_method.payment_method {
            enum_values.insert(EuclidValue::PaymentMethod(pm));
        }

        if let Some(pmt) = payment_method.payment_method_type {
            enum_values.insert(EuclidValue::PaymentMethodType(pmt));
        }

        if let Some(met) = meta_data {
            for (key, value) in met.into_iter() {
                enum_values.insert(EuclidValue::Metadata(MetadataValue { key, value }));
            }
        }

        if let Some(at) = payment.authentication_type {
            enum_values.insert(EuclidValue::AuthenticationType(at));
        }

        if let Some(capture_method) = payment.capture_method {
            enum_values.insert(EuclidValue::CaptureMethod(capture_method));
        }

        if let Some(country) = payment.business_country {
            enum_values.insert(EuclidValue::BusinessCountry(country));
        }

        if let Some(country) = payment.billing_country {
            enum_values.insert(EuclidValue::BillingCountry(country));
        }
        if let Some(card_bin) = payment.card_bin {
            enum_values.insert(EuclidValue::CardBin(StrValue { value: card_bin }));
        }
        if let Some(business_label) = payment.business_label {
            enum_values.insert(EuclidValue::BusinessLabel(StrValue {
                value: business_label,
            }));
        }
        if let Some(setup_future_usage) = payment.setup_future_usage {
            enum_values.insert(EuclidValue::SetupFutureUsage(setup_future_usage));
        }
        if let Some(payment_type) = payment_mandate.payment_type {
            enum_values.insert(EuclidValue::PaymentType(payment_type));
        }
        if let Some(mandate_type) = payment_mandate.mandate_type {
            enum_values.insert(EuclidValue::MandateType(mandate_type));
        }
        if let Some(mandate_acceptance_type) = payment_mandate.mandate_acceptance_type {
            enum_values.insert(EuclidValue::MandateAcceptanceType(mandate_acceptance_type));
        }

        let numeric_values: FxHashMap<EuclidKey, EuclidValue> = FxHashMap::from_iter([(
            EuclidKey::PaymentAmount,
            EuclidValue::PaymentAmount(types::NumValue {
                number: payment.amount,
                refinement: None,
            }),
        )]);

        Self {
            atomic_values: enum_values,
            numeric_values,
        }
    }
}
