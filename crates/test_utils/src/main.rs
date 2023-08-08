use std::{
    env,
    process::{exit, Command as cmd},
};

use clap::{arg, command, Parser};
use masking::PeekInterface;
use test_utils::connector_auth::{ConnectorAuthType, ConnectorAuthenticationMap};

// Just by the name of the connector, this function generates the name of the collection
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/stripe.postman_collection.json
#[inline]
fn path_generation(name: impl AsRef<str>) -> String {
    format!("postman/{}.postman_collection.json", name.as_ref())
}

#[derive(Parser)]
#[command(version, about = "Postman collection runner using newman!", long_about = None)]
struct Args {
    /// Name of the connector
    #[arg(short, long = "connector_name")]
    connector_name: String,
    /// Base URL of the Hyperswitch environment
    #[arg(short, long = "base_url")]
    base_url: String,
    /// Admin API Key of the environment
    #[arg(short, long = "admin_api_key")]
    admin_api_key: String,
    /// Folder name of specific tests
    #[arg(short, long = "folder_name")]
    folder_name_s: Option<String>,
}

fn main() {
    let args = Args::parse();

    let connector_name = args.connector_name;
    let base_url = args.base_url;
    let admin_api_key = args.admin_api_key;

    let collection_path = path_generation(&connector_name);
    let auth_map = ConnectorAuthenticationMap::new();

    let inner_map = auth_map.inner();

    // Newman runner
    // Depending on the conditions satisfied, variables are added. Since certificates of stripe have already
    // been added to the postman collection, those conditions are set to true and collections that have
    // variables set up for certificate, will consider those variables and will fail.

    let mut newman_command = cmd::new("newman");
    newman_command.args(["run", &collection_path]);
    newman_command.args(["--env-var", &format!("admin_api_key={admin_api_key}")]);
    newman_command.args(["--env-var", &format!("baseUrl={base_url}")]);

    if let Some(auth_type) = inner_map.get(&connector_name) {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                // newman_command.args(["--env-var", &format!("connector_api_key={}", api_key.map(|val| val))]);
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
                    &format!("connector_key1={}", key2.peek()),
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

    // Add flags for running specific folders
    if args.folder_name_s.is_some() {
        let folder_names: Vec<String> = args.folder_name_s.unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
    
        for folder_name in folder_names {
            newman_command.args(["--folder", &folder_name]);
        }
    }
    

    newman_command.arg("--delay-request").arg("5");

    newman_command.arg("--color").arg("on");

    // Execute the newman command
    let output = newman_command.spawn();
    let mut child = match output {
        Ok(child) => child,
        Err(err) => {
            eprintln!("Failed to execute command: {err}");
            exit(1);
        }
    };
    let status = child.wait();

    let exit_code = match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Command executed successfully!");
                exit_status.code().unwrap_or(0)
            } else {
                eprintln!("Command failed with exit code: {:?}", exit_status.code());
                exit_status.code().unwrap_or(1)
            }
        }
        Err(err) => {
            eprintln!("Failed to wait for command execution: {err}");
            exit(1);
        }
    };

    exit(exit_code);
}
