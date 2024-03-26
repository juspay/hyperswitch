use std::{collections::HashMap, fmt, ops::Deref, string::ToString};

use serde::Serialize;

use crate::{backend::inputs, frontend::ast::ValueType, types::EuclidKey};

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum InterpreterErrorType {
    #[error("Invalid key received '{0}'")]
    InvalidKey(String),
    #[error("Invalid Comparison")]
    InvalidComparison,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
pub struct InterpreterError {
    pub error_type: InterpreterErrorType,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        InterpreterErrorType::fmt(&self.error_type, f)
    }
}

pub struct Context(HashMap<String, Option<ValueType>>);

impl Deref for Context {
    type Target = HashMap<String, Option<ValueType>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<inputs::BackendInput> for Context {
    fn from(input: inputs::BackendInput) -> Self {
        let ctx = HashMap::<String, Option<ValueType>>::from_iter([
            (
                EuclidKey::PaymentMethod.to_string(),
                input
                    .payment_method
                    .payment_method
                    .map(|pm| ValueType::EnumVariant(pm.to_string())),
            ),
            (
                EuclidKey::PaymentMethodType.to_string(),
                input
                    .payment_method
                    .payment_method_type
                    .map(|pt| ValueType::EnumVariant(pt.to_string())),
            ),
            (
                EuclidKey::AuthenticationType.to_string(),
                input
                    .payment
                    .authentication_type
                    .map(|at| ValueType::EnumVariant(at.to_string())),
            ),
            (
                EuclidKey::CaptureMethod.to_string(),
                input
                    .payment
                    .capture_method
                    .map(|cm| ValueType::EnumVariant(cm.to_string())),
            ),
            (
                EuclidKey::PaymentAmount.to_string(),
                Some(ValueType::Number(input.payment.amount)),
            ),
            (
                EuclidKey::PaymentCurrency.to_string(),
                Some(ValueType::EnumVariant(input.payment.currency.to_string())),
            ),
        ]);

        Self(ctx)
    }
}
