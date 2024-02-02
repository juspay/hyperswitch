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
        /// Checks if the comparison logic is finished processing all the values.
    /// Returns true if the count plus one is greater than or equal to the length of values, or if the comparison logic is set to NegativeConjunction, otherwise returns false.
    fn is_finished(&self) -> bool {
        self.count + 1 >= self.values.len()
            || matches!(self.logic, dir::DirComparisonLogic::NegativeConjunction)
    }

    #[inline]
        /// Advances the count of the current logic if the comparison logic is PositiveDisjunction,
    /// wrapping around to 0 if the count exceeds the length of the values.
    fn advance(&mut self) {
        if let dir::DirComparisonLogic::PositiveDisjunction = self.logic {
            self.count = (self.count + 1) % self.values.len();
        }
    }

    #[inline]
        /// Resets the count to 0.
    fn reset(&mut self) {
        self.count = 0;
    }

    #[inline]
        /// This method takes a mutable reference to a ConjunctiveContext and inserts a ContextValue into it based on the logic and values of the ComparisonStateMachine. If the logic is PositiveDisjunction, it inserts an assertion ContextValue into the context at the specified index using the values and metadata of the ComparisonStateMachine. Returns a Result indicating success or a StateMachineError if there are index out of bounds errors.
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
        /// This method pushes a context value onto the given ConjunctiveContext based on the logic of the ComparisonStateMachine. If the logic is PositiveDisjunction, it asserts a value from the ComparisonStateMachine's values at a given index along with its metadata. If the logic is NegativeConjunction, it negates all the values in the ComparisonStateMachine along with their metadata and pushes the resulting context value onto the ConjunctiveContext.
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
        /// Creates a new ComparisonStateCollection with the given conditions and start index.
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

        /// Initializes the state machines with the given context. It pushes the context into each state machine.
    /// 
    /// # Arguments
    /// 
    /// * `context` - A mutable reference to the `ConjunctiveContext` to be pushed into the state machines.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), StateMachineError>` - A result indicating success or an error if pushing the context into any state machine fails.
    /// 
    fn init(&self, context: &mut types::ConjunctiveContext<'a>) -> Result<(), StateMachineError> {
        for machine in &self.state_machines {
            machine.push(context)?;
        }
        Ok(())
    }

    #[inline]
        /// Truncates the given conjunctive context by removing all elements starting from the specified index.
    fn destroy(&self, context: &mut types::ConjunctiveContext<'a>) {
        context.truncate(self.start_ctx_idx);
    }

    #[inline]
        /// Checks if all state machines in the list have finished their execution.
    /// Returns true if all state machines have finished, false otherwise.
    fn is_finished(&self) -> bool {
        !self
            .state_machines
            .iter()
            .any(|machine| !machine.is_finished())
    }

    #[inline]
        /// Returns the index of the next context by adding the starting context index to the number of state machines currently in the context.
    fn get_next_ctx_idx(&self) -> usize {
        self.start_ctx_idx + self.state_machines.len()
    }

        /// Advance each state machine in reverse order. If a state machine is finished, reset it, put the conjunctive context into it, and move on to the next state machine. If a state machine is not finished, advance it, put the conjunctive context into it, and stop the iteration.
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
        /// Creates a new instance of `Self` (the current struct) with the given `stmt` and `ctx_start_idx`.
    ///
    /// This method initializes the condition state machine with the condition of the `stmt`, and creates a vector of nested `DirIfStatement` references from the `nested` field of the `stmt`.
    ///
    /// # Arguments
    ///
    /// * `stmt` - A reference to a `DirIfStatement` which is used to initialize the condition state machine and to create the vector of nested statements.
    /// * `ctx_start_idx` - An index used for context within the condition state machine.
    ///
    /// # Returns
    ///
    /// A new instance of `Self` with the initialized condition state machine, nested statement vector, and nested index set to 0.
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

        /// Initializes the current state machine with the given context, and returns the result as a Result.
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
        /// Checks if the current nested index plus one is greater than or equal to the length of the nested array.
    fn is_finished(&self) -> bool {
        self.nested_idx + 1 >= self.nested.len()
    }

    #[inline]
        /// Checks if the condition machine is finished or not.
    /// 
    /// Returns true if the condition machine is finished, otherwise returns false.
    fn is_condition_machine_finished(&self) -> bool {
        self.condition_machine.is_finished()
    }

    #[inline]
        /// Destroys the current object by calling the destroy method on the condition machine
    fn destroy(&self, context: &mut types::ConjunctiveContext<'a>) {
        self.condition_machine.destroy(context);
    }

    #[inline]
        /// Advances the condition machine using the provided conjunctive context.
    /// 
    /// # Arguments
    /// 
    /// * `context` - A mutable reference to a conjunctive context.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), StateMachineError>` - A result indicating success or a state machine error.
    /// 
    fn advance_condition_machine(
        &mut self,
        context: &mut types::ConjunctiveContext<'a>,
    ) -> Result<(), StateMachineError> {
        self.condition_machine.advance(context)?;
        Ok(())
    }

        /// Advances the state machine to the next nested state, if available, and returns it as an Option.
    ///
    /// # Returns
    /// - Ok(None) if the nested state is empty
    /// - Ok(Some(StateMachine)) containing the next nested state if available
    /// - Err(StateMachineError) if there is an index out of bounds error while getting the next nested state
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
        /// Creates a new instance of the struct, initializing it with the provided `rule` and `connector_selection_data`.
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

        /// Checks if the current state of the machine is finished. Returns true if there are no if statement machines
    /// left to be executed and the running stack is empty, otherwise returns false.
    fn is_finished(&self) -> bool {
        self.if_stmt_machines.is_empty() && self.running_stack.is_empty()
    }

        /// Initializes the next state machine in the running stack, adding connectors to the context if they have not already been added. 
    /// 
    /// # Arguments
    /// 
    /// * `context` - A mutable reference to the conjunctive context.
    /// 
    /// # Returns
    /// 
    /// * `Result<(), StateMachineError>` - A result indicating success or an error if the initialization fails.
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

        /// Advances the state machine by executing the next step in the running stack. 
        /// If the condition machines for the current statement are not finished, it advances 
        /// the condition machine. If all condition machines are finished, it advances the 
        /// next statement in the running stack. If there are nested running statements, it 
        /// initializes and pushes them onto the running stack. If there are no more statements 
        /// to run, it initializes the next statement. 
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
        /// Constructs a new instance of Self with the given dir rule and connector selection data.
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

        /// Advances the state machine and returns the next conjunctive context if the state machine is not finished,
    /// Otherwise, returns None. If this is the first call to advance, it initializes the state machine and returns the initial conjunctive context.
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

        /// Advances the state machine to the next step and returns a mutable reference to the conjunctive context if the state machine is not finished.
    /// If the state machine is finished, it returns None.
    ///
    /// # Errors
    /// Returns a StateMachineError if the state machine encounters an error during advancement.
    /// 
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
    }}

#[derive(Debug)]
pub struct ProgramStateMachine<'a> {
    rule_machines: Vec<RuleStateMachine<'a>>,
    current_rule_machine: Option<RuleStateMachine<'a>>,
    is_init: bool,
}

impl<'a> ProgramStateMachine<'a> {
        /// Creates a new instance of Self using the given DirProgram and connector_selection_data.
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

        /// Checks if the current rule machine is finished and if there are no more rule machines in the queue.
    pub fn is_finished(&self) -> bool {
        self.current_rule_machine
            .as_ref()
            .map_or(true, |rsm| rsm.is_finished())
            && self.rule_machines.is_empty()
    }

        /// Initializes the state machine with the provided conjunctive context. If the state machine has not been initialized before, it calls the `init_next` method on the current rule machine with the given context. Once initialized, the `is_init` flag is set to true.
    ///
    /// # Arguments
    ///
    /// * `context` - A mutable reference to the conjunctive context to be used for initialization.
    ///
    /// # Returns
    ///
    /// * `Result<(), StateMachineError>` - A result indicating success or an error of type `StateMachineError`.
    ///
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

        /// Advances the state machine by either initializing the next rule machine if the current one is finished,
    /// or by advancing the current rule machine if it is not finished. It also clears the context if a new rule machine
    /// is initialized.
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
        /// Creates a new instance of Self with the given DirProgram and connector selection data.
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

        /// Advances the state machine and returns the next conjunctive context if available
    /// 
    /// # Returns
    /// - `Ok(Some(&types::ConjunctiveContext<'a>))` if the state machine has not finished and the next conjunctive context is available
    /// - `Ok(None)` if the state machine has finished
    /// - `Err(StateMachineError)` if an error occurs during the state machine advancement
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

/// Generates a selection data for connectors in the given DirProgram, where the connectors are of type EuclidAnalysable.
/// 
/// # Arguments
/// 
/// * `program` - A reference to a DirProgram containing connectors of type O that implement EuclidAnalysable.
/// 
/// # Returns
/// 
/// A vector of vectors, where each inner vector contains tuples of DirValue and Metadata representing the selection data for the connectors in the program.
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
        /// This method is used to test correct contexts by initializing a state machine and a context manager
    /// and then comparing the expected contexts with the actual contexts derived by the state machine and the context manager.
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
                expected_contexts
                    .get(expected_idx)
                    .expect("Error deriving contexts")
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
                expected_contexts
                    .get(expected_idx)
                    .expect("Error deriving contexts")
                    .iter()
                    .collect::<Vec<&dir::DirValue>>()
            );
            expected_idx += 1;
        }

        assert_eq!(expected_idx, 14);
    }
}
