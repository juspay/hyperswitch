use actix_web::test::TestRequest;
use serde_json::value::{Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MasterData{
  pub merchant_account : Value,
  pub merchant_id : Option<String>,
  pub admin_api_key : String,
  pub customers : Option<Value>,
  pub connector_create : Value,
  pub api_key_create : Value,
  pub payments_create : Value,
  pub payments_retrieve : Option<Value>,
  pub customer_id : Option<String>,
  pub api_key : Option<String>,
  pub payment_id : Option<String>,
}

pub trait RequestBuilder{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>;
  fn verify_success_response(response : &Value, data : &MasterData) -> Self;
  fn verify_failure_response(response : &Value, data : &MasterData) -> Self;
  fn update_master_data(&self,data : &mut MasterData, resp : &Value);
}