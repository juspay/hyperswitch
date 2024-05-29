use diesel::{Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::schema::connector_ps_identifiers;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = connector_ps_identifiers)]
#[diesel(primary_key(merchant_id, mca_id, connect_account_id, customer_ps_id, pm_ps_id))]
pub struct ConnectorPsIdentifiers {
    pub id: String,
    pub merchant_id: String,
    pub mca_id: String,
    pub connect_account_id: String,
    pub customer_id: String,
    pub pm_id: String,
    pub customer_ps_id: Option<String>,
    pub pm_ps_id: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
}

#[derive(
    Clone,
    Debug,
    Default,
    Eq,
    PartialEq,
    Insertable,
    Serialize,
    Deserialize,
    router_derive::DebugAsDisplay,
    router_derive::Setter,
)]
#[diesel(table_name = connector_ps_identifiers)]
pub struct ConnectorPsIdentifiersNew {
    pub merchant_id: String,
    pub mca_id: String,
    pub connect_account_id: String,
    pub customer_id: String,
    pub pm_id: String,
    pub customer_ps_id: Option<String>,
    pub pm_ps_id: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub modified_at: Option<PrimitiveDateTime>,
}

pub enum ConnectorPsIdentifiersUpdate {
    ConnectorPsCustomerUpdate {
        customer_ps_id: String,
    },
    ConnectorPsPaymentMethodUpdate {
        pm_ps_id: String,
    },
    ConnectorPsUpdate {
        customer_ps_id: String,
        pm_ps_id: String,
    },
}

#[derive(Clone, Debug, router_derive::DebugAsDisplay)]
#[diesel(table_name = connector_ps_identifiers)]
pub struct ConnectorPsIdentifierUpdateInternal {
    pub id: Option<String>,
    pub merchant_id: Option<String>,
    pub mca_id: Option<String>,
    pub connect_account_id: Option<String>,
    pub customer_id: Option<String>,
    pub pm_id: Option<String>,
    pub customer_ps_id: Option<String>,
    pub pm_ps_id: Option<String>,
    pub created_at: Option<PrimitiveDateTime>,
    pub modified_at: PrimitiveDateTime,
}

impl Default for ConnectorPsIdentifierUpdateInternal {
    fn default() -> Self {
        Self {
            id: None,
            merchant_id: None,
            mca_id: None,
            connect_account_id: None,
            customer_id: None,
            pm_id: None,
            customer_ps_id: None,
            pm_ps_id: None,
            created_at: None,
            modified_at: common_utils::date_time::now(),
        }
    }
}

impl From<ConnectorPsIdentifiersUpdate> for ConnectorPsIdentifierUpdateInternal {
    fn from(connector_ps_identifier_update: ConnectorPsIdentifiersUpdate) -> Self {
        match connector_ps_identifier_update {
            ConnectorPsIdentifiersUpdate::ConnectorPsCustomerUpdate { customer_ps_id } => Self {
                customer_ps_id: Some(customer_ps_id),
                ..Default::default()
            },
            ConnectorPsIdentifiersUpdate::ConnectorPsPaymentMethodUpdate { pm_ps_id } => Self {
                pm_ps_id: Some(pm_ps_id),
                ..Default::default()
            },
            ConnectorPsIdentifiersUpdate::ConnectorPsUpdate {
                customer_ps_id,
                pm_ps_id,
            } => Self {
                customer_ps_id: Some(customer_ps_id),
                pm_ps_id: Some(pm_ps_id),
                ..Default::default()
            },
        }
    }
}
