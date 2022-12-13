import http from "k6/http";
import { sleep, check } from "k6";
import { Counter } from "k6/metrics";
import { setup_merchant_apikey } from "./helper/setup.js";
import { random_string } from "./helper/misc.js";
import { readBaseline, storeResult } from "./helper/compare-result.js";

export const requests = new Counter("http_reqs");

const baseline = readBaseline("payment-confirm");

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

export default function (data) {
    let payload = {
        "amount": 6540,
        "currency": "USD",
        "confirm": true,
        "capture_method": "automatic",
        "capture_on": "2022-09-10T10:11:12Z",
        "amount_to_capture": 6540,
        "customer_id": random_string(),
        "email": "guest@example.com",
        "name": "John Doe",
        "phone": "999999999",
        "phone_country_code": "+65",
        "description": "Its my first payment request",
        "authentication_type": "no_three_ds",
        "return_url": "https://google.com",
        "payment_method": "card",
        "setup_future_usage": "on_session",
        "payment_method_data": {
            "card": {
                "card_number": "4242424242424242",
                "card_exp_month": "10",
                "card_exp_year": "35",
                "card_holder_name": "John Doe",
                "card_cvc": "123"
            }
        },
        "statement_descriptor_name": "Juspay",
        "statement_descriptor_suffix": "Router",
        "metadata": {
            "udf1": "value1",
            "new_customer": "true",
            "login_date": "2019-09-10T10:11:12Z"
        }
    };
    let res = http.post("http://router-server:8080/payments", JSON.stringify(payload), {
        "headers": {
            "Content-Type": "application/json",
            "api-key" : data.api_key
        },
    });
    check(res, {
        "confirm payment status 200": (r) => r.status === 200,
    });
}

export function handleSummary(data) {
    return storeResult("payment-confirm", baseline, data)
}
