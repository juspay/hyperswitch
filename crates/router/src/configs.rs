use hyperswitch_interfaces::secrets_interface::secret_state::RawSecret;

pub(crate) mod defaults;
pub mod secrets_transformers;
pub mod settings;
mod validations;

pub type Settings = settings::Settings<RawSecret>;
