#[cfg(feature = "v2")]
use common_enums::enums::PaymentConnectorTransmission;
#[cfg(feature = "v2")]
use common_utils::id_type;
use common_utils::{hashing::HashedString, pii, types::MinorUnit};
use diesel::{
    sql_types::{Json, Jsonb},
    AsExpression, FromSqlRow,
};
use masking::{Secret, WithType};
use serde::{self, Deserialize, Serialize};

#[cfg(feature = "v1")]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, FromSqlRow, AsExpression)]
#[diesel(sql_type = Json)]
pub struct FeatureMetadata {
    /// Redirection response coming in request as metadata field only for redirection scenarios
    pub redirect_response: Option<RedirectResponse>,
    /// Additional tags to be used for global search
    pub search_tags: Option<Vec<HashedString<WithType>>>,
    /// Recurring payment details required for apple pay Merchant Token
    pub apple_pay_recurring_details: Option<ApplePayRecurringDetails>,
}
