## DESCRIPTION

> I WANT TO IMPLMEMENT THIS FEATURE. WE ARE ONLY WORKING ON #9158 AND NOTHING ELSE WE ARE ONLY WORKING ON ADDING INTEGRITY CHECK TO CELERO AND NOTHING ELSE. PLEASE READ THE BELOW DESCRIPTION TO UNDERSTAND WHAT ARE WORKING ON.

### [FEATURE] : [CELERO] Add Integrity Check Support for Authorize, PSync, Refund and RSync Flows #9158

Feature Description/Summary
Integrity check is a scenario where there is a discrepancy between the amount sent in the request and the amount received from the connector, which is checked during response handling.

Context
Integrity checks in a payments flow are critical for ensuring data consistency, correctness, and security when dealing with amounts between Hyperswitch and connectors like Adyen, Stripe, Razorpay, etc.

Starter Tasks
In the handle_response function of Authorize/PSync/Refund/RSync, you will have to call the respective functions - get_authorise_integrity_object for Authorize, get_sync_integrity_object for Payments Sync, get_refund_integrity_object for Refund and RSync and get_capture_integrity_object for Capture.
You would have to call these functions from crates/hyperswitch_connectors/src/utils.rs.
These functions expect amount_convertor, amount and currency.
You can take a look at this PR for reference.
Implementation Hints
You can go to crates/hyperswitch_connectors/src/connectors/celero.rs and call the respective integrity check function as stated above in the handle_response function
Acceptance Criteria

Request and Response body added for each of the flows where integrity check is applied.

All the required GitHub checks passing

Formatted the code using cargo +nightly fmt --all
How to Test it
Hardcode the amount or currency field that is being passed to the connector different from the one you are passing in the request body
This way you would be able to reproduce the integrity checks error message while testing.

## Reference:

> THE BELOW IS ONLY REFERENCE. WE ARE NOT WORKING ON THE BELOW THINGS.

1. [CONTRIBUTION](./docs/CONTRIBUTING.md)
2. [Local system setup](./docs/try_local_system.md)
3. [PULL REQUEST TEMPLATE](./.github/PULL_REQUEST_TEMPLATE.md)
4. [CELERO](https://celerocommerce.com/developers/)

---

## My machine/development environment

I am using NIX to for the development. I will be building/testing everything on nix (Nix is on WSL2, which is on my windows 11 machine).

---

# REFERENCE PR 

> USE THIS PR AS REFERENCE ON HOW TO WRITE CODE AND HOW TO TEST THE CODE. THIS IS ONLY REFERENCE. WE ARE NOT WORKING ON THIS PR.

feat(connector): [FISERV] Added Integrity Check support for all Payment & Refund Flows #8075

## Type of Change

*   [ ]  Bugfix
*   [x]  New feature
*   [ ]  Enhancement
*   [ ]  Refactoring
*   [ ]  Dependency updates
*   [ ]  Documentation
*   [ ]  CI/CD

## Description

Populated network\_advice\_code, network\_decline\_code and network\_error\_response in ErrorResponse. Also added integrity check support for Authorize, Capture, Refund, PSync and RSync flows.

What is an integrity check?  
A scenario where there is a discrepancy between the amount sent in the request and the amount received from the connector, which is checked during response handling.  
[developer.fiserv.com/product/CommerceHub/api?type=post&path=/payments/v1/charges&branch=main&version=1.25.0400](https://developer.fiserv.com/product/CommerceHub/api/?type=post&path=/payments/v1/charges&branch=main&version=1.25.0400)

### Additional Changes

*   [ ]  This PR modifies the API contract
*   [ ]  This PR modifies the database schema
*   [ ]  This PR modifies application configuration/environment variables

## Motivation and Context

## How did you test it?

For Integrity Checks

Case 1: Automatic Capture

cURL:

```
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_8BoyoSPgaCnEZ7hPrLeVLSyEpsxM1d95WnoRRlgyLJfhUD3Rt9vaYiErFlhICRJy' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data-raw '{
    "amount": 651200,
    "currency": "USD",
    "confirm": true,
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            
            "card_number": "4147463011110083",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "123"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}'
```

Response:

```
{
    "error": {
        "type": "api",
        "message": "Integrity Check Failed! as data mismatched for amount expected 651200 but found 651300",
        "code": "IE_00",
        "connector_transaction_id": "CHG0165dcaf19383fef3b997c70f17e33f882"
    }
}
```

We hardcoded the amount at the connector level to a value greater than the one sent in the request. This is verified at response time, causing a discrepancy between the amount passed in the request and the amount passed to the connector, which triggers the integrity check.

2.  Manual Capture:

cURL :

```
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_8BoyoSPgaCnEZ7hPrLeVLSyEpsxM1d95WnoRRlgyLJfhUD3Rt9vaYiErFlhICRJy' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data-raw '{
    "amount": 651200,
    "currency": "USD",
    "confirm": true,
    "capture_method": "manual",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            
            "card_number": "4147463011110083",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "123"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}'
```

Response:

```
{
    "error": {
        "type": "api",
        "message": "Integrity Check Failed! as data mismatched for amount expected 651200 but found 651300",
        "code": "IE_00",
        "connector_transaction_id": "CHG01e6b342a643cf24269162a523ae884b49"
    }
}
```

Reason: We hardcoded the amount in the code to an amount which is more than the one being sent in the connector request.

3.  Refund

First, do a payments create( a successful one) and donot hardcode anything.

cURL :

```
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_8BoyoSPgaCnEZ7hPrLeVLSyEpsxM1d95WnoRRlgyLJfhUD3Rt9vaYiErFlhICRJy' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data-raw '{
    "amount": 651200,
    "currency": "USD",
    "confirm": true,
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            
            "card_number": "4147463011110083",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "123"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}'
```

Response of Payments - Create :

```
{
    "payment_id": "pay_9YN7bLpjbmbFkA1laXqC",
    "merchant_id": "merchant_1747656283",
    "status": "succeeded",
    "amount": 651200,
    "net_amount": 651200,
    "shipping_cost": null,
    "amount_capturable": 0,
    "amount_received": 651200,
    "connector": "fiserv",
    "client_secret": "pay_9YN7bLpjbmbFkA1laXqC_secret_onbFfEy4CWh2pd3jAdTt",
    "created": "2025-05-19T20:59:12.836Z",
    "currency": "USD",
    "customer_id": "First_Customer",
    "customer": {
        "id": "First_Customer",
        "name": "John Doe",
        "email": null,
        "phone": null,
        "phone_country_code": null
    },
    "description": null,
    "refunds": null,
    "disputes": null,
    "mandate_id": null,
    "mandate_data": null,
    "setup_future_usage": null,
    "off_session": null,
    "capture_on": null,
    "capture_method": "automatic",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "last4": "0083",
            "card_type": null,
            "card_network": null,
            "card_issuer": null,
            "card_issuing_country": null,
            "card_isin": "414746",
            "card_extended_bin": null,
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "payment_checks": null,
            "authentication_data": null
        },
        "billing": null
    },
    "payment_token": null,
    "shipping": null,
    "billing": {
        "address": null,
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    },
    "order_details": null,
    "email": null,
    "name": "John Doe",
    "phone": null,
    "return_url": "https://google.com/",
    "authentication_type": "three_ds",
    "statement_descriptor_name": null,
    "statement_descriptor_suffix": null,
    "next_action": null,
    "cancellation_reason": null,
    "error_code": null,
    "error_message": null,
    "unified_code": null,
    "unified_message": null,
    "payment_experience": null,
    "payment_method_type": "credit",
    "connector_label": null,
    "business_country": null,
    "business_label": "default",
    "business_sub_label": null,
    "allowed_payment_method_types": null,
    "ephemeral_key": {
        "customer_id": "First_Customer",
        "created_at": 1747688352,
        "expires": 1747691952,
        "secret": "epk_f6e6cc342a9147eb91b4d8144bc28958"
    },
    "manual_retry_allowed": false,
    "connector_transaction_id": "964bbc9b9e494be8bf575977cf2582b5",
    "frm_message": null,
    "metadata": null,
    "connector_metadata": null,
    "feature_metadata": null,
    "reference_id": "CHG01012441523bde085d4ec1a37b6c72c768",
    "payment_link": null,
    "profile_id": "pro_kg6n3seduV6leLIAxPwL",
    "surcharge_details": null,
    "attempt_count": 1,
    "merchant_decision": null,
    "merchant_connector_id": "mca_EZ1kF48ElbkcXo52Eu0z",
    "incremental_authorization_allowed": null,
    "authorization_count": null,
    "incremental_authorizations": null,
    "external_authentication_details": null,
    "external_3ds_authentication_attempted": false,
    "expires_on": "2025-05-19T21:14:12.836Z",
    "fingerprint": null,
    "browser_info": null,
    "payment_method_id": null,
    "payment_method_status": null,
    "updated": "2025-05-19T20:59:14.534Z",
    "split_payments": null,
    "frm_metadata": null,
    "extended_authorization_applied": null,
    "capture_before": null,
    "merchant_order_reference_id": null,
    "order_tax_amount": null,
    "connector_mandate_id": null,
    "card_discovery": "manual",
    "force_3ds_challenge": false,
    "force_3ds_challenge_trigger": false,
    "issuer_error_code": null,
    "issuer_error_message": null,
    "is_iframe_redirection_enabled": null
}
```

Now attempt a Refund with this payment\_id. In code I have hardcoded the refund amount same as the captured amount but in the request will be passing an amount which will be less than that.

Refunds - Create cURL:

```
curl --location 'http://localhost:8080/refunds' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_8BoyoSPgaCnEZ7hPrLeVLSyEpsxM1d95WnoRRlgyLJfhUD3Rt9vaYiErFlhICRJy' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data '{
    "payment_id": "pay_kshnzcowY4iP7ZhiBArY",
    "amount": 651100,
    "reason": "Customer returned product",
    "refund_type": "instant",
    "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
    }
}'
```

Response :

```
{
    "refund_id": "ref_qvFT2TJmnx0irSvV5MTX",
    "payment_id": "pay_kshnzcowY4iP7ZhiBArY",
    "amount": 651100,
    "currency": "USD",
    "status": "review",
    "reason": "Customer returned product",
    "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
    },
    "error_message": "Integrity Check Failed! as data mismatched for fields refund_amount expected 651100 but found 651200",
    "error_code": "IE",
    "unified_code": null,
    "unified_message": null,
    "created_at": "2025-05-19T21:12:21.095Z",
    "updated_at": "2025-05-19T21:12:22.577Z",
    "connector": "fiserv",
    "profile_id": "pro_kg6n3seduV6leLIAxPwL",
    "merchant_connector_id": "mca_EZ1kF48ElbkcXo52Eu0z",
    "split_refunds": null,
    "issuer_error_code": null,
    "issuer_error_message": null
}
```

## Checklist

*   [x]  I formatted the code `cargo +nightly fmt --all`
*   [x]  I addressed lints thrown by `cargo clippy`
*   [x]  I reviewed the submitted code
*   [ ]  I added unit tests for my changes where possible


---

# REFERENCE PR 2

> USE THIS PR AS REFERENCE ON HOW TO WRITE CODE AND HOW TO TEST THE CODE. THIS IS ONLY REFERENCE. WE ARE NOT WORKING ON THIS PR.

feat(connector): [XENDIT] Added Integrity Check for Authorize, Capture, Refund & RSync flows #8049

## Type of Change

*   [ ]  Bugfix
*   [x]  New feature
*   [ ]  Enhancement
*   [ ]  Refactoring
*   [ ]  Dependency updates
*   [ ]  Documentation
*   [ ]  CI/CD

## Description

In this PR, we have added integrity checks for Authorize, Capture, Refund and RSync flows for Xendit Connector  
Also, moved the integrity functions from `crates/router/src/connector/utils.rs` to `crates/hyperswitch_connectors/src/utils.rs`. Thereby also changing the imports in `Stripe.rs`.

What is an integrity check?  
A scenario where there is a discrepancy between the amount sent in the request and the amount received from the connector, which is checked during response handling.

### Additional Changes

*   [ ]  This PR modifies the API contract
*   [ ]  This PR modifies the database schema
*   [ ]  This PR modifies application configuration/environment variables

## Motivation and Context

This Pr holds the payment in a non-terminal state incase there is any data discrepancy in the above flows.

## How did you test it?

Case 1: AUTOMATIC Capture

Do a Payments - Create  
Request:

```
{
    "amount": 651200,
    "currency": "IDR",
    "confirm": true,
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            "card_number": "4000000000001091",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "124"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}
```

Now after completing the 3DS authentication in the browser, Do a PSync on the payment\_id you received in the response

PSync cURL :

```
curl --location 'http://localhost:8080/payments/pay_ZiImJRPcy4lBxikZXPZW?force_sync=true&expand_captures=true&expand_attempts=true' \
--header 'Accept: application/json' \
--header 'api-key: dev_iaxB63qB728km9UmcoRYaRDwLvz0ZMN9sEvIEZ3shfdxdqK0qwESalvaliyfhzVL' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7'
```

Response:

```
{
    "error": {
        "type": "api",
        "message": "Integrity Check Failed! as data mismatched for amount expected 651200 but found 651300",
        "code": "IE_00",
        "connector_transaction_id": "pr-dbc3c6aa-6734-47c6-b198-c88ca69173a0"
    }
}
```

Why this behaviour?  
I have hardcoded the amount in the code and the amount i hardcoded is more than the amount in the request causing the payment to fail.

Case 2: MANUAL Capture

Do a payments create

Request:

```
{
    "amount": 651200,
    "currency": "IDR",
    "confirm": true,
    "capture_method": "manual",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            "card_number": "4000000000001091",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "124"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}
```

Now do a force PSync after the 3DS authentication is done in the web browser and you got the payment\_id

PSync cURL :

```
curl --location 'http://localhost:8080/payments/pay_TsnuX8XgHwIdYQWYBEIO?force_sync=true&expand_captures=true&expand_attempts=true' \
--header 'Accept: application/json' \
--header 'api-key: dev_iaxB63qB728km9UmcoRYaRDwLvz0ZMN9sEvIEZ3shfdxdqK0qwESalvaliyfhzVL' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7'
```

Response :

```
{
    "error": {
        "type": "api",
        "message": "Integrity Check Failed! as data mismatched for amount expected 651200 but found 651300",
        "code": "IE_00",
        "connector_transaction_id": "pr-a452cba4-c24f-43a1-9e36-9e7bd066e448"
    }
}
```

Why this behaviour?  
I have hardcoded the amount in code for it to fail

Case 3: Refunds

Do a payments create and this time donot hardcode anything in the code for Payments Create

```
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_iaxB63qB728km9UmcoRYaRDwLvz0ZMN9sEvIEZ3shfdxdqK0qwESalvaliyfhzVL' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data-raw '{
    "amount": 651200,
    "currency": "IDR",
    "confirm": true,
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "customer_id": "First_Customer",
    "name": "John Doe",
    "authentication_type": "three_ds",
    "return_url": "https://google.com",
    "payment_method": "card",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            "card_number": "4000000000001091",
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "card_cvc": "124"
        }
    },
    "billing": {
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    }
}'
```

Now do a force PSync after the 3DS authentication is completed in the web browser and you have the payment\_id

PSync cURL :

```
curl --location 'http://localhost:8080/payments/pay_PUy3X0USglGXRhUZwa6y?force_sync=true&expand_captures=true&expand_attempts=true' \
--header 'Accept: application/json' \
--header 'api-key: dev_iaxB63qB728km9UmcoRYaRDwLvz0ZMN9sEvIEZ3shfdxdqK0qwESalvaliyfhzVL' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7'
```

Response:

```
{
    "payment_id": "pay_PUy3X0USglGXRhUZwa6y",
    "merchant_id": "merchant_1747372268",
    "status": "succeeded",
    "amount": 651200,
    "net_amount": 651200,
    "shipping_cost": null,
    "amount_capturable": 0,
    "amount_received": 651200,
    "connector": "xendit",
    "client_secret": "pay_PUy3X0USglGXRhUZwa6y_secret_nMwSH8e7SAzjh0Yq253O",
    "created": "2025-05-16T05:59:21.548Z",
    "currency": "IDR",
    "customer_id": "First_Customer",
    "customer": {
        "id": "First_Customer",
        "name": "John Doe",
        "email": null,
        "phone": null,
        "phone_country_code": null
    },
    "description": null,
    "refunds": null,
    "disputes": null,
    "attempts": [
        {
            "attempt_id": "pay_PUy3X0USglGXRhUZwa6y_1",
            "status": "pending",
            "amount": 651200,
            "order_tax_amount": null,
            "currency": "IDR",
            "connector": "xendit",
            "error_message": null,
            "payment_method": "card",
            "connector_transaction_id": "pr-6899118b-61ed-4eaa-85ed-c19ebafa0517",
            "capture_method": "automatic",
            "authentication_type": "three_ds",
            "created_at": "2025-05-16T05:59:21.548Z",
            "modified_at": "2025-05-16T05:59:36.701Z",
            "cancellation_reason": null,
            "mandate_id": null,
            "error_code": null,
            "payment_token": null,
            "connector_metadata": null,
            "payment_experience": null,
            "payment_method_type": "credit",
            "reference_id": "dd99b9db-2807-4393-a160-18ac5e2598b3",
            "unified_code": null,
            "unified_message": null,
            "client_source": null,
            "client_version": null
        }
    ],
    "mandate_id": null,
    "mandate_data": null,
    "setup_future_usage": null,
    "off_session": null,
    "capture_on": null,
    "capture_method": "automatic",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "last4": "1091",
            "card_type": null,
            "card_network": null,
            "card_issuer": null,
            "card_issuing_country": null,
            "card_isin": "400000",
            "card_extended_bin": null,
            "card_exp_month": "12",
            "card_exp_year": "27",
            "card_holder_name": "joseph Doe",
            "payment_checks": null,
            "authentication_data": null
        },
        "billing": null
    },
    "payment_token": null,
    "shipping": null,
    "billing": {
        "address": null,
        "phone": {
            "number": "8056594427",
            "country_code": "+91"
        },
        "email": "guest@example.com"
    },
    "order_details": null,
    "email": null,
    "name": "John Doe",
    "phone": null,
    "return_url": "https://google.com/",
    "authentication_type": "three_ds",
    "statement_descriptor_name": null,
    "statement_descriptor_suffix": null,
    "next_action": null,
    "cancellation_reason": null,
    "error_code": null,
    "error_message": null,
    "unified_code": null,
    "unified_message": null,
    "payment_experience": null,
    "payment_method_type": "credit",
    "connector_label": null,
    "business_country": null,
    "business_label": "default",
    "business_sub_label": null,
    "allowed_payment_method_types": null,
    "ephemeral_key": null,
    "manual_retry_allowed": false,
    "connector_transaction_id": "pr-6899118b-61ed-4eaa-85ed-c19ebafa0517",
    "frm_message": null,
    "metadata": null,
    "connector_metadata": null,
    "feature_metadata": null,
    "reference_id": "dd99b9db-2807-4393-a160-18ac5e2598b3",
    "payment_link": null,
    "profile_id": "pro_p9XcyZ68jQWhZVhjDfvA",
    "surcharge_details": null,
    "attempt_count": 1,
    "merchant_decision": null,
    "merchant_connector_id": "mca_Yrd0AB3I63wrPAxClDXT",
    "incremental_authorization_allowed": null,
    "authorization_count": null,
    "incremental_authorizations": null,
    "external_authentication_details": null,
    "external_3ds_authentication_attempted": false,
    "expires_on": "2025-05-16T06:14:21.548Z",
    "fingerprint": null,
    "browser_info": null,
    "payment_method_id": null,
    "payment_method_status": null,
    "updated": "2025-05-16T05:59:45.207Z",
    "split_payments": null,
    "frm_metadata": null,
    "extended_authorization_applied": null,
    "capture_before": null,
    "merchant_order_reference_id": null,
    "order_tax_amount": null,
    "connector_mandate_id": null,
    "card_discovery": "manual",
    "force_3ds_challenge": false,
    "force_3ds_challenge_trigger": false,
    "issuer_error_code": null,
    "issuer_error_message": null
}
```

Now do a Refunds Create and in postman send an amount which is lower than the Captured Amount

Refunds Create cURL :

```
curl --location 'http://localhost:8080/refunds' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_iaxB63qB728km9UmcoRYaRDwLvz0ZMN9sEvIEZ3shfdxdqK0qwESalvaliyfhzVL' \
--header 'Cookie: PHPSESSID=0b47db9d7de94c37b6b272087a9f2fa7' \
--data '{
    "payment_id": "pay_PUy3X0USglGXRhUZwa6y",
    
    "amount": 651100,
    "reason": "Customer returned product",
    "refund_type": "instant",
    "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
    }
    
    
    
    
    
}'
```

Response:

```
{
    "refund_id": "ref_UL8G03huxgVfg85dBBpj",
    "payment_id": "pay_PUy3X0USglGXRhUZwa6y",
    "amount": 651100,
    "currency": "IDR",
    "status": "review",
    "reason": "Customer returned product",
    "metadata": {
        "udf1": "value1",
        "new_customer": "true",
        "login_date": "2019-09-10T10:11:12Z"
    },
    "error_message": "Integrity Check Failed! as data mismatched for fields refund_amount expected 651100 but found 651200",
    "error_code": "IE",
    "unified_code": null,
    "unified_message": null,
    "created_at": "2025-05-16T05:59:52.764Z",
    "updated_at": "2025-05-16T05:59:54.555Z",
    "connector": "xendit",
    "profile_id": "pro_p9XcyZ68jQWhZVhjDfvA",
    "merchant_connector_id": "mca_Yrd0AB3I63wrPAxClDXT",
    "split_refunds": null,
    "issuer_error_code": null,
    "issuer_error_message": null
}
```

Why this behaviour?  
I have hardcoded the amount to be refunded in the code as the same as the captured amount but in postman the amount i have sent is less than that, causing the payments to fail (intentionally)

## Checklist

*   [x]  I formatted the code `cargo +nightly fmt --all`
*   [x]  I addressed lints thrown by `cargo clippy`
*   [ ]  I reviewed the submitted code
*   [ ]  I added unit tests for my changes where possible