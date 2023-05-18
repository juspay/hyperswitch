
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

use std::fs::File;
use std::io::Read;
use serde_json;


fn get_master_data() -> MasterData{
    let mut file = File::open("tests/integration/data/test1.json").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");
    println!("{:?}",contents);
    let master_data: MasterData = serde_json::from_str(&contents).expect("Failed to parse JSON");
    return master_data;
}


#[actix_web::test]
async fn test_api() -> Result<(), Box<dyn std::error::Error>> {
  let mut master_data = get_master_data();
  let server = mk_service_with_db().await;
  execute_merchant_account_create_test(&mut master_data,&server).await;
  execute_api_key_create_tests(&mut master_data,&server).await;
  execute_connector_create_test(&mut master_data,&server).await;
  execute_payment_create_test(&mut master_data,&server).await;
  
  println!("{:?}",master_data);
  Ok(())
}