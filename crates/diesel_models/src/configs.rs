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

#[derive(Default, Clone, Debug, Identifiable, Queryable, Deserialize, Serialize)]
#[diesel(table_name = configs)]

pub struct Config {
    #[serde(skip)]
    pub id: i32,
    pub key: String,
    pub config: String,
}

#[derive(Debug)]
pub enum ConfigUpdate {
    Update { config: Option<String> },
}

#[derive(Clone, Debug, AsChangeset, Default)]
#[diesel(table_name = configs)]
pub struct ConfigUpdateInternal {
    config: Option<String>,
}

impl ConfigUpdateInternal {
        /// Creates a new configuration by copying the fields from the provided `source` configuration.
    ///
    /// # Arguments
    ///
    /// * `source` - The source `Config` from which to copy the fields.
    ///
    /// # Returns
    ///
    /// A new `Config` with fields copied from the `source` configuration.
    ///
    pub fn create_config(self, source: Config) -> Config {
        Config { ..source }
    }
}

impl From<ConfigUpdate> for ConfigUpdateInternal {
        /// This method takes a ConfigUpdate enum and extracts the config value from it, then constructs a new instance of Self with the extracted config value. 
    fn from(config_update: ConfigUpdate) -> Self {
        match config_update {
            ConfigUpdate::Update { config } => Self { config },
        }
    }
}
