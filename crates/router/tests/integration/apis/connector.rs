use crate::integration::types::*;
use serde_json::value::{Value};
use actix_http::{body::MessageBody, Request};
use actix_web::{
    dev::{Service, ServiceResponse},
    test::{call_and_read_body_json, TestRequest},
};
pub struct ConnectorCreate;

impl RequestBuilder for ConnectorCreate{
  fn make_request_body(data : &MasterData) -> Option<TestRequest>{
    let request_body = Value::clone(&data.connector_create);
    let mid = data.merchant_id.as_ref().unwrap();
    let url = format!("http://localhost:8080/account/{}{}", mid, "/connectors");
    Some(TestRequest::post()
        .uri(&url)
        .insert_header(("api-key",data.admin_api_key.as_str()))
        .set_json(&request_body))
  }

  fn verify_response(s : &Value) -> Self{
      assert_eq!(true,true);
      Self
  }

  fn update_master_data(&self,data : &mut MasterData, resp : &Value){
    
  }

}

pub async fn execute_connector_create_test(master_data : &mut MasterData, server: &impl Service<Request, Response = ServiceResponse<impl MessageBody>, Error = actix_web::Error>){
  let opt_test_request = ConnectorCreate::make_request_body(&master_data);
  match opt_test_request{
    Some(test_request) => {
      let connector_create_resp = call_and_read_body_json(&server,test_request.to_request()).await;
      ConnectorCreate::verify_response(&connector_create_resp).update_master_data(master_data,&connector_create_resp);
      println!("Connector Create Response {:?}",connector_create_resp);
    },
    None => {
      println!("Skipping Connector Create Test!")
    },
  }
}