#[cfg(feature = "diesel")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

#[cfg(feature = "diesel")]
use crate::schema::mandate;
// use serde::{Deserialize, Serialize};
use crate::{
    pii::{self, Secret},
    types::enums,
};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = mandate))]
pub struct Mandate {
    pub id: i32,
    pub mandate_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub mandate_status: enums::MandateStatus,
    pub mandate_type: enums::MandateType,
    pub customer_accepted_at: Option<PrimitiveDateTime>,
    pub customer_ip_address: Option<Secret<String, pii::IpAddress>>,
    pub customer_user_agent: Option<String>,
    pub network_transaction_id: Option<String>,
    pub previous_transaction_id: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = mandate))]
pub struct MandateNew {
    pub mandate_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub mandate_status: enums::MandateStatus,
    pub mandate_type: enums::MandateType,
    pub customer_accepted_at: Option<PrimitiveDateTime>,
    pub customer_ip_address: Option<Secret<String, pii::IpAddress>>,
    pub customer_user_agent: Option<String>,
    pub network_transaction_id: Option<String>,
    pub previous_transaction_id: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
}

#[derive(Debug)]
pub enum MandateUpdate {
    StatusUpdate {
        mandate_status: enums::MandateStatus,
    },
}

#[derive(Clone, Debug, Default, router_derive::DebugAsDisplay)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = mandate))]
#[allow(dead_code)]
pub(super) struct MandateUpdateInternal {
    mandate_status: enums::MandateStatus,
}

impl From<MandateUpdate> for MandateUpdateInternal {
    fn from(mandate_update: MandateUpdate) -> Self {
        match mandate_update {
            MandateUpdate::StatusUpdate { mandate_status } => Self { mandate_status },
        }
    }
}
