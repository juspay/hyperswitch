use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
  dev::{Service, ServiceResponse},
  test::{call_and_read_body_json, TestRequest},
};
pub struct PaymentCreate;

impl RequestBuilder for PaymentCreate{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.payments_create);
    TestRequest::post()
        .uri(&String::from("http://localhost:8080/payments"))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
        .set_json(&request_body)
  }

  fn verify_response(resp : &Value) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    if let Some(mid) = resp.get("payment_id"){
      match mid{
        Value::String(m)=> data.payment_id = Some(m.to_string()),
        _ => data.payment_id = None,
      };
    }
    else{
      data.payment_id = None
    }
  }
}


pub async fn execute_payment_create_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let payment_create_resp = call_and_read_body_json(&server,PaymentCreate::make_request_body(&master_data).to_request()).await;
  PaymentCreate::verify_response(&payment_create_resp).update_master_data(master_data,&payment_create_resp);
  println!("{:?}",payment_create_resp);
}

pub struct PaymentRetrieve;

impl RequestBuilder for PaymentRetrieve{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let payment_id = data.payment_id.as_ref().unwrap();
    TestRequest::get()
        .uri(&format!("http://localhost:8080/payments/{}", payment_id))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
  }

  fn verify_response(resp : &Value) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
    }
}

pub struct PaymentCapture;

impl RequestBuilder for PaymentCapture{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let payment_id = data.payment_id.as_ref().unwrap();
    TestRequest::get()
        .uri(&format!("http://localhost:8080/payments/{}/capture", payment_id))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
  }

  fn verify_response(resp : &Value) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
    }
}


pub struct PaymentConfirm;

impl RequestBuilder for PaymentConfirm{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let payment_id = data.payment_id.as_ref().unwrap();
    TestRequest::get()
        .uri(&format!("http://localhost:8080/payments/{}/confirm", payment_id))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
  }

  fn verify_response(resp : &Value) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
    }
}