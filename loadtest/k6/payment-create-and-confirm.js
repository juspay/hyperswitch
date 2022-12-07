import http from "k6/http";
import { sleep, check } from "k6";
import { Counter } from "k6/metrics";
import { setup_merchant_apikey } from "./helper/setup.js";
import { random_string } from "./helper/misc.js";
import { readBaseline, storeResult } from "./helper/compare-result.js";

export const requests = new Counter("http_reqs");

const baseline = readBaseline("payment-create-and-confirm");

export const options = {
    stages: [
        { duration: "10s", target: 25 },        // ramp up users to 25 in 10 seconds
        { duration: "10s", target: 25 },        // maintain 25 users for 10 seconds
        { duration: "10s", target: 0 }          // ramp down to 0 users in 10 seconds
    ],
    thresholds: {
        'http_req_duration': ['p(90) < 500'],   // 90% of requests must finish within 500ms.
    },
};

export function setup() {
    return setup_merchant_apikey();
}

export default function(data) {
    const create_payment_payload = {
        "amount": 6540,
        "currency": "USD",
        "confirm": false,
        "capture_method": "automatic",
        "capture_on": "2022-09-10T10:11:12Z",
        "amount_to_capture": 6540,
        "customer_id": random_string(),
        "description": "Its my first payment request",
        "return_url": "http://example.com/payments",
        "authentication_type": "three_ds",
        "payment_method": "card",
        "statement_descriptor_name": "Juspay",
        "statement_descriptor_suffix": "Router"
    };
    let create_payment_res = http.post("http://router-server:8080/payments", JSON.stringify(create_payment_payload), {
        "headers": {
            "Content-Type": "application/json",
            "api-key" : data.api_key
        },
    });
    check(create_payment_res, {
        "create payment status 200": (r) => r.status === 200,
    });
    const payment_id = create_payment_res.json().payment_id;
    const confirm_payment_payload = {
        "return_url": "http://example.com/payments",
        "setup_future_usage": "off_session",
        "authentication_type": "no_three_ds",
        "payment_method": "card",
        "payment_method_data": {
            "card": {
                "card_number": "4242424242424242",
                "card_exp_month": "10",
                "card_exp_year": "35",
                "card_holder_name": "John Doe",
                "card_cvc": "123"
            }
        }
    };
    let confirm_payment_res = http.post(`http://router-server:8080/payments/${payment_id}/confirm`, JSON.stringify(confirm_payment_payload), {
        "headers": {
            "Content-Type": "application/json",
            "api-key" : data.api_key
        },
    });
    check(confirm_payment_res, {
        "confirm payment status 200": (r) => r.status === 200,
    });
};

export function handleSummary(data) {
    return storeResult("payment-create-and-confirm", baseline, data)
}
