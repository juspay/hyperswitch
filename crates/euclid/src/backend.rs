pub mod inputs;
pub mod interpreter;
#[cfg(feature = "valued_jit")]
pub mod vir_interpreter;

pub use inputs::BackendInput;
pub use interpreter::InterpreterBackend;
#[cfg(feature = "valued_jit")]
pub use vir_interpreter::VirInterpreterBackend;

use crate::frontend::ast;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackendOutput<O> {
    pub rule_name: Option<String>,
    pub connector_selection: O,
}

pub trait EuclidBackend<O>: Sized {
    type Error: serde::Serialize;

    fn with_program(program: ast::Program<O>) -> Result<Self, Self::Error>;

    fn execute(&self, input: BackendInput) -> Result<BackendOutput<O>, Self::Error>;
}
