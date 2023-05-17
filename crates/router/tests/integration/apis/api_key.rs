use crate::integration::types::*;
use actix_web::test::TestRequest;
use serde_json::value::{Value};

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
