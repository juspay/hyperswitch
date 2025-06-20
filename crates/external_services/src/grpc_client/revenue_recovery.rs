#![cfg(all(feature = "revenue_recovery", feature = "v2"))]

/// Module declarations
pub mod recovery_trainer_client;

/// Recovery Trainer items
pub use recovery_trainer_client::{
    GetTrainingJobStatusRequest, GetTrainingJobStatusResponse, JobStatus, TrainerClientConfig,
    TrainerClientInterface, TrainerError, TrainerResult, TriggerTrainingRequest,
    TriggerTrainingResponse,
};
