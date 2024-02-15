use std::sync::{Arc, Weak};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    builder,
    dense_map::DenseMap,
    error::{self, AnalysisTrace, GraphError},
    types::{
        CheckingContext, CycleCheck, DomainId, DomainIdentifier, DomainInfo, Edge, EdgeId,
        Memoization, Metadata, Node, NodeId, NodeType, NodeValue, Relation, RelationResolution,
        Strength, ValueNode,
    },
};

struct CheckNodeContext<'a, V: ValueNode, C: CheckingContext<Value = V>> {
    ctx: &'a C,
    node: &'a Node<V>,
    node_id: NodeId,
    relation: Relation,
    strength: Strength,
    memo: &'a mut Memoization<V>,
    cycle_map: &'a mut CycleCheck,
    domains: Option<&'a [DomainId]>,
}

pub struct ConstraintGraph<'a, V: ValueNode> {
    pub domain: DenseMap<DomainId, DomainInfo<'a>>,
    pub domain_identifier_map: FxHashMap<DomainIdentifier<'a>, DomainId>,
    pub nodes: DenseMap<NodeId, Node<V>>,
    pub edges: DenseMap<EdgeId, Edge>,
    pub value_map: FxHashMap<NodeValue<V>, NodeId>,
    pub node_info: DenseMap<NodeId, Option<&'static str>>,
    pub node_metadata: DenseMap<NodeId, Option<Arc<dyn Metadata>>>,
}

impl<'a, V> ConstraintGraph<'a, V>
where
    V: ValueNode,
{
    fn get_predecessor_edges_by_domain(
        &self,
        node_id: NodeId,
        domains: Option<&[DomainId]>,
    ) -> Result<Vec<&Edge>, GraphError<V>> {
        let node = self.nodes.get(node_id).ok_or(GraphError::NodeNotFound)?;
        let mut final_list = Vec::new();
        for &pred in &node.preds {
            let edge = self.edges.get(pred).ok_or(GraphError::EdgeNotFound)?;
            if let Some((domain_id, domains)) = edge.domain.zip(domains) {
                if domains.contains(&domain_id) {
                    final_list.push(edge);
                }
            } else if edge.domain.is_none() {
                final_list.push(edge);
            }
        }

        Ok(final_list)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_node<C>(
        &self,
        ctx: &C,
        node_id: NodeId,
        relation: Relation,
        strength: Strength,
        memo: &mut Memoization<V>,
        cycle_map: &mut CycleCheck,
        domains: Option<&[&str]>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let domains = domains
            .map(|domain_idents| {
                domain_idents
                    .iter()
                    .map(|domain_ident| {
                        self.domain_identifier_map
                            .get(&DomainIdentifier::new(domain_ident))
                            .copied()
                            .ok_or(GraphError::DomainNotFound)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;

        self.check_node_inner(
            ctx,
            node_id,
            relation,
            strength,
            memo,
            cycle_map,
            domains.as_deref(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn check_node_inner<C>(
        &self,
        ctx: &C,
        node_id: NodeId,
        relation: Relation,
        strength: Strength,
        memo: &mut Memoization<V>,
        cycle_map: &mut CycleCheck,
        domains: Option<&[DomainId]>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let node = self.nodes.get(node_id).ok_or(GraphError::NodeNotFound)?;

        if let Some(already_memo) = memo.get(&(node_id, relation, strength)) {
            already_memo
                .clone()
                .map_err(|err| GraphError::AnalysisError(Arc::downgrade(&err)))
        } else if let Some((initial_strength, initial_relation)) = cycle_map.get(&node_id).cloned()
        {
            let strength_relation = Strength::get_resolved_strength(initial_strength, strength);
            let relation_resolve =
                RelationResolution::get_resolved_relation(initial_relation, relation.into());
            cycle_map.entry(node_id).and_modify(|value| {
                value.0 = strength_relation;
                value.1 = relation_resolve
            });
            Ok(())
        } else {
            let check_node_context = CheckNodeContext {
                node,
                node_id,
                relation,
                strength,
                memo,
                cycle_map,
                ctx,
                domains,
            };
            match &node.node_type {
                NodeType::AllAggregator => self.validate_all_aggregator(check_node_context),

                NodeType::AnyAggregator => self.validate_any_aggregator(check_node_context),

                NodeType::InAggregator(expected) => {
                    self.validate_in_aggregator(check_node_context, expected)
                }
                NodeType::Value(val) => self.validate_value_node(check_node_context, val),
            }
        }
    }

    fn validate_all_aggregator<C>(
        &self,
        vald: CheckNodeContext<'_, V, C>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let mut unsatisfied = Vec::<Weak<AnalysisTrace<V>>>::new();

        for edge in self.get_predecessor_edges_by_domain(vald.node_id, vald.domains)? {
            vald.cycle_map
                .insert(vald.node_id, (vald.strength, vald.relation.into()));
            if let Err(e) = self.check_node_inner(
                vald.ctx,
                edge.pred,
                edge.relation,
                edge.strength,
                vald.memo,
                vald.cycle_map,
                vald.domains,
            ) {
                unsatisfied.push(e.get_analysis_trace()?);
            }
            if let Some((_resolved_strength, resolved_relation)) =
                vald.cycle_map.remove(&vald.node_id)
            {
                if resolved_relation == RelationResolution::Contradiction {
                    let err = Arc::new(AnalysisTrace::Contradiction {
                        relation: resolved_relation,
                    });
                    vald.memo.insert(
                        (vald.node_id, vald.relation, vald.strength),
                        Err(Arc::clone(&err)),
                    );
                    return Err(GraphError::AnalysisError(Arc::downgrade(&err)));
                }
            }
        }

        if !unsatisfied.is_empty() {
            let err = Arc::new(AnalysisTrace::AllAggregation {
                unsatisfied,
                info: self.node_info.get(vald.node_id).cloned().flatten(),
                metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
            });

            vald.memo.insert(
                (vald.node_id, vald.relation, vald.strength),
                Err(Arc::clone(&err)),
            );
            Err(GraphError::AnalysisError(Arc::downgrade(&err)))
        } else {
            vald.memo
                .insert((vald.node_id, vald.relation, vald.strength), Ok(()));
            Ok(())
        }
    }

    fn validate_any_aggregator<C>(
        &self,
        vald: CheckNodeContext<'_, V, C>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let mut unsatisfied = Vec::<Weak<AnalysisTrace<V>>>::new();
        let mut matched_one = false;

        for edge in self.get_predecessor_edges_by_domain(vald.node_id, vald.domains)? {
            vald.cycle_map
                .insert(vald.node_id, (vald.strength, vald.relation.into()));
            if let Err(e) = self.check_node_inner(
                vald.ctx,
                edge.pred,
                edge.relation,
                edge.strength,
                vald.memo,
                vald.cycle_map,
                vald.domains,
            ) {
                unsatisfied.push(e.get_analysis_trace()?);
            } else {
                matched_one = true;
            }
            if let Some((_resolved_strength, resolved_relation)) =
                vald.cycle_map.remove(&vald.node_id)
            {
                if resolved_relation == RelationResolution::Contradiction {
                    let err = Arc::new(AnalysisTrace::Contradiction {
                        relation: resolved_relation,
                    });
                    vald.memo.insert(
                        (vald.node_id, vald.relation, vald.strength),
                        Err(Arc::clone(&err)),
                    );

                    return Err(GraphError::AnalysisError(Arc::downgrade(&err)));
                }
            }
        }

        if matched_one || vald.node.preds.is_empty() {
            vald.memo
                .insert((vald.node_id, vald.relation, vald.strength), Ok(()));
            Ok(())
        } else {
            let err = Arc::new(AnalysisTrace::AnyAggregation {
                unsatisfied: unsatisfied.clone(),
                info: self.node_info.get(vald.node_id).cloned().flatten(),
                metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
            });

            vald.memo.insert(
                (vald.node_id, vald.relation, vald.strength),
                Err(Arc::clone(&err)),
            );
            Err(GraphError::AnalysisError(Arc::downgrade(&err)))
        }
    }

    fn validate_in_aggregator<C>(
        &self,
        vald: CheckNodeContext<'_, V, C>,
        expected: &FxHashSet<V>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let the_key = expected
            .iter()
            .next()
            .ok_or_else(|| GraphError::MalformedGraph {
                reason: "An OnlyIn aggregator node must have at least one expected value"
                    .to_string(),
            })?
            .get_key();

        let ctx_vals = if let Some(vals) = vald.ctx.get_values_by_key(&the_key) {
            vals
        } else {
            return if let Strength::Weak = vald.strength {
                vald.memo
                    .insert((vald.node_id, vald.relation, vald.strength), Ok(()));
                Ok(())
            } else {
                let err = Arc::new(AnalysisTrace::InAggregation {
                    expected: expected.iter().cloned().collect(),
                    found: None,
                    relation: vald.relation,
                    info: self.node_info.get(vald.node_id).cloned().flatten(),
                    metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
                });

                vald.memo.insert(
                    (vald.node_id, vald.relation, vald.strength),
                    Err(Arc::clone(&err)),
                );
                Err(GraphError::AnalysisError(Arc::downgrade(&err)))
            };
        };

        let relation_bool: bool = vald.relation.into();
        for ctx_value in ctx_vals {
            if expected.contains(&ctx_value) != relation_bool {
                let err = Arc::new(AnalysisTrace::InAggregation {
                    expected: expected.iter().cloned().collect(),
                    found: Some(ctx_value.clone()),
                    relation: vald.relation,
                    info: self.node_info.get(vald.node_id).cloned().flatten(),
                    metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
                });

                vald.memo.insert(
                    (vald.node_id, vald.relation, vald.strength),
                    Err(Arc::clone(&err)),
                );
                Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
            }
        }

        vald.memo
            .insert((vald.node_id, vald.relation, vald.strength), Ok(()));
        Ok(())
    }

    fn validate_value_node<C>(
        &self,
        vald: CheckNodeContext<'_, V, C>,
        val: &NodeValue<V>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let mut errors = Vec::<Weak<AnalysisTrace<V>>>::new();
        let mut matched_one = false;

        self.context_analysis(
            vald.node_id,
            vald.relation,
            vald.strength,
            vald.ctx,
            val,
            vald.memo,
        )?;

        for edge in self.get_predecessor_edges_by_domain(vald.node_id, vald.domains)? {
            vald.cycle_map
                .insert(vald.node_id, (vald.strength, vald.relation.into()));
            let result = self.check_node_inner(
                vald.ctx,
                edge.pred,
                edge.relation,
                edge.strength,
                vald.memo,
                vald.cycle_map,
                vald.domains,
            );

            if let Some((resolved_strength, resolved_relation)) =
                vald.cycle_map.remove(&vald.node_id)
            {
                if resolved_relation == RelationResolution::Contradiction {
                    let err = Arc::new(AnalysisTrace::Contradiction {
                        relation: resolved_relation,
                    });
                    vald.memo.insert(
                        (vald.node_id, vald.relation, vald.strength),
                        Err(Arc::clone(&err)),
                    );
                    return Err(GraphError::AnalysisError(Arc::downgrade(&err)));
                } else if resolved_strength != vald.strength {
                    self.context_analysis(
                        vald.node_id,
                        vald.relation,
                        resolved_strength,
                        vald.ctx,
                        val,
                        vald.memo,
                    )?
                }
            }
            match (edge.strength, result) {
                (Strength::Strong, Err(trace)) => {
                    let err = Arc::new(AnalysisTrace::Value {
                        value: val.clone(),
                        relation: vald.relation,
                        info: self.node_info.get(vald.node_id).cloned().flatten(),
                        metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
                        predecessors: Some(error::ValueTracePredecessor::Mandatory(Box::new(
                            trace.get_analysis_trace()?,
                        ))),
                    });
                    vald.memo.insert(
                        (vald.node_id, vald.relation, vald.strength),
                        Err(Arc::clone(&err)),
                    );
                    Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
                }

                (Strength::Strong, Ok(_)) => {
                    matched_one = true;
                }

                (Strength::Normal | Strength::Weak, Err(trace)) => {
                    errors.push(trace.get_analysis_trace()?);
                }

                (Strength::Normal | Strength::Weak, Ok(_)) => {
                    matched_one = true;
                }
            }
        }

        if matched_one || vald.node.preds.is_empty() {
            vald.memo
                .insert((vald.node_id, vald.relation, vald.strength), Ok(()));
            Ok(())
        } else {
            let err = Arc::new(AnalysisTrace::Value {
                value: val.clone(),
                relation: vald.relation,
                info: self.node_info.get(vald.node_id).cloned().flatten(),
                metadata: self.node_metadata.get(vald.node_id).cloned().flatten(),
                predecessors: Some(error::ValueTracePredecessor::OneOf(errors.clone())),
            });

            vald.memo.insert(
                (vald.node_id, vald.relation, vald.strength),
                Err(Arc::clone(&err)),
            );
            Err(GraphError::AnalysisError(Arc::downgrade(&err)))
        }
    }

    fn context_analysis<C>(
        &self,
        node_id: NodeId,
        relation: Relation,
        strength: Strength,
        ctx: &C,
        val: &NodeValue<V>,
        memo: &mut Memoization<V>,
    ) -> Result<(), GraphError<V>>
    where
        C: CheckingContext<Value = V>,
    {
        let in_context = ctx.check_presence(val, strength);
        let relation_bool: bool = relation.into();
        if in_context != relation_bool {
            let err = Arc::new(AnalysisTrace::Value {
                value: val.clone(),
                relation,
                predecessors: None,
                info: self.node_info.get(node_id).cloned().flatten(),
                metadata: self.node_metadata.get(node_id).cloned().flatten(),
            });
            memo.insert((node_id, relation, strength), Err(Arc::clone(&err)));
            Err(GraphError::AnalysisError(Arc::downgrade(&err)))?;
        }
        if !relation_bool {
            memo.insert((node_id, relation, strength), Ok(()));
            return Ok(());
        }
        Ok(())
    }

    pub fn combine<'b>(g1: &'b Self, g2: &'b Self) -> Result<Self, GraphError<V>> {
        let mut node_builder = builder::ConstraintGraphBuilder::new();
        let mut g1_old2new_id = DenseMap::<NodeId, NodeId>::new();
        let mut g2_old2new_id = DenseMap::<NodeId, NodeId>::new();
        let mut g1_old2new_domain_id = DenseMap::<DomainId, DomainId>::new();
        let mut g2_old2new_domain_id = DenseMap::<DomainId, DomainId>::new();

        let add_domain = |node_builder: &mut builder::ConstraintGraphBuilder<'a, V>,
                          domain: DomainInfo<'a>|
         -> Result<DomainId, GraphError<V>> {
            node_builder.make_domain(
                domain.domain_identifier.into_inner(),
                domain.domain_description,
            )
        };

        let add_node = |node_builder: &mut builder::ConstraintGraphBuilder<'a, V>,
                        node: &Node<V>|
         -> Result<NodeId, GraphError<V>> {
            match &node.node_type {
                NodeType::Value(node_value) => {
                    Ok(node_builder.make_value_node(node_value.clone(), None, None::<()>))
                }

                NodeType::AllAggregator => {
                    Ok(node_builder.make_all_aggregator(&[], None, None::<()>, None)?)
                }

                NodeType::AnyAggregator => {
                    Ok(node_builder.make_any_aggregator(&[], None, None::<()>, None)?)
                }

                NodeType::InAggregator(expected) => Ok(node_builder.make_in_aggregator(
                    expected.iter().cloned().collect(),
                    None,
                    None::<()>,
                )?),
            }
        };

        for (_old_domain_id, domain) in g1.domain.iter() {
            let new_domain_id = add_domain(&mut node_builder, domain.clone())?;
            g1_old2new_domain_id.push(new_domain_id);
        }

        for (_old_domain_id, domain) in g2.domain.iter() {
            let new_domain_id = add_domain(&mut node_builder, domain.clone())?;
            g2_old2new_domain_id.push(new_domain_id);
        }

        for (_old_node_id, node) in g1.nodes.iter() {
            let new_node_id = add_node(&mut node_builder, node)?;
            g1_old2new_id.push(new_node_id);
        }

        for (_old_node_id, node) in g2.nodes.iter() {
            let new_node_id = add_node(&mut node_builder, node)?;
            g2_old2new_id.push(new_node_id);
        }

        for edge in g1.edges.values() {
            let new_pred_id = g1_old2new_id
                .get(edge.pred)
                .ok_or(GraphError::NodeNotFound)?;
            let new_succ_id = g1_old2new_id
                .get(edge.succ)
                .ok_or(GraphError::NodeNotFound)?;
            let domain_ident = edge
                .domain
                .map(|domain_id| g1.domain.get(domain_id).ok_or(GraphError::DomainNotFound))
                .transpose()?
                .map(|domain| domain.domain_identifier);

            node_builder.make_edge(
                *new_pred_id,
                *new_succ_id,
                edge.strength,
                edge.relation,
                domain_ident.as_deref(),
            )?;
        }

        for edge in g2.edges.values() {
            let new_pred_id = g2_old2new_id
                .get(edge.pred)
                .ok_or(GraphError::NodeNotFound)?;
            let new_succ_id = g2_old2new_id
                .get(edge.succ)
                .ok_or(GraphError::NodeNotFound)?;
            let domain_ident = edge
                .domain
                .map(|domain_id| g2.domain.get(domain_id).ok_or(GraphError::DomainNotFound))
                .transpose()?
                .map(|domain| domain.domain_identifier);

            node_builder.make_edge(
                *new_pred_id,
                *new_succ_id,
                edge.strength,
                edge.relation,
                domain_ident.as_deref(),
            )?;
        }

        Ok(node_builder.build())
    }
}
