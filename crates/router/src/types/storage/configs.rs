use std::convert::From;

#[cfg(feature = "diesel")]
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[cfg(feature = "diesel")]
use crate::schema::configs;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "diesel", derive(Insertable))]
#[cfg_attr(feature = "diesel", diesel(table_name = configs))]

pub struct ConfigNew {
    pub key: String,
    pub config: String,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "diesel", derive(Identifiable, Queryable))]
#[cfg_attr(feature = "diesel", diesel(table_name = configs))]

pub struct Config {
    #[serde(skip_serializing)]
    pub id: i32,
    pub key: String,
    pub config: String,
}

#[derive(Debug)]
pub enum ConfigUpdate {
    Update { config: Option<String> },
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "diesel", derive(AsChangeset))]
#[cfg_attr(feature = "diesel", diesel(table_name = configs))]
#[allow(dead_code)]
pub(super) struct ConfigUpdateInternal {
    config: Option<String>,
}

impl From<ConfigUpdate> for ConfigUpdateInternal {
    fn from(config_update: ConfigUpdate) -> Self {
        match config_update {
            ConfigUpdate::Update { config } => Self { config },
        }
    }
}
