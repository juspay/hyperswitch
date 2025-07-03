import http from "k6/http";
import { check } from "k6";

export function setup_merchant_apikey() {
    let params = {
        "headers": {
            "Content-Type": "application/json",
            "api-key" : "test_admin"
        }
    };
    let merchant_account_payload = {
        "merchant_id":`merchant_${Date.now()}`,
        "merchant_name":"NewAge Retailer",
        "merchant_details":{
            "primary_contact_person":"John Test",
            "primary_email":"JohnTest@test.com",
            "primary_phone":"sunt laborum",
            "secondary_contact_person":"John Test2",
            "secondary_email":"JohnTest2@test.com",
            "secondary_phone":"cillum do dolor id",
            "website":"www.example.com",
            "about_business":"Online Retail with a wide selection of organic products for North America",
            "address":{
                "line1":"Vivamus vitae",
                "line2":"Libero eget",
                "line3":"Cras ultrices",
                "city":"Nascetur",
                "state":"Purus",
                "zip":"010101",
                "country":"ZX"
            }
        },
        "return_url":"www.example.com/success",
        "webhook_details":{
            "webhook_version":"1.0.1",
            "webhook_username":"wh_store",
            "webhook_password":"pwd_wh@101"
        },
        "routing_algorithm": {
            "type": "single",
            "data": "checkout"
        },
        "sub_merchants_enabled":false,
        "metadata":{
            "city":"NY",
            "unit":"245"
        }
    }
    let ma_res = http.post("http://router-server:8080/accounts", JSON.stringify(merchant_account_payload), params);

    let json = ma_res.json();
    let merchant_id = json.merchant_id;
    let api_key = json.api_key;

    let connector_account_payload = {
        "connector_type":"fiz_operations",
        "connector_name":"stripe",
        "connector_account_details":{
            "auth_type":"HeaderKey",
            "api_key":"Bearer sk_test_123"
        },
        "test_mode":false,
        "disabled":false,
        "payment_methods_enabled":[
            {
                "payment_method":"wallet",
                "payment_method_types":[
                    "upi_collect",
                    "upi_intent"
                ],
                "payment_method_issuers":[
                    "labore magna ipsum",
                    "aute"
                ],
                "payment_schemes":[
                    "Discover",
                    "Discover"
                ],
                "accepted_currencies":[
                    "AED",
                    "AED"
                ],
                "accepted_countries":[
                    "in",
                    "us"
                ],
                "minimum_amount":1,
                "maximum_amount":68607706,
                "recurring_enabled":true,
                "installment_payment_enabled":true
            }
        ],
        "metadata":{
            "city":"NY",
            "unit":"245"
        }
    }
    let ca_res = http.post(`http://router-server:8080/account/${merchant_id}/connectors`, JSON.stringify(connector_account_payload), params);

    let update_merchant_account_payload = {
        "merchant_id":merchant_id,
        "merchant_name":"NewAge Retailer",
        "merchant_details":{
            "primary_contact_person":"John Test",
            "primary_email":"JohnTest@test.com",
            "primary_phone":"veniam aute officia ullamco esse",
            "secondary_contact_person":"John Test2",
            "secondary_email":"JohnTest2@test.com",
            "secondary_phone":"proident adipisicing officia nulla",
            "website":"www.example.com",
            "about_business":"Online Retail with a wide selection of organic products for North America",
            "address":{
                "line1":"Vivamus vitae",
                "line2":"Libero eget",
                "line3":"Cras ultrices",
                "city":"Nascetur",
                "state":"Purus",
                "zip":"010101",
                "country":"ZX"
            }
        },
        "return_url":"www.example.com/success",
        "webhook_details":{
            "webhook_version":"1.0.1",
            "webhook_username":"wh_store",
            "webhook_password":"pwd_wh@101"
        },
        "routing_algorithm": {
            "type": "single",
            "data": "stripe"
        },
        "metadata":{
            "city":"NY",
            "unit":"245"
        }
    }
    let uma_res = http.post(`http://router-server:8080/accounts/${merchant_id}`, JSON.stringify(update_merchant_account_payload), params);

    return { "api_key": api_key }
}
