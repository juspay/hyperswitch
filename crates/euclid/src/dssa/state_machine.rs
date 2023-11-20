use super::types::EuclidAnalysable;
use crate::{dssa::types, frontend::dir, types::Metadata};

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
#[serde(tag = "type", content = "info", rename_all = "snake_case")]
pub enum StateMachineError {
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(&'static str),
}

#[derive(Debug)]
struct ComparisonStateMachine<'a> {
    values: &'a [dir::DirValue],
    logic: &'a dir::DirComparisonLogic,
    metadata: &'a Metadata,
    count: usize,
    ctx_idx: usize,
}

impl<'a> ComparisonStateMachine<'a> {
    #[inline]
    fn is_finished(&self) -> bool {
        self.count + 1 >= self.values.len()
            || matches!(self.logic, dir::DirComparisonLogic::NegativeConjunction)
    }

    #[inline]
    fn advance(&mut self) {
        if let dir::DirComparisonLogic::PositiveDisjunction = self.logic {
            self.count = (self.count + 1) % self.values.len();
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.count = 0;
    }

    #[inline]
    fn put(&self, context: &mut types::ConjunctiveContext<'a>) -> Result<(), StateMachineError> {
        if let dir::DirComparisonLogic::PositiveDisjunction = self.logic {
            *context
                .get_mut(self.ctx_idx)
                .ok_or(StateMachineError::IndexOutOfBounds(
                    "in ComparisonStateMachine while indexing into context",
                ))? = types::ContextValue::assertion(
                self.values
                    .get(self.count)
                    .ok_or(StateMachineError::IndexOutOfBounds(
                        "in ComparisonStateMachine while indexing into values",
                    ))?,
                self.metadata,
            );
        }
        Ok(())
    }

    #[inline]
    fn push(&self, context: &mut types::ConjunctiveContext<'a>) -> Result<(), StateMachineError> {
        match self.logic {
            dir::DirComparisonLogic::PositiveDisjunction => {
                context.push(types::ContextValue::assertion(
                    self.values
                        .get(self.count)
                        .ok_or(StateMachineError::IndexOutOfBounds(
                            "in ComparisonStateMachine while pushing",
                        ))?,
                    self.metadata,
                ));
            }

            dir::DirComparisonLogic::NegativeConjunction => {
                context.push(types::ContextValue::negation(self.values, self.metadata));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ConditionStateMachine<'a> {
    state_machines: Vec<ComparisonStateMachine<'a>>,
    start_ctx_idx: usize,
}

impl<'a> ConditionStateMachine<'a> {
    fn new(condition: &'a [dir::DirComparison], start_idx: usize) -> Self {
        let mut machines = Vec::<ComparisonStateMachine<'a>>::with_capacity(condition.len());

        let mut machine_idx = start_idx;
        for cond in condition {
            let machine = ComparisonStateMachine {
                values: &cond.values,
                logic: &cond.logic,
                metadata: &cond.metadata,
                count: 0,
                ctx_idx: machine_idx,
            };
            machines.push(machine);
            machine_idx += 1;
        }

        Self {
            state_machines: machines,
            start_ctx_idx: start_idx,
        }
    }

    fn init(&self, context: &mut types::ConjunctiveContext<'a>) -> Result<(), StateMachineError> {
        for machine in &self.state_machines {
            machine.push(context)?;
        }
        Ok(())
    }

    #[inline]
    fn destroy(&self, context: &mut types::ConjunctiveContext<'a>) {
        context.truncate(self.start_ctx_idx);
    }

    #[inline]
    fn is_finished(&self) -> bool {
        !self
            .state_machines
            .iter()
            .any(|machine| !machine.is_finished())
    }

    #[inline]
    fn get_next_ctx_idx(&self) -> usize {
        self.start_ctx_idx + self.state_machines.len()
    }

    fn advance(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        for machine in self.state_machines.iter_mut().rev() {
            if machine.is_finished() {
                machine.reset();
                machine.put(context)?;
            } else {
                machine.advance();
                machine.put(context)?;
                break;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct IfStmtStateMachine<'a> {
    condition_machine: ConditionStateMachine<'a>,
    nested: Vec<&'a dir::DirIfStatement>,
    nested_idx: usize,
}

impl<'a> IfStmtStateMachine<'a> {
    fn new(stmt: &'a dir::DirIfStatement, ctx_start_idx: usize) -> Self {
        let condition_machine = ConditionStateMachine::new(&stmt.condition, ctx_start_idx);
        let nested: Vec<&'a dir::DirIfStatement> = match &stmt.nested {
            None => Vec::new(),
            Some(nested_stmts) => nested_stmts.iter().collect(),
        };

        Self {
            condition_machine,
            nested,
            nested_idx: 0,
        }
    }

    fn init(
        &self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<Option<Self>, StateMachineError> {
        self.condition_machine.init(context)?;
        Ok(self
            .nested
            .first()
            .map(|nested| Self::new(nested, self.condition_machine.get_next_ctx_idx())))
    }

    #[inline]
    fn is_finished(&self) -> bool {
        self.nested_idx + 1 >= self.nested.len()
    }

    #[inline]
    fn is_condition_machine_finished(&self) -> bool {
        self.condition_machine.is_finished()
    }

    #[inline]
    fn destroy(&self, context: &mut types::ConjunctiveContext<'a>) {
        self.condition_machine.destroy(context);
    }

    #[inline]
    fn advance_condition_machine(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        self.condition_machine.advance(context)?;
        Ok(())
    }

    fn advance(&mut self) -> Result<Option<Self>, StateMachineError> {
        if self.nested.is_empty() {
            Ok(None)
        } else {
            self.nested_idx = (self.nested_idx + 1) % self.nested.len();
            Ok(Some(Self::new(
                self.nested
                    .get(self.nested_idx)
                    .ok_or(StateMachineError::IndexOutOfBounds(
                        "in IfStmtStateMachine while advancing",
                    ))?,
                self.condition_machine.get_next_ctx_idx(),
            )))
        }
    }
}

#[derive(Debug)]
struct RuleStateMachine<'a> {
    connector_selection_data: &'a [(dir::DirValue, Metadata)],
    connectors_added: bool,
    if_stmt_machines: Vec<IfStmtStateMachine<'a>>,
    running_stack: Vec<IfStmtStateMachine<'a>>,
}

impl<'a> RuleStateMachine<'a> {
    fn new<O>(
        rule: &'a dir::DirRule<O>,
        connector_selection_data: &'a [(dir::DirValue, Metadata)],
    ) -> Self {
        let mut if_stmt_machines: Vec<IfStmtStateMachine<'a>> =
            Vec::with_capacity(rule.statements.len());

        for stmt in rule.statements.iter().rev() {
            if_stmt_machines.push(IfStmtStateMachine::new(
                stmt,
                connector_selection_data.len(),
            ));
        }

        Self {
            connector_selection_data,
            connectors_added: false,
            if_stmt_machines,
            running_stack: Vec::new(),
        }
    }

    fn is_finished(&self) -> bool {
        self.if_stmt_machines.is_empty() && self.running_stack.is_empty()
    }

    fn init_next(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        if self.if_stmt_machines.is_empty() || !self.running_stack.is_empty() {
            return Ok(());
        }

        if !self.connectors_added {
            for (dir_val, metadata) in self.connector_selection_data {
                context.push(types::ContextValue::assertion(dir_val, metadata));
            }
            self.connectors_added = true;
        }

        context.truncate(self.connector_selection_data.len());

        if let Some(mut next_running) = self.if_stmt_machines.pop() {
            while let Some(nested_running) = next_running.init(context)? {
                self.running_stack.push(next_running);
                next_running = nested_running;
            }

            self.running_stack.push(next_running);
        }

        Ok(())
    }

    fn advance(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        let mut condition_machines_finished = true;

        for stmt_machine in self.running_stack.iter_mut().rev() {
            if !stmt_machine.is_condition_machine_finished() {
                condition_machines_finished = false;
                stmt_machine.advance_condition_machine(context)?;
                break;
            } else {
                stmt_machine.advance_condition_machine(context)?;
            }
        }

        if !condition_machines_finished {
            return Ok(());
        }

        let mut maybe_next_running: Option<IfStmtStateMachine<'a>> = None;

        while let Some(last) = self.running_stack.last_mut() {
            if !last.is_finished() {
                maybe_next_running = last.advance()?;
                break;
            } else {
                last.destroy(context);
                self.running_stack.pop();
            }
        }

        if let Some(mut next_running) = maybe_next_running {
            while let Some(nested_running) = next_running.init(context)? {
                self.running_stack.push(next_running);
                next_running = nested_running;
            }

            self.running_stack.push(next_running);
        } else {
            self.init_next(context)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct RuleContextManager<'a> {
    context: types::ConjunctiveContext<'a>,
    machine: RuleStateMachine<'a>,
    init: bool,
}

impl<'a> RuleContextManager<'a> {
    pub fn new<O>(
        rule: &'a dir::DirRule<O>,
        connector_selection_data: &'a [(dir::DirValue, Metadata)],
    ) -> Self {
        Self {
            context: Vec::new(),
            machine: RuleStateMachine::new(rule, connector_selection_data),
            init: false,
        }
    }

    pub fn advance(&mut self) -> Result<Option<&types::ConjunctiveContext<'a>>, StateMachineError> {
        if !self.init {
            self.init = true;
            self.machine.init_next(&mut self.context)?;
            Ok(Some(&self.context))
        } else if self.machine.is_finished() {
            Ok(None)
        } else {
            self.machine.advance(&mut self.context)?;

            if self.machine.is_finished() {
                Ok(None)
            } else {
                Ok(Some(&self.context))
            }
        }
    }

    pub fn advance_mut(
        &mut self,
    ) -> Result<Option<&mut types::ConjunctiveContext<'a>>, StateMachineError> {
        if !self.init {
            self.init = true;
            self.machine.init_next(&mut self.context)?;
            Ok(Some(&mut self.context))
        } else if self.machine.is_finished() {
            Ok(None)
        } else {
            self.machine.advance(&mut self.context)?;

            if self.machine.is_finished() {
                Ok(None)
            } else {
                Ok(Some(&mut self.context))
            }
        }
    }
}

#[derive(Debug)]
pub struct ProgramStateMachine<'a> {
    rule_machines: Vec<RuleStateMachine<'a>>,
    current_rule_machine: Option<RuleStateMachine<'a>>,
    is_init: bool,
}

impl<'a> ProgramStateMachine<'a> {
    pub fn new<O>(
        program: &'a dir::DirProgram<O>,
        connector_selection_data: &'a [Vec<(dir::DirValue, Metadata)>],
    ) -> Self {
        let mut rule_machines: Vec<RuleStateMachine<'a>> = program
            .rules
            .iter()
            .zip(connector_selection_data.iter())
            .rev()
            .map(|(rule, connector_selection_data)| {
                RuleStateMachine::new(rule, connector_selection_data)
            })
            .collect();

        Self {
            current_rule_machine: rule_machines.pop(),
            rule_machines,
            is_init: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.current_rule_machine
            .as_ref()
            .map_or(true, |rsm| rsm.is_finished())
            && self.rule_machines.is_empty()
    }

    pub fn init(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        if !self.is_init {
            if let Some(rsm) = self.current_rule_machine.as_mut() {
                rsm.init_next(context)?;
            }
            self.is_init = true;
        }

        Ok(())
    }

    pub fn advance(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        if self
            .current_rule_machine
            .as_ref()
            .map_or(true, |rsm| rsm.is_finished())
        {
            self.current_rule_machine = self.rule_machines.pop();
            context.clear();
            if let Some(rsm) = self.current_rule_machine.as_mut() {
                rsm.init_next(context)?;
            }
        } else if let Some(rsm) = self.current_rule_machine.as_mut() {
            rsm.advance(context)?;
        }

        Ok(())
    }
}

pub struct AnalysisContextManager<'a> {
    context: types::ConjunctiveContext<'a>,
    machine: ProgramStateMachine<'a>,
    init: bool,
}

impl<'a> AnalysisContextManager<'a> {
    pub fn new<O>(
        program: &'a dir::DirProgram<O>,
        connector_selection_data: &'a [Vec<(dir::DirValue, Metadata)>],
    ) -> Self {
        let machine = ProgramStateMachine::new(program, connector_selection_data);
        let context: types::ConjunctiveContext<'a> = Vec::new();

        Self {
            context,
            machine,
            init: false,
        }
    }

    pub fn advance(&mut self) -> Result<Option<&types::ConjunctiveContext<'a>>, StateMachineError> {
        if !self.init {
            self.init = true;
            self.machine.init(&mut self.context)?;
            Ok(Some(&self.context))
        } else if self.machine.is_finished() {
            Ok(None)
        } else {
            self.machine.advance(&mut self.context)?;

            if self.machine.is_finished() {
                Ok(None)
            } else {
                Ok(Some(&self.context))
            }
        }
    }
}

pub fn make_connector_selection_data<O: EuclidAnalysable>(
    program: &dir::DirProgram<O>,
) -> Vec<Vec<(dir::DirValue, Metadata)>> {
    program
        .rules
        .iter()
        .map(|rule| {
            rule.connector_selection
                .get_dir_value_for_analysis(rule.name.clone())
        })
        .collect()
}

#[cfg(all(test, feature = "ast_parser"))]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::{dirval, frontend::ast, types::DummyOutput};

    #[test]
    fn test_correct_contexts() {
        let program_str = r#"
            default: ["stripe", "adyen"]

            stripe_first: ["stripe", "adyen"]
            {
                payment_method = wallet {
                    payment_method = (card, bank_redirect) {
                        currency = USD
                        currency = GBP
                    }

                    payment_method = pay_later {
                        capture_method = automatic
                        capture_method = manual
                    }
                }

                payment_method = card {
                    payment_method = (card, bank_redirect) & capture_method = (automatic, manual) {
                        currency = (USD, GBP)
                    }
                }
            }
        "#;
        let (_, program) = ast::parser::program::<DummyOutput>(program_str).expect("Program");
        let lowered = ast::lowering::lower_program(program).expect("Lowering");

        let selection_data = make_connector_selection_data(&lowered);
        let mut state_machine = ProgramStateMachine::new(&lowered, &selection_data);
        let mut ctx: types::ConjunctiveContext<'_> = Vec::new();
        state_machine.init(&mut ctx).expect("State machine init");

        let expected_contexts: Vec<Vec<dir::DirValue>> = vec![
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentCurrency = GBP),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(PaymentCurrency = GBP),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
                dirval!(CaptureMethod = Automatic),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Wallet),
                dirval!(PaymentMethod = PayLater),
                dirval!(CaptureMethod = Manual),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentCurrency = GBP),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Manual),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = Card),
                dirval!(CaptureMethod = Manual),
                dirval!(PaymentCurrency = GBP),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(CaptureMethod = Automatic),
                dirval!(PaymentCurrency = GBP),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(CaptureMethod = Manual),
                dirval!(PaymentCurrency = USD),
            ],
            vec![
                dirval!("MetadataKey" = "stripe"),
                dirval!("MetadataKey" = "adyen"),
                dirval!(PaymentMethod = Card),
                dirval!(PaymentMethod = BankRedirect),
                dirval!(CaptureMethod = Manual),
                dirval!(PaymentCurrency = GBP),
            ],
        ];

        let mut expected_idx = 0usize;
        while !state_machine.is_finished() {
            let values = ctx
                .iter()
                .flat_map(|c| match c.value {
                    types::CtxValueKind::Assertion(val) => vec![val],
                    types::CtxValueKind::Negation(vals) => vals.iter().collect(),
                })
                .collect::<Vec<&dir::DirValue>>();
            assert_eq!(
                values,
                expected_contexts[expected_idx]
                    .iter()
                    .collect::<Vec<&dir::DirValue>>()
            );
            expected_idx += 1;
            state_machine
                .advance(&mut ctx)
                .expect("State Machine advance");
        }

        assert_eq!(expected_idx, 14);

        let mut ctx_manager = AnalysisContextManager::new(&lowered, &selection_data);
        expected_idx = 0;
        while let Some(ctx) = ctx_manager.advance().expect("Context Manager Context") {
            let values = ctx
                .iter()
                .flat_map(|c| match c.value {
                    types::CtxValueKind::Assertion(val) => vec![val],
                    types::CtxValueKind::Negation(vals) => vals.iter().collect(),
                })
                .collect::<Vec<&dir::DirValue>>();
            assert_eq!(
                values,
                expected_contexts[expected_idx]
                    .iter()
                    .collect::<Vec<&dir::DirValue>>()
            );
            expected_idx += 1;
        }

        assert_eq!(expected_idx, 14);
    }
}
