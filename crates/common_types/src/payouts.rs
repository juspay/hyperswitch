//! Payout related types.

#[cfg(feature = "diesel")]
use common_utils::impl_to_sql_from_sql_json;
#[cfg(feature = "diesel")]
use diesel::{sql_types::Jsonb, AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Billing descriptor information for a payout.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub struct PayoutsBillingDescriptor {
    /// Reference displayed on the beneficiary's bank statement.
    pub reference: Option<String>,
    /// Statement descriptor displayed for the payout.
    pub statement_descriptor: Option<String>,
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(PayoutsBillingDescriptor);

/// Connector-specific metadata supplied with a payout request.
///
/// Each connector owns its nested contract so connector wire semantics do not leak into shared
/// payout fields.
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = Jsonb))]
pub struct ConnectorPayoutMetadata {
    /// Checkout.com-specific payout metadata.
    pub checkout: Option<CheckoutPayoutMetadata>,
}

#[cfg(feature = "diesel")]
impl_to_sql_from_sql_json!(ConnectorPayoutMetadata);

/// Checkout.com-specific metadata for card payouts.
#[derive(Default, Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckoutPayoutMetadata {
    /// Funds Transfer Type assigned to the merchant by Checkout.com.
    /// Do not select or default this value independently. When omitted, Checkout receives `null`.
    pub funds_transfer_type: Option<CheckoutPayoutFundsTransferType>,
}

/// Checkout.com Funds Transfer Type values for Visa Direct and Mastercard Send payouts.
/// The configured value must be the one assigned to the merchant by Checkout.com.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutPayoutFundsTransferType {
    /// Visa Direct account-to-account transfer.
    AccountToAccount,
    /// Visa Direct funds transfer.
    FundsTransfer,
    /// Visa Direct cross-border money transfer.
    CrossBorderMoneyTransfer,
    /// Visa Direct peer-to-peer transfer.
    PeerToPeer,
    /// Visa Direct staged digital-wallet transfer.
    StagedDigitalWalletTransfer,
    /// Visa Direct funds disbursement.
    FundsDisbursement,
    /// Visa Direct loyalty payment.
    LoyaltyPayments,
    /// Visa Direct payroll or pension disbursement.
    PayrollAndPensionDisbursements,
    /// Visa Direct online-gambling transfer.
    OnlineGambling,
    /// Mastercard Send person-to-person transfer.
    MastercardPersonToPerson,
    /// Mastercard Send transfer to the recipient's own account.
    MastercardTransferToOwnAccount,
    /// Mastercard Send business disbursement.
    MastercardBusinessDisbursement,
    /// Mastercard Send business-to-business transfer.
    MastercardBusinessToBusiness,
    /// Mastercard Send gaming repayment.
    MastercardGamingRepay,
}
