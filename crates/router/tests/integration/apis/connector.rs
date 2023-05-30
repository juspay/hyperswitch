use actix_http::{body::MessageBody, Request};
use actix_web::{
    dev::{Service, ServiceResponse},
    test::{call_and_read_body_json, TestRequest},
};
use serde_json::value::Value;

use crate::integration::types::*;
pub struct ConnectorCreate;

impl RequestBuilder for ConnectorCreate {
    fn make_request_body(data: &MasterData) -> Option<TestRequest> {
        data.connector_create.as_ref().map(|connector_create| {
            let request_body = Value::clone(connector_create);
            let mid = data.merchant_id.as_ref().unwrap();
            let url = format!("http://localhost:8080/account/{}{}", mid, "/connectors");
            TestRequest::post()
                .uri(&url)
                .insert_header(("api-key", data.admin_api_key.as_str()))
                .set_json(&request_body)
        })
    }

    fn verify_success_response(resp: &Value, _data: &MasterData) -> Self {
        let merchant_connector_id = resp.get("merchant_connector_id");
        assert_ne!(merchant_connector_id, None);
        Self
    }

    fn verify_failure_response(_response: &Value, _data: &MasterData) -> Self {
        unimplemented!();
    }

    fn update_master_data(&self, _data: &mut MasterData, _resp: &Value) {}
}

pub async fn execute_connector_create_test(
    master_data: &mut MasterData,
    server: &impl Service<
        Request,
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
    >,
) -> Option<Value> {
    let opt_test_request = ConnectorCreate::make_request_body(&master_data);
    match opt_test_request {
        Some(test_request) => {
            let connector_create_resp =
                call_and_read_body_json(&server, test_request.to_request()).await;
            ConnectorCreate::verify_success_response(&connector_create_resp, master_data)
                .update_master_data(master_data, &connector_create_resp);
            //println!("Connector Create Response {:?}",connector_create_resp);
            println!("Connector Create Test successful!");
            Some(connector_create_resp)
        }
        None => {
            println!("Skipping Connector Create Test!");
            None
        }
    }
}
