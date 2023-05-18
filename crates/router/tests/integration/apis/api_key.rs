use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
    dev::{Service, ServiceResponse},
    test::{call_and_read_body_json, TestRequest},
};
pub struct ApiKey;

impl RequestBuilder for ApiKey{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.api_key_create);
    let mid = data.merchant_id.as_ref().unwrap();
    TestRequest::post()
        .uri(&format!("http://localhost:8080/api_keys/{}", mid))
        .insert_header(("api-key",data.admin_api_key.as_str()))
        .set_json(&request_body)
  }

  fn verify_response(resp : &Value) -> Self{
      let api_key = resp.get("api_key");
      assert_ne!(api_key,None);
      Self
  }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    if let Some(mid) = resp.get("api_key"){
      match mid{
        Value::String(m)=> data.api_key = Some(m.to_string()),
        _ => data.api_key = None,
      };
    }
    else{
      data.api_key = None
    }
  }

}

pub async fn execute_api_key_create_tests(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let api_resp = call_and_read_body_json(&server,ApiKey::make_request_body(&master_data).to_request()).await;
  ApiKey::verify_response(&api_resp).update_master_data(master_data,&api_resp);
  println!("{:?}",api_resp);
}
