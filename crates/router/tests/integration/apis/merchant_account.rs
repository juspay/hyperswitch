use crate::integration::types::*;
use serde_json::value::{Value};
use serde_json::value;
use actix_http::{body::MessageBody, Request};
use actix_web::{
  dev::{Service, ServiceResponse},
  test::{call_and_read_body_json, TestRequest},
};

pub struct MerchantAccount;

impl RequestBuilder for MerchantAccount{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    let request_body = Value::clone(&data.merchant_account);
    Some(TestRequest::post()
        .uri(&String::from("http://localhost:8080/accounts"))
        .insert_header(("api-key",data.admin_api_key.as_str()))
        .set_json(&request_body))
  }

  fn verify_success_response(resp : &Value, data : &MasterData) -> Self{
      let req_mid = data.merchant_account.get("merchant_id");
      let res = resp.get("merchant_id");
      assert_eq!(req_mid,res);
      Self
    }
  fn verify_failure_response(response : &Value, data : &MasterData) -> Self{
      unimplemented!();
    }
  
  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
      if let Some(mid) = resp.get("merchant_id"){
        match mid{
          Value::String(m)=> data.merchant_id = Some(m.to_string()),
          _ => data.merchant_id = None,
        };
      }
      else{
        data.merchant_id = None
      }
  }
}

pub async fn execute_merchant_account_create_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let opt_test_req = MerchantAccount::make_request_body(&master_data);
  match opt_test_req{
    Some(test_request) => {
      let merchant_account_create_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      MerchantAccount::verify_success_response(&merchant_account_create_resp,master_data).update_master_data(master_data,&merchant_account_create_resp);
      println!("{:?}",merchant_account_create_resp);
    },
    None => {
      println!("Skipping Payment Create Test!")
    },
  }
}