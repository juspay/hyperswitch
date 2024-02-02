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
        /// Evaluates the given number against an array of NumberComparison objects and returns a Result indicating whether any of the comparisons are true or false.
    /// 
    /// # Arguments
    /// * `num` - The number to be evaluated
    /// * `array` - An array of NumberComparison objects containing the comparison type and the number to compare against
    /// 
    /// # Returns
    /// * `Ok(true)` - If any of the comparisons are true
    /// * `Ok(false)` - If none of the comparisons are true
    /// * `Err` - If there is an error evaluating the comparisons
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

        /// Evaluates a comparison expression and returns a boolean result.
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

        /// Evaluates a list of comparisons within an if condition and returns a boolean result.
    ///
    /// # Arguments
    ///
    /// * `condition` - A reference to the if condition AST node containing the comparisons to evaluate.
    /// * `ctx` - A reference to the context in which the comparisons should be evaluated.
    ///
    /// # Returns
    ///
    /// A `Result` containing a boolean value indicating the result of evaluating the comparisons. If all comparisons evaluate to true, the result is Ok(true); otherwise, it is Ok(false).
    ///
    /// # Errors
    ///
    /// Returns an `InterpreterError` if there is an error while evaluating the comparisons.
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

        /// Evaluates an if statement and its nested if statements, returning true if any of them
    /// evaluate to true, or false if none of them do.
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

        /// Evaluates a list of if statements and returns true if at least one of them evaluates to true, otherwise returns false.
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
        /// Evaluates the given rule using the provided context and returns a Result indicating
    /// whether the rule is true or false. If the evaluation encounters an error, it returns
    /// an InterpreterError.
    ///
    /// # Arguments
    ///
    /// * `rule` - a reference to the rule to be evaluated
    /// * `ctx` - a reference to the context in which the rule should be evaluated
    ///
    /// # Returns
    ///
    /// A Result containing a boolean indicating whether the rule is true or false, or an
    /// InterpreterError if the evaluation encounters an error.
    fn eval_rule(
        rule: &ast::Rule<O>,
        ctx: &types::Context,
    ) -> Result<bool, types::InterpreterError> {
        Self::eval_rule_statements(&rule.statements, ctx)
    }

        /// Evaluates the given program by iterating through its rules and evaluating each rule using the provided context.
    /// If a rule evaluates to true, returns a BackendOutput containing the connector selection and rule name.
    /// If no rule evaluates to true, returns a BackendOutput containing the program's default selection and no rule name.
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

        /// Constructs a new instance of the current struct by associating it with the given program.
    ///
    /// # Arguments
    ///
    /// * `program` - The program to associate with the current instance
    ///
    /// # Returns
    ///
    /// A Result containing the new instance of the current struct if the program association was successful, or an error if the association failed.
    fn with_program(program: ast::Program<O>) -> Result<Self, Self::Error> {
        Ok(Self { program })
    }

        /// Executes a backend input to produce a backend output, or returns an error if the execution fails.
    fn execute(&self, input: inputs::BackendInput) -> Result<super::BackendOutput<O>, Self::Error> {
        let ctx: types::Context = input.into();
        Self::eval_program(&self.program, &ctx)
    }
}
