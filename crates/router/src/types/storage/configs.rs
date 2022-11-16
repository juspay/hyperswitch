use std::convert::From;

use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::configs;

#[derive(Default, Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = configs)]
pub struct ConfigNew {
    pub key: String,
    pub config: String,
}

#[derive(Default, Clone, Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(table_name = configs)]
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

#[derive(Clone, Debug, Default, AsChangeset)]
#[diesel(table_name = configs)]
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
