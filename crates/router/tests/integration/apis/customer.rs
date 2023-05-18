use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
  dev::{Service, ServiceResponse},
  test::{call_and_read_body_json, TestRequest},
};

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

pub async fn execute_customer_create_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let customer_create_resp = call_and_read_body_json(&server,Customer::make_request_body(&master_data).to_request()).await;
  Customer::verify_response(&Customer_create_resp).update_master_data(master_data,&customer_create_resp);
  println!("{:?}",customer_create_resp);
}