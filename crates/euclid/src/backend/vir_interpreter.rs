pub mod types;

use crate::{
    backend::{self, inputs, EuclidBackend},
    frontend::{
        ast,
        dir::{self, EuclidDirFilter},
        vir,
    },
};

pub struct VirInterpreterBackend<O> {
    program: vir::ValuedProgram<O>,
}

impl<O> VirInterpreterBackend<O>
where
    O: Clone,
{
    #[inline]
        /// Evaluates a valued comparison based on the provided logic and context.
    fn eval_comparison(comp: &vir::ValuedComparison, ctx: &types::Context) -> bool {
        match &comp.logic {
            vir::ValuedComparisonLogic::PositiveDisjunction => {
                comp.values.iter().any(|v| ctx.check_presence(v))
            }
            vir::ValuedComparisonLogic::NegativeConjunction => {
                comp.values.iter().all(|v| !ctx.check_presence(v))
            }
        }
    }

    #[inline]
        /// Evaluates a condition using the given context.
    /// 
    /// This method takes a valued if condition and a context as input arguments and evaluates the condition by iterating through each comparison in the condition and checking if they all evaluate to true using the provided context. It returns true if all comparisons in the condition evaluate to true, otherwise it returns false.
    fn eval_condition(cond: &vir::ValuedIfCondition, ctx: &types::Context) -> bool {
        cond.iter().all(|comp| Self::eval_comparison(comp, ctx))
    }

        /// Evaluates a valued if statement with the given context and returns a boolean value.
    fn eval_statement(stmt: &vir::ValuedIfStatement, ctx: &types::Context) -> bool {
        Self::eval_condition(&stmt.condition, ctx)
            .then(|| {
                stmt.nested.as_ref().map_or(true, |nested_stmts| {
                    nested_stmts.iter().any(|s| Self::eval_statement(s, ctx))
                })
            })
            .unwrap_or(false)
    }

        /// Evaluates a valued rule using the provided context. Returns true if any of the statements in the rule evaluate to true, otherwise returns false.
    fn eval_rule(rule: &vir::ValuedRule<O>, ctx: &types::Context) -> bool {
        rule.statements
            .iter()
            .any(|stmt| Self::eval_statement(stmt, ctx))
    }

        /// Evaluates a program by iterating through its rules and finding the first rule that evaluates to true based on the given context. Returns a backend output containing the connector selection and the name of the rule that evaluated to true, or the default selection if no rule evaluates to true.
    fn eval_program(
        program: &vir::ValuedProgram<O>,
        ctx: &types::Context,
    ) -> backend::BackendOutput<O> {
        program
            .rules
            .iter()
            .find(|rule| Self::eval_rule(rule, ctx))
            .map_or_else(
                || backend::BackendOutput {
                    connector_selection: program.default_selection.clone(),
                    rule_name: None,
                },
                |rule| backend::BackendOutput {
                    connector_selection: rule.connector_selection.clone(),
                    rule_name: Some(rule.name.clone()),
                },
            )
    }
}

impl<O> EuclidBackend<O> for VirInterpreterBackend<O>
where
    O: Clone + EuclidDirFilter,
{
    type Error = types::VirInterpreterError;

        /// Transforms an abstract syntax tree program into a lower-level representation and constructs a new instance of the current struct with the lower-level program.
    fn with_program(program: ast::Program<O>) -> Result<Self, Self::Error> {
        let dir_program = ast::lowering::lower_program(program)
            .map_err(types::VirInterpreterError::LoweringError)?;

        let vir_program = dir::lowering::lower_program(dir_program)
            .map_err(types::VirInterpreterError::LoweringError)?;

        Ok(Self {
            program: vir_program,
        })
    }

        /// Executes the backend program with the given input and returns the result.
    fn execute(
        &self,
        input: inputs::BackendInput,
    ) -> Result<backend::BackendOutput<O>, Self::Error> {
        let ctx = types::Context::from_input(input);
        Ok(Self::eval_program(&self.program, &ctx))
    }
}
#[cfg(all(test, feature = "ast_parser"))]
mod test {
    #![allow(clippy::expect_used)]
    use rustc_hash::FxHashMap;

    use super::*;
    use crate::{enums, types::DummyOutput};

    #[test]
        /// Parses a program string, creates input data, and executes a backend, then checks if the result matches the expected rule name.
    fn test_execution() {
        let program_str = r#"
        default: [ "stripe",  "adyen"]

        rule_1: ["stripe"]
        {
            pay_later = klarna
        }

        rule_2: ["adyen"]
        {
            pay_later = affirm
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                card_bin: None,
                currency: enums::Currency::USD,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_2");
    }
    #[test]
        /// This method is used to test the payment type by parsing a program string and creating input data. It then executes the program using a VirInterpreterBackend and asserts that the result rule name is as expected.
    fn test_payment_type() {
        let program_str = r#"
        default: ["stripe", "adyen"]
        rule_1: ["stripe"]
        {
           payment_type = setup_mandate
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: Some("123456".to_string()),
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: Some(enums::PaymentType::SetupMandate),
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }

    #[test]
        /// This method tests the mandate type by parsing a program string, creating input data, executing the program using a backend interpreter, and asserting that the result matches the expected rule name.
    fn test_mandate_type() {
        let program_str = r#"
        default: ["stripe", "adyen"]
        rule_1: ["stripe"]
        {
           mandate_type = single_use
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: Some("123456".to_string()),
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: Some(enums::MandateType::SingleUse),
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }

    #[test]
        /// This method is used to test the mandate acceptance type by providing a program string and input data, then executing the backend with the program and input to check if the rule name matches the expected value.
    fn test_mandate_acceptance_type() {
        let program_str = r#"
        default: ["stripe","adyen"]
        rule_1: ["stripe"]
        {
           mandate_acceptance_type = online
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: Some("123456".to_string()),
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: Some(enums::MandateAcceptanceType::Online),
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }
    #[test]
        /// Parses a program string, creates a backend input, and executes the backend to test a specific card bin rule.
    fn test_card_bin() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           card_bin="123456"
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: Some("123456".to_string()),
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }
    #[test]
        /// This method tests the payment amount by parsing a program string, creating input data, executing the backend with the program and input, and asserting that the result's rule name matches the expected value.
    fn test_payment_amount() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           amount = 32
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: None,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }
    #[test]
        /// This method is used to test a payment method by providing a program string and input data. It parses the program string, creates a backend input with payment and method data, executes the backend with the program, and asserts that the result has the expected rule name.
    fn test_payment_method() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           payment_method = pay_later
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: None,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }
    #[test]
        /// Parses a program string and executes it using a backend, then asserts the result against an expected rule name.
    fn test_future_usage() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           setup_future_usage = off_session
        }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 32,
                currency: enums::Currency::USD,
                card_bin: None,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: Some(enums::SetupFutureUsage::OffSession),
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }

    #[test]
        /// Executes a test for metadata using a predefined program and input, then asserts the result.
    fn test_metadata_execution() {
        let program_str = r#"
        default: ["stripe"," adyen"]

        rule_1: ["stripe"]
        {
        "metadata_key" = "arbitrary meta"
        }
        "#;
        let mut meta_map = FxHashMap::default();
        meta_map.insert("metadata_key".to_string(), "arbitrary meta".to_string());
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp = inputs::BackendInput {
            metadata: Some(meta_map),
            payment: inputs::PaymentInput {
                amount: 32,
                card_bin: None,
                currency: enums::Currency::USD,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };

        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result = backend.execute(inp).expect("Execution");
        assert_eq!(result.rule_name.expect("Rule Name").as_str(), "rule_1");
    }

    #[test]
        /// This method tests the less than operator by creating a program with a rule that checks if the amount is greater than or equal to 123. It then creates input data with an amount greater than and equal to 123, and executes the program using a VirInterpreterBackend. It finally asserts that the rule is correctly applied for both inputs.
    fn test_less_than_operator() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           amount>=123
        }
        "#;
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp_greater = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 150,
                card_bin: None,
                currency: enums::Currency::USD,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };
        let mut inp_equal = inp_greater.clone();
        inp_equal.payment.amount = 123;
        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result_greater = backend.execute(inp_greater).expect("Execution");
        let result_equal = backend.execute(inp_equal).expect("Execution");
        assert_eq!(
            result_equal.rule_name.expect("Rule Name").as_str(),
            "rule_1"
        );
        assert_eq!(
            result_greater.rule_name.expect("Rule Name").as_str(),
            "rule_1"
        );
    }

    #[test]
        /// This method tests the greater than operator by defining a program with a default rule and a specific rule that checks if the payment amount is less than or equal to 123. It then creates input data with a payment amount of 120 and 123, and runs the program using a virtual interpreter backend. It asserts that the results of both inputs match the expected rule name "rule_1".
    fn test_greater_than_operator() {
        let program_str = r#"
        default: ["stripe", "adyen"]

        rule_1: ["stripe"]
        {
           amount<=123
        }
        "#;
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let inp_lower = inputs::BackendInput {
            metadata: None,
            payment: inputs::PaymentInput {
                amount: 120,
                card_bin: None,
                currency: enums::Currency::USD,
                authentication_type: Some(enums::AuthenticationType::NoThreeDs),
                capture_method: Some(enums::CaptureMethod::Automatic),
                business_country: Some(enums::Country::UnitedStatesOfAmerica),
                billing_country: Some(enums::Country::France),
                business_label: None,
                setup_future_usage: None,
            },
            payment_method: inputs::PaymentMethodInput {
                payment_method: Some(enums::PaymentMethod::PayLater),
                payment_method_type: Some(enums::PaymentMethodType::Affirm),
                card_network: None,
            },
            mandate: inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
        };
        let mut inp_equal = inp_lower.clone();
        inp_equal.payment.amount = 123;
        let backend = VirInterpreterBackend::<DummyOutput>::with_program(program).expect("Program");
        let result_equal = backend.execute(inp_equal).expect("Execution");
        let result_lower = backend.execute(inp_lower).expect("Execution");
        assert_eq!(
            result_equal.rule_name.expect("Rule Name").as_str(),
            "rule_1"
        );
        assert_eq!(
            result_lower.rule_name.expect("Rule Name").as_str(),
            "rule_1"
        );
    }
}
