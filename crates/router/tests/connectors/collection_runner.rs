// For running tests, we can use unwrap()

use std::{env, process::Command};

use crate::connector_auth;

fn path_generation(name: &str) -> String {
    format!("postman/{}.postman_collection.json", name)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        println!("Usage: cargo collection_runner <connector_name> <base_url> <admin_api_key>");
        return;
    }

    let connector_name = &args[1];
    let base_url = &args[2];
    let admin_api_key = &args[3];

    let collection_path = path_generation(connector_name);

    let auth_map = connector_auth::ConnectorAuthenticationMap::new();
    let auth_info = auth_map.lookup(connector_name);

    let mut newman_command = Command::new("newman");
    newman_command.arg("run");
    newman_command.arg(&collection_path);
    newman_command
        .arg("--env-var")
        .arg(format!("admin_api_key={}", admin_api_key));
    newman_command
        .arg("--env-var")
        .arg(format!("baseUrl={}", base_url));
    newman_command.arg(&auth_info);

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
        .output()
        .expect("Failed to execute newman command");

    if output.status.success() {
        println!("Collection run completed successfully");
    } else {
        let error_message = String::from_utf8_lossy(&output.stderr);
        println!("Failed to execute newman command:\n{}", error_message);
    }
}
