use common_utils::impl_to_sql_from_sql_json;
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use euclid::frontend::dir::{DirKeyKind, EuclidDirFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Enum representing the possible outcomes of the 3DS Decision Rule Engine.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromSqlRow,
    AsExpression,
    ToSchema,
    Default,
)]
#[diesel(sql_type = Jsonb)]
#[serde(rename_all = "snake_case")]
pub enum ThreeDSDecision {
    /// No 3DS authentication required
    #[default]
    NoThreeDs,
    /// Mandate 3DS Challenge
    ChallengeRequested,
    /// Prefer 3DS Challenge
    ChallengePreferred,
    /// Request 3DS Exemption, Type: Transaction Risk Analysis (TRA)
    ThreeDsExemptionRequestedTra,
    /// Request 3DS Exemption, Type: Low Value Transaction
    ThreeDsExemptionRequestedLowValue,
    /// No challenge requested by merchant (e.g., delegated authentication)
    IssuerThreeDsExemptionRequested,
}
impl_to_sql_from_sql_json!(ThreeDSDecision);

impl ThreeDSDecision {
    /// Checks if the decision is to mandate a 3DS challenge
    pub fn should_force_3ds_challenge(self) -> bool {
        matches!(self, Self::ChallengeRequested)
    }
}

/// Struct representing the output configuration for the 3DS Decision Rule Engine.
#[derive(
    Serialize, Default, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct ThreeDSDecisionRule {
    /// The decided 3DS action based on the rules
    pub decision: ThreeDSDecision,
}

impl ThreeDSDecisionRule {
    /// Returns the decision
    pub fn get_decision(&self) -> ThreeDSDecision {
        self.decision
    }
}

impl_to_sql_from_sql_json!(ThreeDSDecisionRule);

impl EuclidDirFilter for ThreeDSDecisionRule {
    const ALLOWED: &'static [DirKeyKind] = &[
        DirKeyKind::CardNetwork,
        DirKeyKind::PaymentAmount,
        DirKeyKind::PaymentCurrency,
        DirKeyKind::IssuerName,
        DirKeyKind::IssuerCountry,
        DirKeyKind::CustomerDevicePlatform,
        DirKeyKind::CustomerDeviceType,
        DirKeyKind::CustomerDeviceDisplaySize,
        DirKeyKind::AcquirerCountry,
        DirKeyKind::AcquirerFraudRate,
    ];
}
