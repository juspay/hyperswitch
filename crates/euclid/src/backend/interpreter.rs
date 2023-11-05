pub mod types;

use crate::{
    backend::{self, inputs, EuclidBackend},
    frontend::ast,
};

pub struct InterpreterBackend<O> {
    program: ast::Program<O>,
}

impl<O> InterpreterBackend<O>
where
    O: Clone,
{
    fn eval_number_comparison_array(
        num: i64,
        array: &[ast::NumberComparison],
    ) -> Result<bool, types::InterpreterError> {
        for comparison in array {
            let other = comparison.number;
            let res = match comparison.comparison_type {
                ast::ComparisonType::GreaterThan => num > other,
                ast::ComparisonType::LessThan => num < other,
                ast::ComparisonType::LessThanEqual => num <= other,
                ast::ComparisonType::GreaterThanEqual => num >= other,
                ast::ComparisonType::Equal => num == other,
                ast::ComparisonType::NotEqual => num != other,
            };

            if res {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn eval_comparison(
        comparison: &ast::Comparison,
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        use ast::{ComparisonType::*, ValueType::*};

        let value = ctx
            .get(&comparison.lhs)
            .ok_or_else(|| types::InterpreterError {
                error_type: types::InterpreterErrorType::InvalidKey(comparison.lhs.clone()),
                metadata: comparison.metadata.clone(),
            })?;

        if let Some(val) = value {
            match (val, &comparison.comparison, &comparison.value) {
                (EnumVariant(e1), Equal, EnumVariant(e2)) => Ok(e1 == e2),
                (EnumVariant(e1), NotEqual, EnumVariant(e2)) => Ok(e1 != e2),
                (EnumVariant(e), Equal, EnumVariantArray(evec)) => Ok(evec.iter().any(|v| e == v)),
                (EnumVariant(e), NotEqual, EnumVariantArray(evec)) => {
                    Ok(evec.iter().all(|v| e != v))
                }
                (Number(n1), Equal, Number(n2)) => Ok(n1 == n2),
                (Number(n1), NotEqual, Number(n2)) => Ok(n1 != n2),
                (Number(n1), LessThanEqual, Number(n2)) => Ok(n1 <= n2),
                (Number(n1), GreaterThanEqual, Number(n2)) => Ok(n1 >= n2),
                (Number(n1), LessThan, Number(n2)) => Ok(n1 < n2),
                (Number(n1), GreaterThan, Number(n2)) => Ok(n1 > n2),
                (Number(n), Equal, NumberArray(nvec)) => Ok(nvec.iter().any(|v| v == n)),
                (Number(n), NotEqual, NumberArray(nvec)) => Ok(nvec.iter().all(|v| v != n)),
                (Number(n), Equal, NumberComparisonArray(ncvec)) => {
                    Self::eval_number_comparison_array(*n, ncvec)
                }
                _ => Err(types::InterpreterError {
                    error_type: types::InterpreterErrorType::InvalidComparison,
                    metadata: comparison.metadata.clone(),
                }),
            }
        } else {
            Ok(false)
        }
    }

    fn eval_if_condition(
        condition: &ast::IfCondition,
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        for comparison in condition {
            let res = Self::eval_comparison(comparison, ctx)?;

            if !res {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn eval_if_statement(
        stmt: &ast::IfStatement,
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        let cond_res = Self::eval_if_condition(&stmt.condition, ctx)?;

        if !cond_res {
            return Ok(false);
        }

        if let Some(ref nested) = stmt.nested {
            for nested_if in nested {
                let res = Self::eval_if_statement(nested_if, ctx)?;

                if res {
                    return Ok(true);
                }
            }

            return Ok(false);
        }

        Ok(true)
    }

    fn eval_rule_statements(
        statements: &[ast::IfStatement],
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        for stmt in statements {
            let res = Self::eval_if_statement(stmt, ctx)?;

            if res {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[inline]
    fn eval_rule(
        rule: &ast::Rule<O>,
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        Self::eval_rule_statements(&rule.statements, ctx)
    }

    fn eval_program(
        program: &ast::Program<O>,
        ctx: &types::Context,
    ) -> Result<backend::BackendOutput<O>, types::InterpreterError> {
        for rule in &program.rules {
            let res = Self::eval_rule(rule, ctx)?;

            if res {
                return Ok(backend::BackendOutput {
                    connector_selection: rule.connector_selection.clone(),
                    rule_name: Some(rule.name.clone()),
                });
            }
        }

        Ok(backend::BackendOutput {
            connector_selection: program.default_selection.clone(),
            rule_name: None,
        })
    }
}

impl<O> EuclidBackend<O> for InterpreterBackend<O>
where
    O: Clone,
{
    type Error = types::InterpreterError;

    fn with_program(program: ast::Program<O>) -> Result<Self, Self::Error> {
        Ok(Self { program })
    }

    fn execute(&self, input: inputs::BackendInput) -> Result<super::BackendOutput<O>, Self::Error> {
        let ctx: types::Context = input.into();
        Self::eval_program(&self.program, &ctx)
    }
}
