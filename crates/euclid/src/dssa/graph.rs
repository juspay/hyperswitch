use std::{fmt::Debug, sync::Weak};

use hyperswitch_constraint_graph as cgraph;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    dssa::types,
    frontend::dir,
    types::{DataType, Metadata},
};

pub mod euclid_graph_prelude {
    pub use hyperswitch_constraint_graph as cgraph;
    pub use rustc_hash::{FxHashMap, FxHashSet};

    pub use crate::{
        dssa::graph::*,
        frontend::dir::{enums::*, DirKey, DirKeyKind, DirValue},
        types::*,
    };
}

impl cgraph::KeyNode for dir::DirKey {}

impl cgraph::ValueNode for dir::DirValue {
    type Key = dir::DirKey;

    fn get_key(&self) -> Self::Key {
        Self::get_key(self)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", content = "details", rename_all = "snake_case")]
pub enum AnalysisError<V: cgraph::ValueNode> {
    Graph(cgraph::GraphError<V>),
    AssertionTrace {
        trace: Weak<cgraph::AnalysisTrace<V>>,
        metadata: Metadata,
    },
    NegationTrace {
        trace: Weak<cgraph::AnalysisTrace<V>>,
        metadata: Vec<Metadata>,
    },
}

impl<V: cgraph::ValueNode> AnalysisError<V> {
    fn assertion_from_graph_error(metadata: &Metadata, graph_error: cgraph::GraphError<V>) -> Self {
        match graph_error {
            cgraph::GraphError::AnalysisError(trace) => Self::AssertionTrace {
                trace,
                metadata: metadata.clone(),
            },

            other => Self::Graph(other),
        }
    }

    fn negation_from_graph_error(
        metadata: Vec<&Metadata>,
        graph_error: cgraph::GraphError<V>,
    ) -> Self {
        match graph_error {
            cgraph::GraphError::AnalysisError(trace) => Self::NegationTrace {
                trace,
                metadata: metadata.iter().map(|m| (*m).clone()).collect(),
            },

            other => Self::Graph(other),
        }
    }
}

pub struct AnalysisContext {
    keywise_values: FxHashMap<dir::DirKey, FxHashSet<dir::DirValue>>,
}

impl AnalysisContext {
    pub fn from_dir_values(vals: impl IntoIterator<Item = dir::DirValue>) -> Self {
        let mut keywise_values: FxHashMap<dir::DirKey, FxHashSet<dir::DirValue>> =
            FxHashMap::default();

        for dir_val in vals {
            let key = dir_val.get_key();
            let set = keywise_values.entry(key).or_default();
            set.insert(dir_val);
        }

        Self { keywise_values }
    }

    pub fn insert(&mut self, value: dir::DirValue) {
        self.keywise_values
            .entry(value.get_key())
            .or_default()
            .insert(value);
    }

    pub fn remove(&mut self, value: dir::DirValue) {
        let set = self.keywise_values.entry(value.get_key()).or_default();

        set.remove(&value);

        if set.is_empty() {
            self.keywise_values.remove(&value.get_key());
        }
    }
}

impl cgraph::CheckingContext for AnalysisContext {
    type Value = dir::DirValue;

    fn from_node_values<L>(vals: impl IntoIterator<Item = L>) -> Self
    where
        L: Into<Self::Value>,
    {
        let mut keywise_values: FxHashMap<dir::DirKey, FxHashSet<dir::DirValue>> =
            FxHashMap::default();

        for dir_val in vals.into_iter().map(L::into) {
            let key = dir_val.get_key();
            let set = keywise_values.entry(key).or_default();
            set.insert(dir_val);
        }

        Self { keywise_values }
    }

    fn check_presence(
        &self,
        value: &cgraph::NodeValue<dir::DirValue>,
        strength: cgraph::Strength,
    ) -> bool {
        match value {
            cgraph::NodeValue::Key(k) => {
                self.keywise_values.contains_key(k) || matches!(strength, cgraph::Strength::Weak)
            }

            cgraph::NodeValue::Value(val) => {
                let key = val.get_key();
                let value_set = if let Some(set) = self.keywise_values.get(&key) {
                    set
                } else {
                    return matches!(strength, cgraph::Strength::Weak);
                };

                match key.kind.get_type() {
                    DataType::EnumVariant | DataType::StrValue | DataType::MetadataValue => {
                        value_set.contains(val)
                    }
                    DataType::Number => val.get_num_value().map_or(false, |num_val| {
                        value_set.iter().any(|ctx_val| {
                            ctx_val
                                .get_num_value()
                                .map_or(false, |ctx_num_val| num_val.fits(&ctx_num_val))
                        })
                    }),
                }
            }
        }
    }

    fn get_values_by_key(
        &self,
        key: &<Self::Value as cgraph::ValueNode>::Key,
    ) -> Option<Vec<Self::Value>> {
        self.keywise_values
            .get(key)
            .map(|set| set.iter().cloned().collect())
    }
}

pub trait CgraphExt {
    fn key_analysis(
        &self,
        key: dir::DirKey,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>>;

    fn value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>>;

    fn check_value_validity(
        &self,
        val: dir::DirValue,
        analysis_ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<bool, cgraph::GraphError<dir::DirValue>>;

    fn key_value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>>;

    fn assertion_analysis(
        &self,
        positive_ctx: &[(&dir::DirValue, &Metadata)],
        analysis_ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>>;

    fn negation_analysis(
        &self,
        negative_ctx: &[(&[dir::DirValue], &Metadata)],
        analysis_ctx: &mut AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>>;

    fn perform_context_analysis(
        &self,
        ctx: &types::ConjunctiveContext<'_>,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>>;
}

impl CgraphExt for cgraph::ConstraintGraph<'_, dir::DirValue> {
    fn key_analysis(
        &self,
        key: dir::DirKey,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>> {
        self.value_map
            .get(&cgraph::NodeValue::Key(key))
            .map_or(Ok(()), |node_id| {
                self.check_node(
                    ctx,
                    *node_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                    memo,
                    cycle_map,
                    domains,
                )
            })
    }

    fn value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>> {
        self.value_map
            .get(&cgraph::NodeValue::Value(val))
            .map_or(Ok(()), |node_id| {
                self.check_node(
                    ctx,
                    *node_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                    memo,
                    cycle_map,
                    domains,
                )
            })
    }

    fn check_value_validity(
        &self,
        val: dir::DirValue,
        analysis_ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<bool, cgraph::GraphError<dir::DirValue>> {
        let maybe_node_id = self.value_map.get(&cgraph::NodeValue::Value(val));

        let node_id = if let Some(nid) = maybe_node_id {
            nid
        } else {
            return Ok(false);
        };

        let result = self.check_node(
            analysis_ctx,
            *node_id,
            cgraph::Relation::Positive,
            cgraph::Strength::Weak,
            memo,
            cycle_map,
            domains,
        );

        match result {
            Ok(_) => Ok(true),
            Err(e) => {
                e.get_analysis_trace()?;
                Ok(false)
            }
        }
    }

    fn key_value_analysis(
        &self,
        val: dir::DirValue,
        ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), cgraph::GraphError<dir::DirValue>> {
        self.key_analysis(val.get_key(), ctx, memo, cycle_map, domains)
            .and_then(|_| self.value_analysis(val, ctx, memo, cycle_map, domains))
    }

    fn assertion_analysis(
        &self,
        positive_ctx: &[(&dir::DirValue, &Metadata)],
        analysis_ctx: &AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>> {
        positive_ctx.iter().try_for_each(|(value, metadata)| {
            self.key_value_analysis((*value).clone(), analysis_ctx, memo, cycle_map, domains)
                .map_err(|e| AnalysisError::assertion_from_graph_error(metadata, e))
        })
    }

    fn negation_analysis(
        &self,
        negative_ctx: &[(&[dir::DirValue], &Metadata)],
        analysis_ctx: &mut AnalysisContext,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        cycle_map: &mut cgraph::CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>> {
        let mut keywise_metadata: FxHashMap<dir::DirKey, Vec<&Metadata>> = FxHashMap::default();
        let mut keywise_negation: FxHashMap<dir::DirKey, FxHashSet<&dir::DirValue>> =
            FxHashMap::default();

        for (values, metadata) in negative_ctx {
            let mut metadata_added = false;

            for dir_value in *values {
                if !metadata_added {
                    keywise_metadata
                        .entry(dir_value.get_key())
                        .or_default()
                        .push(metadata);

                    metadata_added = true;
                }

                keywise_negation
                    .entry(dir_value.get_key())
                    .or_default()
                    .insert(dir_value);
            }
        }

        for (key, negation_set) in keywise_negation {
            let all_metadata = keywise_metadata.remove(&key).unwrap_or_default();
            let first_metadata = all_metadata.first().cloned().cloned().unwrap_or_default();

            self.key_analysis(key.clone(), analysis_ctx, memo, cycle_map, domains)
                .map_err(|e| AnalysisError::assertion_from_graph_error(&first_metadata, e))?;

            let mut value_set = if let Some(set) = key.kind.get_value_set() {
                set
            } else {
                continue;
            };

            value_set.retain(|v| !negation_set.contains(v));

            for value in value_set {
                analysis_ctx.insert(value.clone());
                self.value_analysis(value.clone(), analysis_ctx, memo, cycle_map, domains)
                    .map_err(|e| {
                        AnalysisError::negation_from_graph_error(all_metadata.clone(), e)
                    })?;
                analysis_ctx.remove(value);
            }
        }

        Ok(())
    }

    fn perform_context_analysis(
        &self,
        ctx: &types::ConjunctiveContext<'_>,
        memo: &mut cgraph::Memoization<dir::DirValue>,
        domains: Option<&[&str]>,
    ) -> Result<(), AnalysisError<dir::DirValue>> {
        let mut analysis_ctx = AnalysisContext::from_dir_values(
            ctx.iter()
                .filter_map(|ctx_val| ctx_val.value.get_assertion().cloned()),
        );

        let positive_ctx = ctx
            .iter()
            .filter_map(|ctx_val| {
                ctx_val
                    .value
                    .get_assertion()
                    .map(|val| (val, ctx_val.metadata))
            })
            .collect::<Vec<_>>();
        self.assertion_analysis(
            &positive_ctx,
            &analysis_ctx,
            memo,
            &mut cgraph::CycleCheck::new(),
            domains,
        )?;

        let negative_ctx = ctx
            .iter()
            .filter_map(|ctx_val| {
                ctx_val
                    .value
                    .get_negation()
                    .map(|vals| (vals, ctx_val.metadata))
            })
            .collect::<Vec<_>>();
        self.negation_analysis(
            &negative_ctx,
            &mut analysis_ctx,
            memo,
            &mut cgraph::CycleCheck::new(),
            domains,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::ops::Deref;

    use euclid_macros::knowledge;
    use hyperswitch_constraint_graph::CycleCheck;

    use super::*;
    use crate::{dirval, frontend::dir::enums};

    #[test]
    fn test_strong_positive_relation_success() {
        let graph = knowledge! {
            PaymentMethod(Card) ->> CaptureMethod(Automatic);
            PaymentMethod(not Wallet)
                & PaymentMethod(not PayLater) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_strong_positive_relation_failure() {
        let graph = knowledge! {
            PaymentMethod(Card) ->> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([dirval!(CaptureMethod = Automatic)]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_strong_negative_relation_success() {
        let graph = knowledge! {
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_strong_negative_relation_failure() {
        let graph = knowledge! {
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Wallet),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_normal_one_of_failure() {
        let graph = knowledge! {
            PaymentMethod(Card) -> CaptureMethod(Automatic);
            PaymentMethod(Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );
        assert!(matches!(
            *Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
                .expect("Expected Arc"),
            cgraph::AnalysisTrace::Value {
                predecessors: Some(cgraph::error::ValueTracePredecessor::OneOf(_)),
                ..
            }
        ));
    }

    #[test]
    fn test_all_aggregator_success() {
        let graph = knowledge! {
            PaymentMethod(Card) & PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Automatic),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_all_aggregator_failure() {
        let graph = knowledge! {
            PaymentMethod(Card) & PaymentMethod(not Wallet) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_all_aggregator_mandatory_failure() {
        let graph = knowledge! {
            PaymentMethod(Card) & PaymentMethod(not Wallet) ->> CaptureMethod(Automatic);
        };
        let mut memo = cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
            ]),
            &mut memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(matches!(
            *Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
                .expect("Expected Arc"),
            cgraph::AnalysisTrace::Value {
                predecessors: Some(cgraph::error::ValueTracePredecessor::Mandatory(_)),
                ..
            }
        ));
    }

    #[test]
    fn test_in_aggregator_success() {
        let graph = knowledge! {
            PaymentMethod(in [Card, Wallet]) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_in_aggregator_failure() {
        let graph = knowledge! {
            PaymentMethod(in [Card, Wallet]) -> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_not_in_aggregator_success() {
        let graph = knowledge! {
            PaymentMethod(not in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
                dirval!(PaymentMethod = BankRedirect),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_not_in_aggregator_failure() {
        let graph = knowledge! {
            PaymentMethod(not in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = PayLater),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(PaymentMethod = Card),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_in_aggregator_failure_trace() {
        let graph = knowledge! {
            PaymentMethod(in [Card, Wallet]) ->> CaptureMethod(Automatic);
        };
        let memo = &mut cgraph::Memoization::new();
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
            ]),
            memo,
            &mut CycleCheck::new(),
            None,
        );

        if let cgraph::AnalysisTrace::Value {
            predecessors: Some(cgraph::error::ValueTracePredecessor::Mandatory(agg_error)),
            ..
        } = Weak::upgrade(&result.unwrap_err().get_analysis_trace().unwrap())
            .expect("Expected arc")
            .deref()
        {
            assert!(matches!(
                *Weak::upgrade(agg_error.deref()).expect("Expected Arc"),
                cgraph::AnalysisTrace::InAggregation {
                    found: Some(dir::DirValue::PaymentMethod(enums::PaymentMethod::PayLater)),
                    ..
                }
            ));
        } else {
            panic!("Failed unwrapping OnlyInAggregation trace from AnalysisTrace");
        }
    }

    #[test]
    fn test_memoization_in_kgraph() {
        let mut builder = cgraph::ConstraintGraphBuilder::new();
        let _node_1 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Wallet)),
            None,
            None::<()>,
        );
        let _node_2 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::BillingCountry(enums::BillingCountry::India)),
            None,
            None::<()>,
        );
        let _node_3 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::BusinessCountry(
                enums::BusinessCountry::UnitedStatesOfAmerica,
            )),
            None,
            None::<()>,
        );
        let mut memo = cgraph::Memoization::new();
        let mut cycle_map = CycleCheck::new();
        let _edge_1 = builder
            .make_edge(
                _node_1,
                _node_2,
                cgraph::Strength::Strong,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_2 = builder
            .make_edge(
                _node_2,
                _node_3,
                cgraph::Strength::Strong,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to an edge");
        let graph = builder.build();
        let _result = graph.key_value_analysis(
            dirval!(BusinessCountry = UnitedStatesOfAmerica),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Wallet),
                dirval!(BillingCountry = India),
                dirval!(BusinessCountry = UnitedStatesOfAmerica),
            ]),
            &mut memo,
            &mut cycle_map,
            None,
        );
        let _answer = memo
            .get(&(
                _node_3,
                cgraph::Relation::Positive,
                cgraph::Strength::Strong,
            ))
            .expect("Memoization not workng");
        matches!(_answer, Ok(()));
    }

    #[test]
    fn test_cycle_resolution_in_graph() {
        let mut builder = cgraph::ConstraintGraphBuilder::new();
        let _node_1 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Wallet)),
            None,
            None::<()>,
        );
        let _node_2 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Card)),
            None,
            None::<()>,
        );
        let mut memo = cgraph::Memoization::new();
        let mut cycle_map = cgraph::CycleCheck::new();
        let _edge_1 = builder
            .make_edge(
                _node_1,
                _node_2,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_2 = builder
            .make_edge(
                _node_2,
                _node_1,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to an edge");
        let graph = builder.build();
        let _result = graph.key_value_analysis(
            dirval!(PaymentMethod = Wallet),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = Card),
            ]),
            &mut memo,
            &mut cycle_map,
            None,
        );

        assert!(_result.is_ok());
    }

    #[test]
    fn test_cycle_resolution_in_graph1() {
        let mut builder = cgraph::ConstraintGraphBuilder::new();
        let _node_1 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(
                enums::CaptureMethod::Automatic,
            )),
            None,
            None::<()>,
        );

        let _node_2 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Card)),
            None,
            None::<()>,
        );
        let _node_3 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Wallet)),
            None,
            None::<()>,
        );
        let mut memo = cgraph::Memoization::new();
        let mut cycle_map = cgraph::CycleCheck::new();

        let _edge_1 = builder
            .make_edge(
                _node_1,
                _node_2,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_2 = builder
            .make_edge(
                _node_1,
                _node_3,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_3 = builder
            .make_edge(
                _node_2,
                _node_1,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_4 = builder
            .make_edge(
                _node_3,
                _node_1,
                cgraph::Strength::Strong,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");

        let graph = builder.build();
        let _result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(CaptureMethod = Automatic),
            ]),
            &mut memo,
            &mut cycle_map,
            None,
        );

        assert!(_result.is_ok());
    }

    #[test]
    fn test_cycle_resolution_in_graph2() {
        let mut builder = cgraph::ConstraintGraphBuilder::new();
        let _node_0 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::BillingCountry(
                enums::BillingCountry::Afghanistan,
            )),
            None,
            None::<()>,
        );

        let _node_1 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(
                enums::CaptureMethod::Automatic,
            )),
            None,
            None::<()>,
        );

        let _node_2 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Card)),
            None,
            None::<()>,
        );
        let _node_3 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Wallet)),
            None,
            None::<()>,
        );

        let _node_4 = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::PaymentCurrency(enums::PaymentCurrency::USD)),
            None,
            None::<()>,
        );

        let mut memo = cgraph::Memoization::new();
        let mut cycle_map = cgraph::CycleCheck::new();

        let _edge_1 = builder
            .make_edge(
                _node_0,
                _node_1,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_2 = builder
            .make_edge(
                _node_1,
                _node_2,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_3 = builder
            .make_edge(
                _node_1,
                _node_3,
                cgraph::Strength::Weak,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_4 = builder
            .make_edge(
                _node_3,
                _node_4,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_5 = builder
            .make_edge(
                _node_2,
                _node_4,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");

        let _edge_6 = builder
            .make_edge(
                _node_4,
                _node_1,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");
        let _edge_7 = builder
            .make_edge(
                _node_4,
                _node_0,
                cgraph::Strength::Normal,
                cgraph::Relation::Positive,
                None::<cgraph::DomainId>,
            )
            .expect("Failed to make an edge");

        let graph = builder.build();
        let _result = graph.key_value_analysis(
            dirval!(BillingCountry = Afghanistan),
            &AnalysisContext::from_dir_values([
                dirval!(PaymentCurrency = USD),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Wallet),
                dirval!(CaptureMethod = Automatic),
                dirval!(BillingCountry = Afghanistan),
            ]),
            &mut memo,
            &mut cycle_map,
            None,
        );

        assert!(_result.is_ok());
    }
}
