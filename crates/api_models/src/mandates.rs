use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::{enums as api_enums, payments};

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MandateId {
    pub mandate_id: String,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema)]
pub struct MandateRevokedResponse {
    /// The identifier for mandate
    pub mandate_id: String,
    /// The status for mandates
    #[schema(value_type = MandateStatus)]
    pub status: api_enums::MandateStatus,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct MandateResponse {
    /// The identifier for mandate
    pub mandate_id: String,
    /// The status for mandates
    #[schema(value_type = MandateStatus)]
    pub status: api_enums::MandateStatus,
    /// The identifier for payment method
    pub payment_method_id: String,
    /// The payment method
    pub payment_method: String,
    /// The card details for mandate
    pub card: Option<MandateCardDetails>,
    /// Details about the customer’s acceptance
    #[schema(value_type = Option<CustomerAcceptance>)]
    pub customer_acceptance: Option<payments::CustomerAcceptance>,
}

#[derive(Default, Debug, Deserialize, Serialize, ToSchema, Clone)]
pub struct MandateCardDetails {
    /// The last 4 digits of card
    pub last4_digits: Option<String>,
    /// The expiry month of card
    #[schema(value_type = Option<String>)]
    pub card_exp_month: Option<Secret<String>>,
    /// The expiry year of card
    #[schema(value_type = Option<String>)]
    pub card_exp_year: Option<Secret<String>>,
    /// The card holder name
    #[schema(value_type = Option<String>)]
    pub card_holder_name: Option<Secret<String>>,
    /// The token from card locker
    #[schema(value_type = Option<String>)]
    pub card_token: Option<Secret<String>>,
    /// The card scheme network for the particular card
    pub scheme: Option<String>,
    /// The country code in in which the card was issued
    pub issuer_country: Option<String>,
    #[schema(value_type = Option<String>)]
    /// A unique identifier alias to identify a particular card
    pub card_fingerprint: Option<Secret<String>>,
}

#[derive(Clone, Debug, Deserialize, ToSchema, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MandateListConstraints {
    /// limit on the number of objects to return
    pub limit: Option<i64>,
    /// status of the mandate
    pub mandate_status: Option<api_enums::MandateStatus>,
    /// connector linked to mandate
    pub connector: Option<String>,
    /// The time at which mandate is created
    #[schema(example = "2022-09-10T10:11:12Z")]
    pub created_time: Option<PrimitiveDateTime>,
    /// Time less than the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.lt")]
    pub created_time_lt: Option<PrimitiveDateTime>,
    /// Time greater than the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.gt")]
    pub created_time_gt: Option<PrimitiveDateTime>,
    /// Time less than or equals to the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.lte")]
    pub created_time_lte: Option<PrimitiveDateTime>,
    /// Time greater than or equals to the mandate created time
    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(rename = "created_time.gte")]
    pub created_time_gte: Option<PrimitiveDateTime>,
}
