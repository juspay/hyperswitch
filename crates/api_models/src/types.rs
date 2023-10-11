use std::collections::HashMap;

use serde::Deserialize;
use serde_with::serde_as;

use crate::payment_methods::SurchargeDetailsResponse;
// this type will be serialized to json_value and stored in PaymentAttempt.surcharge_metadata
#[serde_as]
#[derive(Clone, Debug, PartialEq, serde::Serialize, Deserialize)]
pub struct SurchargeMetadata {
    #[serde_as(as = "HashMap<_, _>")]
    pub surcharge_results: HashMap<String, SurchargeDetailsResponse>,
}
