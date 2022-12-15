use envconfig::Envconfig;

/// See: https://chromium.googlesource.com/chromium/src/+/HEAD/docs/patterns/passkey.md
pub struct DotenvProof(());

pub fn dotenv_with_proof() -> Result<DotenvProof, dotenv::Error> {
    dotenv::dotenv().map(|_| DotenvProof(()))
}

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "APPLICATION_URL")]
    pub application_url: String,
    #[envconfig(from = "DB_CONNECTION")]
    pub db_connection: String,
}

impl Config {
    pub fn new(_: &DotenvProof) -> Result<Self, envconfig::Error> {
        Self::init_from_env()
    }
}
