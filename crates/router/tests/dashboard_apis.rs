
mod utils;
mod integration{
  pub mod types;
  pub mod apis{
    pub mod connector;
    pub mod merchant_account;
    pub mod payment;
    pub mod api_key;
    pub mod customer;
  }
}

use utils::{mk_service_with_db};
use crate::integration::types::*;
use crate::integration::apis::merchant_account::*;
use crate::integration::apis::connector::*;
use crate::integration::apis::api_key::*;
use std::fs;
use serde_json;

#[actix_web::test]
async fn run_dashboard_api_test(){
  let test_input_dir = "./tests/senarios/dashboard_apis";
  if let Ok(test_data_list) = collect_test_data(test_input_dir){
    for (test_file_path,mut test_master_data) in test_data_list{
      println!("Test execution started for : {:?}\n",test_file_path);
      let test_result = test_api(&mut test_master_data).await;
      match test_result{
        Ok(()) => println!("Test execution successful : {:?}\n",test_file_path),
        Err(error) => println!("Test execution failed for path: {:?} with error : {}\n",test_file_path,error),
      }
}
  }
  else{
    println!("Failed to read directory: {}", test_input_dir);
  }
}
//TODO: Add brackets to create and delete resources
async fn test_api(master_data : &mut MasterData) -> Result<(), Box<dyn std::error::Error>> {
  let server = mk_service_with_db().await;
  execute_merchant_account_create_test(master_data,&server).await;
  execute_api_key_create_test(master_data,&server).await;
  execute_api_key_update_test(master_data,&server).await;
  execute_connector_create_test(master_data,&server).await;
  execute_api_key_delete_test(master_data,&server).await;
  execute_merchant_account_delete_test(master_data,&server).await;
  Ok(())
}