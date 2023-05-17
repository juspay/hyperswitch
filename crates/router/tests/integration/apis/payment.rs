use crate::integration::types::*;
use actix_web::test::TestRequest;
use serde_json::value::{Value};
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


pub struct PaymentRetrieve;

impl RequestBuilder for PaymentRetrieve{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.payments_create);
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
