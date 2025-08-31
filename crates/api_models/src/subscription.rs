use std::fmt::Debug;

use common_utils::events::ApiEventMetric;
// use common_utils::{
//     errors::{ParsingError, ValidationError},
//     ext_traits::ValueExt,
// };
use utoipa::ToSchema;

use crate::payments::CustomerDetails;

pub const SUBSCRIPTION_ID_PREFIX: &str = "sub";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateSubscriptionRequest {
    pub plan_id: Option<String>,
    pub coupon_code: Option<String>,
    pub mca_id: Option<String>,
    pub confirm: bool,
    pub customer_id: Option<common_utils::id_type::CustomerId>,
    pub customer: Option<CustomerDetails>,
}

impl CreateSubscriptionRequest {
    pub fn get_customer_id(&self) -> Option<&common_utils::id_type::CustomerId> {
        self.customer_id
            .as_ref()
            .or(self.customer.as_ref().map(|customer| &customer.id))
    }
}

impl ApiEventMetric for CreateSubscriptionRequest {}
