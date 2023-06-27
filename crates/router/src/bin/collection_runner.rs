mod auth {
    include!("../../tests/connectors/connector_auth.rs");
}
use router::types::ConnectorAuthType;
use std::{env, process::Command};

// Just by the name of the connector, this function generates the name of the collection
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/stripe.postman_collection.json
fn path_generation(name: &str) -> String {
    format!("postman/{}.postman_collection.json", name)
}

// Removes double quotes
fn trim(str: &str) -> String {
    str.replace("\"", "")
}

// runner starts here
fn main() {
    let args: Vec<String> = env::args().collect();

    // Usage Info
    if args.len() < 4 {
        println!("Usage: cargo collection_runner <connector_name> <base_url> <admin_api_key>");
        return;
    }

    // Arguments
    let connector_name = &args[1];
    let base_url = &args[2];
    let admin_api_key = &args[3];

    // Function calls
    let collection_path = path_generation(connector_name);
    let auth_map = auth::ConnectorAuthenticationMap::new();

    let inner_map = &auth_map.0;

    // Newman runner
    // Depending on the conditions satisfied, variables are added. Since certificates of stripe have already
    // been added to the postman collection, those conditions are set to true and collections that have
    // variables set up for certificate, will consider those variables and will fail.

    let mut newman_command = Command::new("newman");

    newman_command.arg("run");
    newman_command.arg(&collection_path);

    newman_command
        .arg("--env-var")
        .arg(format!("admin_api_key={}", admin_api_key));

    newman_command
        .arg("--env-var")
        .arg(format!("baseUrl={}", base_url));

    if let Some(auth_type) = inner_map.get(&connector_name.to_string()) {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={}", trim(api_key)));
            }
            ConnectorAuthType::BodyKey { api_key, key1 } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={}", trim(api_key)))
                    .arg("--env-var")
                    .arg(format!("connector_key1={}", trim(key1)));
            }
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={}", trim(api_key)))
                    .arg("--env-var")
                    .arg(format!("connector_key1={}", trim(key1)))
                    .arg("--env-var")
                    .arg(format!("connector_api_secret={}", trim(api_secret)));
            }
            ConnectorAuthType::MultiAuthKey {
                api_key,
                key1,
                key2,
                api_secret,
            } => {
                newman_command
                    .arg("--env-var")
                    .arg(format!("connector_api_key={}", trim(api_key)))
                    .arg("--env-var")
                    .arg(format!("connector_key1={}", trim(key1)))
                    .arg("--env-var")
                    .arg(format!("connector_key2={}", trim(key2)))
                    .arg("--env-var")
                    .arg(format!("connector_api_secret={}", trim(api_secret)));
            }
            // Handle other ConnectorAuthType variants
            _ => {
                println!("Invalid authentication type.");
            }
        }
    } else {
        println!("Connector not found.");
    }

    // Add additional environment variables if present
    if let Ok(gateway_merchant_id) = env::var("GATEWAY_MERCHANT_ID") {
        newman_command
            .arg("--env-var")
            .arg(format!("gateway_merchant_id={}", gateway_merchant_id));
    }

    if let Ok(gpay_certificate) = env::var("GPAY_CERTIFICATE") {
        newman_command
            .arg("--env-var")
            .arg(format!("certificate={}", gpay_certificate));
    }

    if let Ok(gpay_certificate_keys) = env::var("GPAY_CERTIFICATE_KEYS") {
        newman_command
            .arg("--env-var")
            .arg(format!("certificate_keys={}", gpay_certificate_keys));
    }

    newman_command.arg("--delay-request").arg("5");

    // Execute the newman command
    let output = newman_command
        .spawn()
        .expect("Failed to execute newman command")
        .wait_with_output();

    if output
        .as_ref()
        .map(|stat| stat.status.success())
        .unwrap_or_default()
    {
        // Command executed successfully
        let stdout = output
            .as_ref()
            .map(|out| out.clone().stdout)
            .expect("Failed to read stdout");
        let stderr = output
            .map(|err| err.stderr)
            .expect("Failed to read the error response");
        println!("stdout: {:#?}", stdout);
        println!("stderr: {:#?}", stderr);
    } else {
        // Command execution failed
        let stderr = output
            .map(|err| err.stderr)
            .expect("Failed to read the error response");
        eprintln!("Command failed with error: {:#?}", stderr);
    }
}
