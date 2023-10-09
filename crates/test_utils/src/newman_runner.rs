use std::{env, process::Command};

use clap::{arg, command, Parser};
use masking::PeekInterface;

use crate::connector_auth::{ConnectorAuthType, ConnectorAuthenticationMap};

#[derive(Parser)]
#[command(version, about = "Postman collection runner using newman!", long_about = None)]
struct Args {
    /// Admin API Key of the environment
    #[arg(short, long = "admin_api_key")]
    admin_api_key: String,
    /// Base URL of the Hyperswitch environment
    #[arg(short, long = "base_url")]
    base_url: String,
    /// Name of the connector
    #[arg(short, long = "connector_name")]
    connector_name: String,
    /// Folder name of specific tests
    #[arg(short, long = "folder")]
    folder_s: Option<String>,
    /// Optional Verbose logs
    #[arg(short, long)]
    verbose: bool,
}

// Just by the name of the connector, this function generates the name of the collection dir
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/collection-dir/stripe
#[inline]
fn get_path(name: impl AsRef<str>) -> String {
    format!("postman/collection-dir/{}", name.as_ref())
}

pub fn command_generate() -> Command {
    let args = Args::parse();

    let connector_name = args.connector_name;
    let base_url = args.base_url;
    let admin_api_key = args.admin_api_key;

    let collection_path = get_path(&connector_name);
    let auth_map = ConnectorAuthenticationMap::new();

    let inner_map = auth_map.inner();

    // Newman runner
    // Depending on the conditions satisfied, variables are added. Since certificates of stripe have already
    // been added to the postman collection, those conditions are set to true and collections that have
    // variables set up for certificate, will consider those variables and will fail.

    let mut newman_command = Command::new("newman");
    newman_command.args(["dir-run", &collection_path]);
    newman_command.args(["--env-var", &format!("admin_api_key={admin_api_key}")]);
    newman_command.args(["--env-var", &format!("baseUrl={base_url}")]);

    if let Some(auth_type) = inner_map.get(&connector_name) {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                newman_command.args([
                    "--env-var",
                    &format!("connector_api_key={}", api_key.peek()),
                ]);
            }
            ConnectorAuthType::BodyKey { api_key, key1 } => {
                newman_command.args([
                    "--env-var",
                    &format!("connector_api_key={}", api_key.peek()),
                    "--env-var",
                    &format!("connector_key1={}", key1.peek()),
                ]);
            }
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                newman_command.args([
                    "--env-var",
                    &format!("connector_api_key={}", api_key.peek()),
                    "--env-var",
                    &format!("connector_key1={}", key1.peek()),
                    "--env-var",
                    &format!("connector_api_secret={}", api_secret.peek()),
                ]);
            }
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                key2,
                api_secret,
            } => {
                newman_command.args([
                    "--env-var",
                    &format!("connector_api_key={}", api_key.peek()),
                    "--env-var",
                    &format!("connector_key1={}", key1.peek()),
                    "--env-var",
                    &format!("connector_key2={}", key2.peek()),
                    "--env-var",
                    &format!("connector_api_secret={}", api_secret.peek()),
                ]);
            }
            // Handle other ConnectorAuthType variants
            _ => {
                eprintln!("Invalid authentication type.");
            }
        }
    } else {
        eprintln!("Connector not found.");
    }

    // Add additional environment variables if present
    if let Ok(gateway_merchant_id) = env::var("GATEWAY_MERCHANT_ID") {
        newman_command.args([
            "--env-var",
            &format!("gateway_merchant_id={gateway_merchant_id}"),
        ]);
    }

    if let Ok(gpay_certificate) = env::var("GPAY_CERTIFICATE") {
        newman_command.args(["--env-var", &format!("certificate={gpay_certificate}")]);
    }

    if let Ok(gpay_certificate_keys) = env::var("GPAY_CERTIFICATE_KEYS") {
        newman_command.args([
            "--env-var",
            &format!("certificate_keys={gpay_certificate_keys}"),
        ]);
    }

    newman_command.arg("--delay-request").arg("7"); // 7 milli seconds delay

    newman_command.arg("--color").arg("on");

    // Add flags for running specific folders
    if let Some(folders) = &args.folder_s {
        let folder_names: Vec<String> = folders.split(',').map(|s| s.trim().to_string()).collect();

        for folder_name in folder_names {
            if !folder_name.contains("QuickStart") {
                // This is a quick fix, "QuickStart" is intentional to have merchant account and API keys set up
                // This will be replaced by a more robust and efficient account creation or reuse existing old account
                newman_command.args(["--folder", "QuickStart"]);
            }
            newman_command.args(["--folder", &folder_name]);
        }
    }

    if args.verbose {
        newman_command.arg("--verbose");
    }

    newman_command
}
