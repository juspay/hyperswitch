use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    process::{exit, Command},
};

use clap::{arg, command, Parser};
use masking::PeekInterface;
use regex::Regex;

use crate::connector_auth::{ConnectorAuthType, ConnectorAuthenticationMap};
#[derive(Parser)]
#[command(version, about = "Postman collection runner using newman!", long_about = None)]
struct Args {
    /// Admin API Key of the environment
    #[arg(short, long)]
    admin_api_key: String,
    /// Base URL of the Hyperswitch environment
    #[arg(short, long)]
    base_url: String,
    /// Name of the connector
    #[arg(short, long)]
    connector_name: String,
    /// Custom headers
    #[arg(short = 'H', long = "header")]
    custom_headers: Option<Vec<String>>,
    /// Minimum delay in milliseconds to be added before sending a request
    /// By default, 7 milliseconds will be the delay
    #[arg(short, long, default_value_t = 7)]
    delay_request: u32,
    /// Folder name of specific tests
    #[arg(short, long = "folder")]
    folders: Option<String>,
    /// Optional Verbose logs
    #[arg(short, long)]
    verbose: bool,
}

pub struct ReturnArgs {
    pub newman_command: Command,
    pub modified_file_paths: Vec<Option<String>>,
    pub collection_path: String,
}

// Generates the name of the collection JSON file for the specified connector.
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/collection-json/stripe.postman_collection.json
#[inline]
fn get_collection_path(name: impl AsRef<str>) -> String {
    format!(
        "postman/collection-json/{}.postman_collection.json",
        name.as_ref()
    )
}

// Generates the name of the collection directory for the specified connector.
// Example: CONNECTOR_NAME="stripe" -> OUTPUT: postman/collection-dir/stripe
#[inline]
fn get_dir_path(name: impl AsRef<str>) -> String {
    format!("postman/collection-dir/{}", name.as_ref())
}

// This function currently allows you to add only custom headers.
// In future, as we scale, this can be modified based on the need
fn insert_content<T, U>(dir: T, content_to_insert: U) -> std::io::Result<()>
where
    T: AsRef<Path>,
    U: AsRef<str>,
{
    let file_name = "event.prerequest.js";
    let file_path = dir.as_ref().join(file_name);

    // Open the file in write mode or create it if it doesn't exist
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    write!(file, "{}", content_to_insert.as_ref())?;

    Ok(())
}

pub fn generate_newman_command() -> ReturnArgs {
    let args = Args::parse();

    let connector_name = args.connector_name;
    let base_url = args.base_url;
    let admin_api_key = args.admin_api_key;

    let collection_path = get_collection_path(&connector_name);
    let collection_dir_path = get_dir_path(&connector_name);
    let auth_map = ConnectorAuthenticationMap::new();

    let inner_map = auth_map.inner();

    /*
    Newman runner
    Certificate keys are added through secrets in CI, so there's no need to explicitly pass it as arguments.
    It can be overridden by explicitly passing certificates as arguments.

    If the collection requires certificates (Stripe collection for example) during the merchant connector account create step,
    then Stripe's certificates will be passed implicitly (for now).
    If any other connector requires certificates to be passed, that has to be passed explicitly for now.
    */

    let mut newman_command = Command::new("newman");
    newman_command.args(["run", &collection_path]);
    newman_command.args(["--env-var", &format!("admin_api_key={admin_api_key}")]);
    newman_command.args(["--env-var", &format!("baseUrl={base_url}")]);

    let custom_header_exist = check_for_custom_headers(args.custom_headers, &collection_dir_path);

    // validation of connector is needed here as a work around to the limitation of the fork of newman that Hyperswitch uses
    let (connector_name, modified_collection_file_paths) =
        check_connector_for_dynamic_amount(&connector_name);

    if let Some(auth_type) = inner_map.get(connector_name) {
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

    newman_command.args([
        "--delay-request",
        format!("{}", &args.delay_request).as_str(),
    ]);

    newman_command.arg("--color").arg("on");

    // Add flags for running specific folders
    if let Some(folders) = &args.folders {
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

    ReturnArgs {
        newman_command,
        modified_file_paths: vec![modified_collection_file_paths, custom_header_exist],
        collection_path,
    }
}

pub fn check_for_custom_headers(headers: Option<Vec<String>>, path: &str) -> Option<String> {
    if let Some(headers) = &headers {
        for header in headers {
            if let Some((key, value)) = header.split_once(':') {
                let content_to_insert =
                    format!(r#"pm.request.headers.add({{key: "{key}", value: "{value}"}});"#);
                let _ = insert_content(path, &content_to_insert).is_ok();
            } else {
                eprintln!("Invalid header format: {}", header);
            }
        }

        return Some(format!("{}/event.prerequest.js", path));
    }
    None
}

// If the connector name exists in dynamic_amount_connectors,
// the corresponding collection is modified at runtime to remove double quotes
pub fn check_connector_for_dynamic_amount(connector_name: &str) -> (&str, Option<String>) {
    let collection_dir_path = get_dir_path(connector_name);

    let dynamic_amount_connectors = ["nmi", "powertranz"];

    if dynamic_amount_connectors.contains(&connector_name) {
        return remove_quotes_for_integer_values(connector_name).unwrap_or((connector_name, None));
    }
    /*
    If connector name does not exist in dynamic_amount_connectors but we run it on custom_pod,
    since we're running from collections directly, we'll have to export the collection again and it is much simpler.
    We could directly inject the custom-headers using regex, but it is not encouraged as it is hard
    to determine the place of edit.
    */
    export_collection(connector_name, collection_dir_path);

    (connector_name, None)
}

/*
Existing issue with the fork of newman is that, it requires you to pass variables like `{{value}}` within
double quotes without which it fails to execute.
For integer values like `amount`, this is a bummer as it flags the value stating it is of type
string and not integer.
Refactoring is done in 2 steps:
- Export the collection to json (although the json will be up-to-date, we export it again for safety)
- Use regex to replace the values which removes double quotes from integer values
  Ex: \"{{amount}}\" -> {{amount}}
*/

pub fn remove_quotes_for_integer_values(
    connector_name: &str,
) -> Result<(&str, Option<String>), io::Error> {
    let collection_path = get_collection_path(connector_name);
    let collection_dir_path = get_dir_path(connector_name);

    let values_to_replace = [
        "amount",
        "another_random_number",
        "capture_amount",
        "random_number",
        "refund_amount",
    ];

    export_collection(connector_name, collection_dir_path);

    let mut contents = fs::read_to_string(&collection_path)?;
    for value_to_replace in values_to_replace {
        if let Ok(re) = Regex::new(&format!(
            r#"\\"(?P<field>\{{\{{{}\}}\}})\\""#,
            value_to_replace
        )) {
            contents = re.replace_all(&contents, "$field").to_string();
        } else {
            eprintln!("Regex validation failed.");
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&collection_path)?;

        file.write_all(contents.as_bytes())?;
    }

    Ok((connector_name, Some(collection_path)))
}

pub fn export_collection(connector_name: &str, collection_dir_path: String) {
    let collection_path = get_collection_path(connector_name);

    let mut newman_command = Command::new("newman");
    newman_command.args([
        "dir-import".to_owned(),
        collection_dir_path,
        "-o".to_owned(),
        collection_path.clone(),
    ]);

    match newman_command.spawn().and_then(|mut child| child.wait()) {
        Ok(exit_status) => {
            if exit_status.success() {
                println!("Conversion of collection from directory structure to json successful!");
            } else {
                eprintln!("Conversion of collection from directory structure to json failed!");
                exit(exit_status.code().unwrap_or(1));
            }
        }
        Err(err) => {
            eprintln!("Failed to execute dir-import: {:?}", err);
            exit(1);
        }
    }
}
