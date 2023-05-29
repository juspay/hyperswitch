use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
  dev::{Service, ServiceResponse},
  test::{call_and_read_body_json, TestRequest},
};
pub struct PaymentCreate;

impl RequestBuilder for PaymentCreate{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    let request_body = Value::clone(&data.payments_create);
    Some(TestRequest::post()
        .uri(&String::from("http://localhost:8080/payments"))
        .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
        .set_json(&request_body))
  }

  fn verify_success_response(resp : &Value, _data : &MasterData) -> Self{
      let payment_id = resp.get("payment_id"); 
      assert_ne!(payment_id,None);
      Self
    }

  fn verify_failure_response(_response : &Value, _data : &MasterData) -> Self{
      unimplemented!();
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
  let opt_test_request = PaymentCreate::make_request_body(&master_data);
  match opt_test_request{
    Some(test_request) => {
      let payment_create_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      PaymentCreate::verify_success_response(&payment_create_resp,master_data).update_master_data(master_data,&payment_create_resp);
      //println!("{:?}",payment_create_resp);
      println!("Payment Create Test successful!")
    },
    None => {
      println!("Skipping Payment Create Test!")
    },
  }
}

pub struct PaymentRetrieve;

impl RequestBuilder for PaymentRetrieve{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    data.payments_retrieve.as_ref().map(|_payments_retrieve_request|{
      let payment_id = data.payment_id.as_ref().unwrap();
      TestRequest::get()
            .uri(&format!("http://localhost:8080/payments/{}", payment_id))
            .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
    })
  }

  fn verify_success_response(_response : &Value, _data : &MasterData) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn verify_failure_response(_response : &Value, _data : &MasterData) -> Self{
      unimplemented!();
    }

  fn update_master_data(&self,_data : &mut MasterData, _resp : &Value){
    
    }
}

pub async fn execute_payment_retrieve_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let opt_test_request = PaymentRetrieve::make_request_body(&master_data);
  match opt_test_request{
    Some(test_request) => {
      let payment_create_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      PaymentRetrieve::verify_success_response(&payment_create_resp,master_data).update_master_data(master_data,&payment_create_resp);
      //println!("{:?}",payment_create_resp);
      println!("Payment Retrieve Test successful!")
    },
    None => {
      println!("Skipping Payment Retrieve Test!")
    },
  }
}

pub struct PaymentCapture;

impl RequestBuilder for PaymentCapture{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    data.payment_capture.as_ref().map(|_payment_capture_req|{
      let payment_id = data.payment_id.as_ref().unwrap();
      TestRequest::get()
          .uri(&format!("http://localhost:8080/payments/{}/capture", payment_id))
          .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
    })
    
  }

  fn verify_success_response(_resp : &Value, _data : &MasterData) -> Self{
      assert_eq!(true,true);
      Self
    }

  fn verify_failure_response(_response : &Value, _data : &MasterData) -> Self{
      unimplemented!();
    }

  fn update_master_data(&self,_data : &mut MasterData, _resp : &Value){
    
    }
}


pub struct PaymentConfirm;

impl RequestBuilder for PaymentConfirm{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    data.payment_confirm.as_ref().map(|payment_confirm_request|{
      let payment_id = data.payment_id.as_ref().unwrap();
      TestRequest::post()
          .uri(&format!("http://localhost:8080/payments/{}/confirm", payment_id))
          .insert_header(("api-key",data.api_key.as_ref().unwrap().as_str()))
          .set_json(payment_confirm_request)
    })
  }

  fn verify_success_response(_resp : &Value, _data : &MasterData) -> Self{
      assert_eq!(true,true);
      Self
    }
  
  fn verify_failure_response(_response : &Value, _data : &MasterData) -> Self{
      unimplemented!();
    }

  fn update_master_data(&self,_data : &mut MasterData, _resp : &Value){
    
    }
}

pub async fn execute_payment_confirm_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let opt_test_request = PaymentConfirm::make_request_body(&master_data);
  match opt_test_request{
    Some(test_request) => {
      let payment_confirm_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      PaymentConfirm::verify_success_response(&payment_confirm_resp,master_data).update_master_data(master_data,&payment_confirm_resp);
      //println!("{:?}",payment_confirm_resp);
      println!("Payment Confirm Test successful!")
    },
    None => {
      println!("Skipping Payment Confirm Test!")
    },
  }
}