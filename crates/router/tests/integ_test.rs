
mod utils;
mod integration{
  pub mod types;
  pub mod apis{
    pub mod connector;
    pub mod merchant_account;
    pub mod payment;
    pub mod api_key;
  }
}

use utils::{mk_service_with_db};
use crate::integration::types::*;
use crate::integration::apis::merchant_account::*;
use crate::integration::apis::connector::*;
use crate::integration::apis::payment::*;
use crate::integration::apis::api_key::*;
use std::fs;
use std::fs::File;
use std::io::Read;
use serde_json;


fn get_master_data(test_file_path : std::path::PathBuf) -> MasterData{
    let contents = fs::read_to_string(&test_file_path).expect("Failed to read file");
    let master_data: MasterData = serde_json::from_str(&contents).expect("Failed to parse JSON");
    println!("Initial Master Data : \n {:?}",master_data);
    return master_data;
}

#[actix_web::test]
async fn run_integration_test(){
  let test_input_dir = "./tests/integration/data";
  if let Ok(test_files) = fs::read_dir(test_input_dir) {
      for test_file in test_files {
          if let Ok(test_file) = test_file {
              let test_file_path = test_file.path();
              if test_file_path.is_file() {
                let mut master_data = get_master_data(test_file_path);
                let test_result = test_api(&mut master_data).await;
                match test_result{
                  Ok(()) => println!("Test execution successfull"),
                  Err(error) => println!("Test execution failed with error : {}",error),
                }
              }
          }
      }
  } else {
      println!("Failed to read directory: {}", test_input_dir);
  }

}

async fn test_api(master_data : &mut MasterData) -> Result<(), Box<dyn std::error::Error>> {
  let server = mk_service_with_db().await;
  execute_merchant_account_create_test(master_data,&server).await;
  execute_api_key_create_tests(master_data,&server).await;
  execute_connector_create_test(master_data,&server).await;
  execute_payment_create_test(master_data,&server).await;
  
  println!("Final Master Data : \n{:?}",master_data);
  Ok(())
}