//! Static Analysis for the Euclid Rule DSL
//!
//! Exposes certain functions that can be used to perform static analysis over programs
//! in the Euclid Rule DSL. These include standard control flow analyses like testing
//! conflicting assertions, to Domain Specific Analyses making use of the
//! [`Knowledge Graph Framework`](crate::dssa::graph).
use rustc_hash::{FxHashMap, FxHashSet};

use super::{graph::Memoization, types::EuclidAnalysable};
use crate::{
    dssa::{graph, state_machine, truth, types},
    frontend::{
        ast,
        dir::{self, EuclidDirFilter},
        vir,
    },
    types::{DataType, Metadata},
};

/// Analyses conflicting assertions on the same key in a conjunctive context.
///
/// For example,
/// ```notrust
/// payment_method = card && ... && payment_method = bank_debit
/// ```notrust
/// This is a condition that will never evaluate to `true` given a single
/// payment method and needs to be caught in analysis.
pub fn analyze_conflicting_assertions(
    keywise_assertions: &FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>>,
    assertion_metadata: &FxHashMap<&dir::DirValue, &Metadata>,
) -> Result<(), types::AnalysisError> {
    for (key, value_set) in keywise_assertions {
        if value_set.len() > 1 {
            let err_type = types::AnalysisErrorType::ConflictingAssertions {
                key: key.clone(),
                values: value_set
                    .iter()
                    .map(|val| types::ValueData {
                        value: (*val).clone(),
                        metadata: assertion_metadata
                            .get(val)
                            .map(|meta| (*meta).clone())
                            .unwrap_or_default(),
                    })
                    .collect(),
            };

            Err(types::AnalysisError {
                error_type: err_type,
                metadata: Default::default(),
            })?;
        }
    }
    Ok(())
}

/// Analyses exhaustive negations on the same key in a conjunctive context.
///
/// For example,
/// ```notrust
/// authentication_type /= three_ds && ... && authentication_type /= no_three_ds
/// ```notrust
/// This is a condition that will never evaluate to `true` given any authentication_type
/// since all the possible values authentication_type can take have been negated.
pub fn analyze_exhaustive_negations(
    keywise_negations: &FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>>,
    keywise_negation_metadata: &FxHashMap<dir::DirKey, Vec<&Metadata>>,
) -> Result<(), types::AnalysisError> {
    for (key, negation_set) in keywise_negations {
        let mut value_set = if let Some(set) = key.kind.get_value_set() {
            set
        } else {
            continue;
        };

        value_set.retain(|val| !negation_set.contains(val));

        if value_set.is_empty() {
            let error_type = types::AnalysisErrorType::ExhaustiveNegation {
                key: key.clone(),
                metadata: keywise_negation_metadata
                    .get(key)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .cloned()
                    .cloned()
                    .collect(),
            };

            Err(types::AnalysisError {
                error_type,
                metadata: Default::default(),
            })?;
        }
    }
    Ok(())
}

fn analyze_negated_assertions(
    keywise_assertions: &FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>>,
    assertion_metadata: &FxHashMap<&dir::DirValue, &Metadata>,
    keywise_negations: &FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>>,
    negation_metadata: &FxHashMap<&dir::DirValue, &Metadata>,
) -> Result<(), types::AnalysisError> {
    for (key, negation_set) in keywise_negations {
        let assertion_set = if let Some(set) = keywise_assertions.get(key) {
            set
        } else {
            continue;
        };

        let intersection = negation_set & assertion_set;

        intersection.iter().next().map_or(Ok(()), |val| {
            let error_type = types::AnalysisErrorType::NegatedAssertion {
                value: (*val).clone(),
                assertion_metadata: assertion_metadata
                    .get(*val)
                    .cloned()
                    .cloned()
                    .unwrap_or_default(),
                negation_metadata: negation_metadata
                    .get(*val)
                    .cloned()
                    .cloned()
                    .unwrap_or_default(),
            };

            Err(types::AnalysisError {
                error_type,
                metadata: Default::default(),
            })
        })?;
    }
    Ok(())
}

fn perform_condition_analyses(
    context: &types::ConjunctiveContext<'_>,
) -> Result<(), types::AnalysisError> {
    let mut assertion_metadata: FxHashMap<&dir::DirValue, &Metadata> = FxHashMap::default();
    let mut keywise_assertions: FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>> =
        FxHashMap::default();
    let mut negation_metadata: FxHashMap<&dir::DirValue, &Metadata> = FxHashMap::default();
    let mut keywise_negation_metadata: FxHashMap<dir::DirKey, Vec<&Metadata>> =
        FxHashMap::default();
    let mut keywise_negations: FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>> =
        FxHashMap::default();

    for ctx_val in context {
        let key = if let Some(k) = ctx_val.value.get_key() {
            k
        } else {
            continue;
        };

        if let dir::DirKeyKind::Connector = key.kind {
            continue;
        }

        if !matches!(key.kind.get_type(), DataType::EnumVariant) {
            continue;
        }

        match ctx_val.value {
            types::CtxValueKind::Assertion(val) => {
                keywise_assertions
                    .entry(key.clone())
                    .or_default()
                    .insert(val);

                assertion_metadata.insert(val, ctx_val.metadata);
            }

            types::CtxValueKind::Negation(vals) => {
                let negation_set = keywise_negations.entry(key.clone()).or_default();

                for val in vals {
                    negation_set.insert(val);
                    negation_metadata.insert(val, ctx_val.metadata);
                }

                keywise_negation_metadata
                    .entry(key.clone())
                    .or_default()
                    .push(ctx_val.metadata);
            }
        }
    }

    analyze_conflicting_assertions(&keywise_assertions, &assertion_metadata)?;
    analyze_exhaustive_negations(&keywise_negations, &keywise_negation_metadata)?;
    analyze_negated_assertions(
        &keywise_assertions,
        &assertion_metadata,
        &keywise_negations,
        &negation_metadata,
    )?;

    Ok(())
}

fn perform_context_analyses(
    context: &types::ConjunctiveContext<'_>,
    knowledge_graph: &graph::KnowledgeGraph<'_>,
) -> Result<(), types::AnalysisError> {
    perform_condition_analyses(context)?;
    let mut memo = Memoization::new();
    knowledge_graph
        .perform_context_analysis(context, &mut memo)
        .map_err(|err| types::AnalysisError {
            error_type: types::AnalysisErrorType::GraphAnalysis(err, memo),
            metadata: Default::default(),
        })?;
    Ok(())
}

pub fn analyze<O: EuclidAnalysable + EuclidDirFilter>(
    program: ast::Program<O>,
    knowledge_graph: Option<&graph::KnowledgeGraph<'_>>,
) -> Result<vir::ValuedProgram<O>, types::AnalysisError> {
    let dir_program = ast::lowering::lower_program(program)?;

    let selection_data = state_machine::make_connector_selection_data(&dir_program);
    let mut ctx_manager = state_machine::AnalysisContextManager::new(&dir_program, &selection_data);
    while let Some(ctx) = ctx_manager.advance().map_err(|err| types::AnalysisError {
        metadata: Default::default(),
        error_type: types::AnalysisErrorType::StateMachine(err),
    })? {
        perform_context_analyses(ctx, knowledge_graph.unwrap_or(&truth::ANALYSIS_GRAPH))?;
    }

    dir::lowering::lower_program(dir_program)
}

#[cfg(all(test, feature = "ast_parser"))]
mod tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use std::{ops::Deref, sync::Weak};

    use euclid_macros::knowledge;

    use super::*;
    use crate::{dirval, types::DummyOutput};

    #[test]
    fn test_conflicting_assertion_detection() {
        let program_str = r#"
            default: ["stripe", "adyen"]

            stripe_first: ["stripe", "adyen"]
            {
                payment_method = wallet {
                    amount > 500 & capture_method = automatic
                    amount < 500 & payment_method = card
                }
            }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let analysis_result = analyze(program, None);

        if let Err(types::AnalysisError {
            error_type: types::AnalysisErrorType::ConflictingAssertions { key, values },
            ..
        }) = analysis_result
        {
            assert!(
                matches!(key.kind, dir::DirKeyKind::PaymentMethod),
                "Key should be payment_method"
            );
            let values: Vec<dir::DirValue> = values.into_iter().map(|v| v.value).collect();
            assert_eq!(values.len(), 2, "There should be 2 conflicting conditions");
            assert!(
                values.contains(&dirval!(PaymentMethod = Wallet)),
                "Condition should include payment_method = wallet"
            );
            assert!(
                values.contains(&dirval!(PaymentMethod = Card)),
                "Condition should include payment_method = card"
            );
        } else {
            panic!("Did not receive conflicting assertions error");
        }
    }

    #[test]
    fn test_exhaustive_negation_detection() {
        let program_str = r#"
            default: ["stripe"]

            rule_1: ["adyen"]
            {
                payment_method /= wallet {
                    capture_method = manual & payment_method /= card {
                        authentication_type = three_ds & payment_method /= pay_later {
                            amount > 1000 & payment_method /= bank_redirect {
                                payment_method /= crypto
                                    & payment_method /= bank_debit
                                    & payment_method /= bank_transfer
                                    & payment_method /= upi
                                    & payment_method /= reward
                                    & payment_method /= voucher
                                    & payment_method /= gift_card

                            }
                        }
                    }
                }
            }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let analysis_result = analyze(program, None);

        if let Err(types::AnalysisError {
            error_type: types::AnalysisErrorType::ExhaustiveNegation { key, .. },
            ..
        }) = analysis_result
        {
            assert!(
                matches!(key.kind, dir::DirKeyKind::PaymentMethod),
                "Expected key to be payment_method"
            );
        } else {
            panic!("Expected exhaustive negation error");
        }
    }

    #[test]
    fn test_negated_assertions_detection() {
        let program_str = r#"
            default: ["stripe"]

            rule_1: ["adyen"]
            {
                payment_method = wallet {
                    amount > 500 {
                        capture_method = automatic
                    }

                    amount < 501 {
                        payment_method /= wallet
                    }
                }
            }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let analysis_result = analyze(program, None);

        if let Err(types::AnalysisError {
            error_type: types::AnalysisErrorType::NegatedAssertion { value, .. },
            ..
        }) = analysis_result
        {
            assert_eq!(
                value,
                dirval!(PaymentMethod = Wallet),
                "Expected to catch payment_method = wallet as conflict"
            );
        } else {
            panic!("Expected negated assertion error");
        }
    }

    #[test]
    fn test_negation_graph_analysis() {
        let graph = knowledge! {crate
            CaptureMethod(Automatic) ->> PaymentMethod(Card);
        };

        let program_str = r#"
            default: ["stripe"]

            rule_1: ["adyen"]
            {
                amount > 500 {
                    payment_method = pay_later
                }

                amount < 500 {
                    payment_method /= wallet & payment_method /= pay_later
                }
            }
        "#;

        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Graph");
        let analysis_result = analyze(program, Some(&graph));

        let error_type = match analysis_result {
            Err(types::AnalysisError { error_type, .. }) => error_type,
            _ => panic!("Error_type not found"),
        };

        let a_err = match error_type {
            types::AnalysisErrorType::GraphAnalysis(trace, memo) => (trace, memo),
            _ => panic!("Graph Analysis not found"),
        };

        let (trace, metadata) = match a_err.0 {
            graph::AnalysisError::NegationTrace { trace, metadata } => (trace, metadata),
            _ => panic!("Negation Trace not found"),
        };

        let predecessor = match Weak::upgrade(&trace)
            .expect("Expected Arc not found")
            .deref()
            .clone()
        {
            graph::AnalysisTrace::Value { predecessors, .. } => {
                let _value = graph::NodeValue::Value(dir::DirValue::PaymentMethod(
                    dir::enums::PaymentMethod::Card,
                ));
                let _relation = graph::Relation::Positive;
                predecessors
            }
            _ => panic!("Expected Negation Trace for payment method = card"),
        };

        let pred = match predecessor {
            Some(graph::ValueTracePredecessor::Mandatory(predecessor)) => predecessor,
            _ => panic!("No predecessor found"),
        };
        assert_eq!(
            metadata.len(),
            2,
            "Expected two metadats for wallet and pay_later"
        );
        assert!(matches!(
            *Weak::upgrade(&pred)
                .expect("Expected Arc not found")
                .deref(),
            graph::AnalysisTrace::Value {
                value: graph::NodeValue::Value(dir::DirValue::CaptureMethod(
                    dir::enums::CaptureMethod::Automatic
                )),
                relation: graph::Relation::Positive,
                info: None,
                metadata: None,
                predecessors: None,
            }
        ));
    }
}
