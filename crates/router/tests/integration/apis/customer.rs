use crate::integration::types::*;
use actix_web::test::TestRequest;
use serde_json::value::{Value};

pub struct Customer;

impl RequestBuilder for Customer{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.customers.as_ref().unwrap());
    TestRequest::post()
        .uri(&String::from("http://localhost:8080/customers"))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
        .set_json(&request_body)
  }

  fn verify_response(s : &Value) -> Self{
      assert_eq!(true,true);
      Self
  }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
  }

}
