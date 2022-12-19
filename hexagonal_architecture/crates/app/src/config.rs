use envconfig::Envconfig;

/// See: https://chromium.googlesource.com/chromium/src/+/HEAD/docs/patterns/passkey.md
#[derive(Debug)]
pub struct DotenvProof(());

/// It loads the .env file located in the environment's current directory or its parents in sequence.
pub fn dotenv_with_proof() -> Result<DotenvProof, dotenv::Error> {
    dotenv::dotenv().map(|_| DotenvProof(()))
}

#[derive(Envconfig, Debug)]
/// Config is a type of structure that is used to store information.
/// It is typically used to store settings or preferences for a program or application.
/// In this case, it is used to store the application URL and the database connection URL.
pub struct Config {
    #[envconfig(from = "APPLICATION_URL")]
    /// application_url is a variable that stores the URL of an application.
    pub application_url: String,
    #[envconfig(from = "DB_CONNECTION")]
    /// db_connection is a variable that stores a URL that is used to connect to a database.
    /// The URL is in the format of "protocol//[hosts][/database][?properties]".
    pub db_connection: String,
}

impl Config {
    /// Initialize structure from environment variables.
    pub fn new(_: &DotenvProof) -> Result<Self, envconfig::Error> {
        Self::init_from_env()
    }
}
