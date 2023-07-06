use std::{
    env,
    process::{exit, Command as cmd},
};

use clap::{arg, command, Parser};
use router::types::ConnectorAuthType;
use test_utils::connector_auth::ConnectorAuthenticationMap;

// Just by the name of the connector, this function generates the name of the collection
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/stripe.postman_collection.json
fn path_generation(name: &String) -> String {
    format!("postman/{}.postman_collection.json", name)
}

#[derive(Debug, Parser)]
#[command(author = "Me, PiX, I'll remove this, pinky promise!", version, about = "Postman collection runner using newman!", long_about = None)]
struct Arguments {
    /// Name of the connector
    #[arg(long)]
    connector_name: String,
    /// Base URL of the Hyperswitch environment
    #[arg(long)]
    base_url: String,
    /// Admin API Key of the environment
    #[arg(long)]
    admin_api_key: String,
}

// runner starts here
fn main() {
    let args = Arguments::parse();

    let c_name = args.connector_name;
    let b_url = args.base_url;
    let a_api_key = args.admin_api_key;

    // Function calls
    let collection_path = path_generation(&c_name);
    let auth_map = ConnectorAuthenticationMap::new();

    let inner_map = &auth_map.0;

    // Newman runner
    // Depending on the conditions satisfied, variables are added. Since certificates of stripe have already
    // been added to the postman collection, those conditions are set to true and collections that have
    // variables set up for certificate, will consider those variables and will fail.

    let mut newman_command = cmd::new("newman");

    newman_command.arg("run");
    newman_command.arg(&collection_path);

    newman_command
        .arg("--env-var")
        .arg(format!("admin_api_key={a_api_key}"));

    newman_command
        .arg("--env-var")
        .arg(format!("baseUrl={b_url}"));

    if let Some(auth_type) = inner_map.get(&c_name) {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={api_key}"));
            }
            ConnectorAuthType::BodyKey { api_key, key1 } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={api_key}"))
                    .arg("--env-var")
                    .arg(format!("connector_key1={key1}"));
            }
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={api_key}"))
                    .arg("--env-var")
                    .arg(format!("connector_key1={key1}"))
                    .arg("--env-var")
                    .arg(format!("connector_api_secret={api_secret}"));
            }
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                key2,
                api_secret,
            } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={api_key}"))
                    .arg("--env-var")
                    .arg(format!("connector_key1={key1}"))
                    .arg("--env-var")
                    .arg(format!("connector_key2={key2}"))
                    .arg("--env-var")
                    .arg(format!("connector_api_secret={api_secret}"));
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
        newman_command
            .arg("--env-var")
            .arg(format!("gateway_merchant_id={gateway_merchant_id}"));
    }

    if let Ok(gpay_certificate) = env::var("GPAY_CERTIFICATE") {
        newman_command
            .arg("--env-var")
            .arg(format!("certificate={gpay_certificate}"));
    }

    if let Ok(gpay_certificate_keys) = env::var("GPAY_CERTIFICATE_KEYS") {
        newman_command
            .arg("--env-var")
            .arg(format!("certificate_keys={gpay_certificate_keys}"));
    }

    newman_command.arg("--delay-request").arg("5");

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
            eprintln!("Failed to wait for command execution: {}", err);
            exit(1);
        }
    };

    exit(exit_code);
}
