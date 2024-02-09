use std::convert::From;

use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::configs;

#[derive(Default, Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = configs)]

pub struct ConfigNew {
    pub key: String,
    pub config: Vec<u8>,
}

#[derive(Default, Clone, Debug, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = configs)]

pub struct Config {
    #[serde(skip)]
    pub id: i32,
    pub key: String,
    pub config: Vec<u8>,
}

#[derive(Debug)]
pub enum ConfigUpdate {
    Update { config: Option<Vec<u8>> },
}

#[derive(Clone, Debug, AsChangeset, Default)]
#[diesel(table_name = configs)]
pub struct ConfigUpdateInternal {
    config: Option<Vec<u8>>,
}

impl ConfigUpdateInternal {
    pub fn create_config(self, source: Config) -> Config {
        Config { ..source }
    }
}

impl From<ConfigUpdate> for ConfigUpdateInternal {
    fn from(config_update: ConfigUpdate) -> Self {
        match config_update {
            ConfigUpdate::Update { config } => Self { config },
        }
    }
}
