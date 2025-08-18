use common_utils::id_type;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::enums;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionWorkflowTrackingData {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub payment_method_id: String,
    pub subscription_id: Option<String>,
    pub invoice_id: String,
    pub amount: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
    pub customer_id: Option<id_type::CustomerId>,
    pub connector_name: String,
}
