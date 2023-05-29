
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
use crate::integration::apis::customer::*;
use crate::integration::apis::payment::*;
use crate::integration::apis::api_key::*;

#[actix_web::test]
async fn run_payment_apis_test(){
  let test_input_dir = "./tests/senarios/payments_apis";
  if let Ok(test_data_list) = collect_test_data(test_input_dir){
    for (test_file_path,mut test_master_data) in test_data_list{
      println!("Test execution started for : {:?}\n",test_file_path);
      let test_result = execute_test_sequence(&mut test_master_data).await;
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
async fn execute_test_sequence(master_data : &mut MasterData) -> Result<(), Box<dyn std::error::Error>> {
  let server = mk_service_with_db().await;
  execute_merchant_account_create_test(master_data,&server).await;
  execute_api_key_create_test(master_data,&server).await;
  execute_customer_create_test(master_data,&server).await;
  execute_connector_create_test(master_data,&server).await;
  execute_payment_create_test(master_data,&server).await;
  execute_payment_confirm_test(master_data,&server).await;
  execute_payment_retrieve_test(master_data,&server).await;
  execute_api_key_delete_test(master_data,&server).await;
  execute_merchant_account_delete_test(master_data,&server).await;
  //println!("Final Master Data : \n{:?}",master_data);
  Ok(())
}