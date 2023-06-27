// For running tests, we can use unwrap()
// cargo test --package router --test connectors -- collection_runner nmi P9R3ZMmohOKMc7GROqRPFPjw4 https://integ-api.hyperswitch.io
use router::types::{self, api, storage::enums};
use std::{env, process::Command};

use crate::{
    ::connector_auth,
    utils::{self, ConnectorActions},
};

fn path_generation(name: &str) -> String {
    let collection_name = format!("postman/{}.postman_collection.json", name);
    collection_name
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        println!("Usage: cargo collection_runner <connector_name> <admin_api_key> <base_url>");
        return;
    }

    let connector_name = &args[1];
    let admin_api_key = &args[2];
    let base_url = &args[3];

    let mut key_type = String::new();
    let collection_path = path_generation(connector_name);

    let auth_map = connector_auth::ConnectorAuthenticationMap::new();
    let key = &args[1];
    let auth_info = auth_map.lookup(key);

    let mut newman_command = Command::new("newman");
    newman_command.arg("run");
    newman_command.arg(&collection_path);
    newman_command.arg("--env-var");
    newman_command.arg(format!(
        "admin_api_key={}",
        env::var("ADMIN_API_KEY").unwrap()
    ));
    newman_command.arg("--env-var");
    newman_command.arg(format!("baseUrl={}", env::var("BASE_URL").unwrap()));
    newman_command.arg(&auth_info);

    // Add additional environment variables if present
    if let Ok(gateway_merchant_id) = env::var("GATEWAY_MERCHANT_ID") {
        newman_command.arg("--env-var");
        newman_command.arg(format!("gateway_merchant_id={}", gateway_merchant_id));
    }
    if let Ok(gpay_certificate) = env::var("GPAY_CERTIFICATE") {
        newman_command.arg("--env-var");
        newman_command.arg(format!("certificate={}", gpay_certificate));
    }
    if let Ok(gpay_certificate_keys) = env::var("GPAY_CERTIFICATE_KEYS") {
        newman_command.arg("--env-var");
        newman_command.arg(format!("certificate_keys={}", gpay_certificate_keys));
    }

    newman_command.arg("--delay-request");
    newman_command.arg("5");

    // Execute the newman command
    let output = newman_command
        .output()
        .expect("Failed to execute newman command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Newman command executed successfully:\n{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Failed to execute newman command:\n{}", stderr);
    }
}
