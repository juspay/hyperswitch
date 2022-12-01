use std::fmt;
use serde::Deserialize;
use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
  pub server: Server,
  pub env: ENV,
  pub database: Database,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
  pub port: u16,
  pub host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
  pub username: String,
  pub password: String, 
  pub host: String, 
  pub port: u16,
  pub dbname: String, 
}

const CONFIG_FILE_PATH: &str = "./src/configs/Default.toml";
const CONFIG_FILE_PREFIX: &str = "./src/configs/";

#[derive(Debug, Deserialize, Clone)]
pub enum ENV {
  Development,
  Sandbox,
  Production,
}

impl fmt::Display for ENV {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ENV::Development => write!(f, "Development"),
      ENV::Sandbox  => write!(f, "Sandbox"),
      ENV::Production => write!(f, "Production"),
    }
  }
}

impl From<&str> for ENV {
  fn from(env: &str) -> Self {
    match env {
      "Sandbox"    => ENV::Sandbox,
      "Production" => ENV::Production,
      _            => ENV::Development,
    }
  }
}

impl Settings {
  pub fn new() -> Result<Self, ConfigError> {
    let env = std::env::var("RUN_ENV").unwrap_or_else(|_| "Development".into());
    let s = Config::builder()
      .set_default("env", env.clone())?
      .add_source(File::with_name(CONFIG_FILE_PATH))
      .add_source(File::with_name(&format!("{}{}", CONFIG_FILE_PREFIX, env)))
      .build()?;
    s.try_deserialize()
  }
}
