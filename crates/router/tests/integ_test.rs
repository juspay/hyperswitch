
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

use actix_web::test::TestRequest;
use serde_json::value::{Value};
use serde_json::json;
use actix_web::{test, HttpRequest, HttpResponse, HttpMessage};
use actix_web::http::{header, StatusCode};
use utils::{mk_service_with_db};
use actix_web::test::call_and_read_body_json;
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
  

  let ma_json_resp : Value = call_and_read_body_json(&server,MerchantAccount::make_request_body(&master_data).to_request()).await;
  MerchantAccount::verify_response(&ma_json_resp).update_master_data(&mut master_data,&ma_json_resp);
  println!("{:?}",ma_json_resp);

  let api_resp = call_and_read_body_json(&server,ApiKey::make_request_body(&master_data).to_request()).await;
  ApiKey::verify_response(&api_resp).update_master_data(&mut master_data,&api_resp);
  println!("{:?}",api_resp);

  let connector_create_resp = call_and_read_body_json(&server,ConnectorCreate::make_request_body(&master_data).to_request()).await;
  ConnectorCreate::verify_response(&connector_create_resp).update_master_data(&mut master_data,&connector_create_resp);
  println!("{:?}",connector_create_resp);

  let payment_create_resp = call_and_read_body_json(&server,PaymentCreate::make_request_body(&master_data).to_request()).await;
  PaymentCreate::verify_response(&payment_create_resp).update_master_data(&mut master_data,&payment_create_resp);

  println!("{:?}",payment_create_resp);
  println!("{:?}",master_data);
  Ok(())
}