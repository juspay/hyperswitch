use actix_http::{body::MessageBody, Request};
use actix_web::{
    dev::{Service, ServiceResponse},
    test::{call_and_read_body_json, TestRequest},
};
use serde_json::value::Value;

use crate::integration::types::*;

pub struct Customer;

impl RequestBuilder for Customer {
    fn make_request_body(data: &MasterData) -> Option<TestRequest> {
        data.customers.as_ref().map(|customer_data| {
            let request_body = Value::clone(customer_data);
            TestRequest::post()
                .uri(&String::from("http://localhost:8080/customers"))
                .insert_header(("api-key", data.api_key.as_ref().unwrap().as_str()))
                .set_json(&request_body)
        })
    }

    fn verify_success_response(resp: &Value, data: &MasterData) -> Self {
        let customer_id = resp.get("customer_id");
        let req_customer_id = data
            .customers
            .as_ref()
            .and_then(|customer_req| customer_req.get("customer_id"));
        assert_ne!(customer_id, None);
        assert_eq!(customer_id, req_customer_id);
        Self
    }

    fn verify_failure_response(_response: &Value, _data: &MasterData) -> Self {
        unimplemented!();
    }

    fn update_master_data(&self, _data: &mut MasterData, _resp: &Value) {}
}

pub async fn execute_customer_create_test(
    master_data: &mut MasterData,
    server: &impl Service<
        Request,
        Response = ServiceResponse<impl MessageBody>,
        Error = actix_web::Error,
    >,
) -> Option<Value> {
    let opt_test_request = Customer::make_request_body(&master_data);
    match opt_test_request {
        Some(test_request) => {
            let customer_create_resp =
                call_and_read_body_json(&server, test_request.to_request()).await;
            Customer::verify_success_response(&customer_create_resp, master_data)
                .update_master_data(master_data, &customer_create_resp);
            //println!("Customer Create Response : {:?}",customer_create_resp);
            println!("Customer Create Test successful!");
            Some(customer_create_resp)
        }
        None => {
            println!("Skipping Customer Create Test!");
            None
        }
    }
}
