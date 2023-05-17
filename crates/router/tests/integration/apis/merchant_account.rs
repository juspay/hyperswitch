use crate::integration::types::*;
use actix_web::test::TestRequest;
use serde_json::value::{Value};
use serde_json::value;
pub struct MerchantAccount;

impl RequestBuilder for MerchantAccount{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.merchant_account);
    TestRequest::post()
        .uri(&String::from("http://localhost:8080/accounts"))
        .insert_header(("api-key",data.admin_api_key.as_str()))
        .set_json(&request_body)
  }

  fn verify_response(resp : &Value) -> Self{
      let res = resp.get("merchant_id");
      let req_mid = resp.get("merchant_id");
      assert_eq!(req_mid,res);
      Self
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
