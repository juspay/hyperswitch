// Add enum variants as necessary
#[derive(Debug)]
pub enum WorkflowState {
    DummyWorkflowState(DummyWorkflowState),
}

// Rename struct as necessary, typically based on runner/workflow
// Add fields as necessary
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct DummyWorkflowState {
    order_id: Option<String>,
    merchant_id: Option<String>,
    acquired_locks: Vec<String>,
    flow_name: Option<String>,
}

impl DummyWorkflowState {
    // Implement methods as required for each struct
}
