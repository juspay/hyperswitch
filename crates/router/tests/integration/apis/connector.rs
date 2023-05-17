use crate::integration::types::*;
use actix_web::test::TestRequest;
use serde_json::value::{Value};

pub struct ConnectorCreate;

impl RequestBuilder for ConnectorCreate{
  fn make_request_body(data : &MasterData) -> TestRequest{
    let request_body = Value::clone(&data.connector_create);
    let mid = data.merchant_id.as_ref().unwrap();
    let url = format!("http://localhost:8080/account/{}{}", mid, "/connectors");
    TestRequest::post()
        .uri(&url)
        .insert_header(("api-key",data.admin_api_key.as_str()))
        .set_json(&request_body)
  }

  fn verify_response(s : &Value) -> Self{
      assert_eq!(true,true);
      Self
  }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
  }

}
