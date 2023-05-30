use std::fs;

use actix_web::test::TestRequest;
use serde::{Deserialize, Serialize};
use serde_json::{self, value::Value};

#[derive(Debug, Deserialize, Serialize)]
pub struct MasterData {
    // fields derieved from Api responses
    pub merchant_id: Option<String>,
    pub api_key: Option<String>,
    pub api_key_id: Option<String>,
    pub payment_id: Option<String>,
    pub customer_id: Option<String>,
    // fields in test data
    pub admin_api_key: String,
    pub merchant_account: Value,
    pub merchant_account_update: Option<Value>,
    pub merchant_account_delete: Option<Value>,
    pub merchant_account_retrieve: Option<Value>,
    pub customers: Option<Value>,
    pub connector_create: Option<Value>,
    pub api_key_create: Option<Value>,
    pub api_key_update: Option<Value>,
    pub api_key_delete: Option<Value>,
    pub api_key_retrieve: Option<Value>,
    pub payments_create: Option<Value>,
    pub payments_retrieve: Option<Value>,
    pub payment_confirm: Option<Value>,
    pub payment_capture: Option<Value>,
}

pub trait RequestBuilder {
    fn make_request_body(data: &MasterData) -> Option<TestRequest>;
    fn verify_success_response(response: &Value, data: &MasterData) -> Self;
    fn verify_failure_response(response: &Value, data: &MasterData) -> Self;
    fn update_master_data(&self, data: &mut MasterData, resp: &Value);
}

fn get_master_data(test_file_path: std::path::PathBuf) -> MasterData {
    let contents = fs::read_to_string(&test_file_path).expect("Failed to read file");
    let master_data: MasterData = serde_json::from_str(&contents).expect("Failed to parse JSON");
    //println!("Initial Master Data : \n {:?}",master_data);
    return master_data;
}

pub fn collect_test_data(test_data_dir_path: &str) -> Result<Vec<(String, MasterData)>, String> {
    if let Ok(test_files) = fs::read_dir(test_data_dir_path) {
        let mut master_data_list = Vec::new();
        for test_file in test_files {
            if let Ok(test_file) = test_file {
                let test_file_path = test_file.path();
                if test_file_path.is_file() {
                    let master_data = get_master_data(test_file_path);
                    master_data_list
                        .push((test_file.path().to_string_lossy().into_owned(), master_data));
                }
            }
        }
        Ok(master_data_list)
    } else {
        Err(String::from("Unable to read dir"))
    }
}
