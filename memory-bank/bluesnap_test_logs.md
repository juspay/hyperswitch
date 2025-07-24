# Bluesnap Connector Test Logs

- **Merchant ID:** `merchant_bluesnap_12345`
- **API Key:** `dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J`

## Card - Authorize - 200 OK (Success)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "automatic",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_exp_year": "30",
            "card_holder_name": "John",
            "card_cvc": "737"
        }
    },
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response:**
```json
{"payment_id":"pay_fjlSb5PotOSZwJuNXQZF","merchant_id":"merchant_bluesnap_12345","status":"succeeded","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":6540,"connector":"bluesnap","client_secret":"pay_fjlSb5PotOSZwJuNXQZF_secret_9dkHnmqH8mYez7etVNxu","created":"2025-07-24T10:54:15.337Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"automatic","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_6Z8mUBXxlFOBZ0qw33X0","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113795643","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113795643","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:09:15.337Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T10:55:47.044Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

## Card - Authorize - 200 OK (Failure)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "automatic",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "card_number": "4000000000000002",
            "card_exp_month": "10",
            "card_exp_year": "30",
            "card_holder_name": "John",
            "card_cvc": "737"
        }
    },
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response:**
```json
{"payment_id":"pay_q3I3Zq5gwOiKF4LA9NAw","merchant_id":"merchant_bluesnap_12345","status":"succeeded","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":6540,"connector":"bluesnap","client_secret":"pay_q3I3Zq5gwOiKF4LA9NAw_secret_V3hyxv7Vzc6OhirXTYtV","created":"2025-07-24T10:57:09.147Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"automatic","payment_method":"card","payment_method_data":{"card":{"last4":"0002","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"400000","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_UVd1XYZaMZXiFU5vLDJp","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113795649","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113795649","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:12:09.147Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T10:57:27.215Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

Failed

## Card - Authorize - 4xx Client Error

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "automatic",
    "payment_method": "card",
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response:**
```json
{"error":{"type":"invalid_request","message":"Missing required param: payment_method_data","code":"IR_04"}}
```

## Card - Capture - 200 OK (Success)

**cURL Command (Create Payment):**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "manual",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_exp_year": "30",
            "card_holder_name": "John",
            "card_cvc": "737"
        }
    },
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response (Create Payment):**
```json
{"payment_id":"pay_99crHLftSJv79wjhDAx9","merchant_id":"merchant_bluesnap_12345","status":"requires_capture","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":6540,"amount_received":null,"connector":"bluesnap","client_secret":"pay_99crHLftSJv79wjhDAx9_secret_uYAc8x9JnXudmziGK6Jr","created":"2025-07-24T11:00:26.540Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"manual","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_Lfw6bet63QfFsNAAOqhy","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113795833","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113795833","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:15:26.540Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:00:48.742Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

**cURL Command (Capture):**
```bash
curl --location 'http://localhost:8080/payments/pay_99crHLftSJv79wjhDAx9/capture' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount_to_capture": 6540
}'
```

**Response (Capture):**
```json
{"payment_id":"pay_99crHLftSJv79wjhDAx9","merchant_id":"merchant_bluesnap_12345","status":"succeeded","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":6540,"connector":"bluesnap","client_secret":"pay_99crHLftSJv79wjhDAx9_secret_uYAc8x9JnXudmziGK6Jr","created":"2025-07-24T11:00:26.540Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"manual","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_Lfw6bet63QfFsNAAOqhy","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113795833","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113795833","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:15:26.540Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:01:05.704Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

## Card - Capture - 200 OK (Failure)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments/pay_99crHLftSJv79wjhDAx9/capture' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount_to_capture": 6540
}'
```

**Response:**
```json
{"error":{"type":"invalid_request","message":"This Payment could not be captured because it has a payment.status of succeeded. The expected state is requires_capture, partially_captured_and_capturable, processing","code":"IR_14"}}
```

## Card - PSync - 200 OK (Success)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments/sync' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "payment_id": "pay_99crHLftSJv79wjhDAx9",
    "connector_transaction_id": "1113795833",
    "connector": "bluesnap"
}'
```

**Response:**
```json
{"payment_id":"pay_99crHLftSJv79wjhDAx9","merchant_id":"merchant_bluesnap_12345","status":"succeeded","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":6540,"connector":"bluesnap","client_secret":"pay_99crHLftSJv79wjhDAx9_secret_uYAc8x9JnXudmziGK6Jr","created":"2025-07-24T11:00:26.540Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"manual","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_Lfw6bet63QfFsNAAOqhy","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113795833","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113795833","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:15:26.540Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:01:05.704Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

## Card - PSync - 4xx Client Error

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments/sync' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "connector_transaction_id": "1113795833",
    "connector": "bluesnap"
}'
```

**Response:**
```json
{"error":{"type":"invalid_request","message":"Missing required param: payment_id","code":"IR_06"}}
```

## Card - Void - 200 OK (Success)

**cURL Command (Create Payment):**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "manual",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_exp_year": "30",
            "card_holder_name": "John",
            "card_cvc": "737"
        }
    },
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response (Create Payment):**
```json
{"payment_id":"pay_yVLgJp9EQx7UXzKukb9H","merchant_id":"merchant_bluesnap_12345","status":"requires_confirmation","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":6540,"amount_received":null,"connector":null,"client_secret":"pay_yVLgJp9EQx7UXzKukb9H_secret_vlLvgYd8JOXJ0kzbYZkz","created":"2025-07-24T11:19:50.880Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"manual","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_X9XLMLKqjWCKWQFqpDuL","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":null,"statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":null,"connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":null,"connector_transaction_id":null,"frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":null,"payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":null,"incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:34:50.880Z","fingerprint":null,"browser_info":{"height":600,"language":"en-US","time_zone":-330,"user_agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36","color_depth":24,"java_enabled":true,"screen_width":800,"accept_header":"application/json","java_script_enabled":true},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:19:50.894Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":null,"force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

**cURL Command (Void):**
```bash
curl --location 'http://localhost:8080/payments/pay_yVLgJp9EQx7UXzKukb9H/cancel' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{}'
```

**Response (Void):**
```json
{"payment_id":"pay_yVLgJp9EQx7UXzKukb9H","merchant_id":"merchant_bluesnap_12345","status":"cancelled","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":null,"connector":"bluesnap","client_secret":"pay_yVLgJp9EQx7UXzKukb9H_secret_vlLvgYd8JOXJ0kzbYZkz","created":"2025-07-24T11:19:50.880Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"manual","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_X9XLMLKqjWCKWQFqpDuL","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113796141","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113796141","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:34:50.880Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:21:12.677Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

## Card - Void - 200 OK (Failure)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/payments/pay_yVLgJp9EQx7UXzKukb9H/cancel' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{}'
```

**Response:**
```json
{"error":{"type":"invalid_request","message":"You cannot cancel this payment because it has status cancelled","code":"IR_16"}}
```

## Card - Refund - 200 OK (Success)

**cURL Command (Create Payment):**
```bash
curl --location 'http://localhost:8080/payments' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "amount": 6540,
    "currency": "USD",
    "capture_method": "automatic",
    "payment_method": "card",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_exp_year": "30",
            "card_holder_name": "John",
            "card_cvc": "737"
        }
    },
    "email": "john.doe@example.com",
    "billing": {
        "address": {
            "line1": "1467",
            "line2": "Harrison Street",
            "line3": "Harrison Street",
            "city": "San Francisco",
            "state": "California",
            "zip": "94107",
            "country": "US"
        },
        "phone": {
            "number": "803-456-3456",
            "country_code": "+1"
        }
    },
    "browser_info": {
        "accept_header": "application/json",
        "color_depth": 24,
        "height": 600,
        "java_enabled": true,
        "java_script_enabled": true,
        "language": "en-US",
        "screen_width": 800,
        "time_zone": -330,
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
    },
    "connector": ["bluesnap"]
}'
```

**Response (Create Payment):**
```json
{"payment_id":"pay_zakPxWl4IKdMxdw8rgTl","merchant_id":"merchant_bluesnap_12345","status":"requires_confirmation","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":6540,"amount_received":null,"connector":null,"client_secret":"pay_zakPxWl4IKdMxdw8rgTl_secret_qnm0hKvfppVde0muC8UE","created":"2025-07-24T11:28:36.863Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"automatic","payment_method":"card","payment_method_data":{"card":{"last4":"4242","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"424242","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"John","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_besF9tqUaCMkYJBHC89R","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":null,"statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":null,"connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":null,"connector_transaction_id":null,"frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":null,"payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":null,"incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:43:36.863Z","fingerprint":null,"browser_info":{"height":600,"language":"en-US","time_zone":-330,"user_agent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36","color_depth":24,"java_enabled":true,"screen_width":800,"accept_header":"application/json","java_script_enabled":true},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:28:36.877Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":null,"force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

**cURL Command (Refund):**
```bash
curl --location 'http://localhost:8080/refunds' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "payment_id": "pay_zakPxWl4IKdMxdw8rgTl",
    "amount": 6540
}'
```

**Response (Refund):**
```json
{"refund_id":"ref_sVjnm4FGGgRktwE3BOgM","payment_id":"pay_zakPxWl4IKdMxdw8rgTl","amount":6540,"currency":"USD","status":"succeeded","reason":null,"metadata":null,"error_message":null,"error_code":null,"unified_code":null,"unified_message":null,"created_at":"2025-07-24T11:29:27.632Z","updated_at":"2025-07-24T11:29:30.171Z","connector":"bluesnap","profile_id":"pro_JUQS8T69eoIbB9FKlUP7","merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","split_refunds":null,"issuer_error_code":null,"issuer_error_message":null}
```

## Card - Refund - 4xx Client Error

**cURL Command:**
```bash
curl --location 'http://localhost:8080/refunds' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{
    "payment_id": "pay_invalid",
    "amount": 6540
}'
```

**Response:**
```json
{"error":{"type":"invalid_request","message":"Payment does not exist in our records","code":"HE_02"}}
```

## Card - RSync - 200 OK (Success)

**cURL Command:**
```bash
curl --location 'http://localhost:8080/refunds/ref_sVjnm4FGGgRktwE3BOgM' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{}'
```

**Response:**
```json
{"refund_id":"ref_sVjnm4FGGgRktwE3BOgM","payment_id":"pay_zakPxWl4IKdMxdw8rgTl","amount":6540,"currency":"USD","status":"succeeded","reason":null,"metadata":null,"error_message":null,"error_code":null,"unified_code":null,"unified_message":null,"created_at":"2025-07-24T11:29:27.632Z","updated_at":"2025-07-24T12:03:19.244Z","connector":"bluesnap","profile_id":"pro_JUQS8T69eoIbB9FKlUP7","merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","split_refunds":null,"issuer_error_code":null,"issuer_error_message":null}
```

## Card - RSync - 4xx 

**cURL Command:**
```bash
curl --location 'http://localhost:8080/refunds/ref_sVjnm4FRktwE3BOgM' \
--header 'Content-Type: application/json' \
--header 'Accept: application/json' \
--header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
--data '{}'

**Response:**
```json
{"error":{"type":"invalid_request","message":"Refund does not exist in our records.","code":"HE_02"}}
```

## Card - CompleteAuthorize - 200( OK )


```bash
curl --location 'http://localhost:8080/payments' \
> --header 'Content-Type: application/json' \
> --header 'Accept: application/json' \
> --header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
> --data '{
quote>     "amount": 6540,
quote>     "currency": "USD",
quote>     "capture_method": "automatic",
quote>     "payment_method": "card",
quote>     "payment_method_data": {
quote>         "card": {
quote>             "card_number": "4000000000001091",
quote>             "card_exp_month": "10",
quote>             "card_exp_year": "30",
quote>             "card_holder_name": "Joseph",
quote>             "card_cvc": "737"
quote>         }
quote>     },
quote>     "email": "john.doe@example.com",
quote>     "billing": {
quote>         "address": {
quote>             "line1": "1467",
quote>             "line2": "Harrison Street",
quote>             "line3": "Harrison Street",
quote>             "city": "San Francisco",
quote>             "state": "California",
quote>             "zip": "94107",
quote>             "country": "US"
quote>         },
quote>         "phone": {
quote>             "number": "803-456-3456",
quote>             "country_code": "+1"
quote>         }
quote>     },
quote>     "browser_info": {
quote>         "accept_header": "application/json",
quote>         "color_depth": 24,
quote>         "height": 600,
quote>         "java_enabled": true,
quote>         "java_script_enabled": true,
quote>         "language": "en-US",
quote>         "screen_width": 800,
quote>         "time_zone": -330,
quote>         "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/106.0.0.0 Safari/537.36"
quote>     },
quote>     "connector": ["bluesnap"]
quote> }'
```

```bash
curl --location 'http://localhost:8080/payments/pay_4QXWjCBY1VqUBREgchg8/confirm' \
> --header 'Content-Type: application/json' \
> --header 'Accept: application/json' \
> --header 'api-key: dev_DKUlvEscPFvLypGSYpY1sRWggJOq2TLvMMKxSNNou52cw5MV9jif0T8wYJNIjH8J' \
> --data '{
quote>     "payment_method": "card",
quote>     "payment_method_type": "credit",
quote>     "email": "john.doe@example.com",
quote>     "payment_method_data": {
quote>         "card": {
quote>             "card_number": "4000000000001091",
quote>             "card_exp_month": "10",
quote>             "card_exp_year": "30",
quote>             "card_holder_name": "Joseph",
quote>             "card_cvc": "737"
quote>         }
quote>     }
quote> }'
```

```json
{"payment_id":"pay_4QXWjCBY1VqUBREgchg8","merchant_id":"merchant_bluesnap_12345","status":"succeeded","amount":6540,"net_amount":6540,"shipping_cost":null,"amount_capturable":0,"amount_received":6540,"connector":"bluesnap","client_secret":"pay_4QXWjCBY1VqUBREgchg8_secret_CaUx5viTawQ2vR1E8wx8","created":"2025-07-24T11:44:49.856Z","currency":"USD","customer_id":null,"customer":{"id":null,"name":null,"email":"john.doe@example.com","phone":null,"phone_country_code":null},"description":null,"refunds":null,"disputes":null,"mandate_id":null,"mandate_data":null,"setup_future_usage":null,"off_session":null,"capture_on":null,"capture_method":"automatic","payment_method":"card","payment_method_data":{"card":{"last4":"1091","card_type":null,"card_network":null,"card_issuer":null,"card_issuing_country":null,"card_isin":"400000","card_extended_bin":null,"card_exp_month":"10","card_exp_year":"30","card_holder_name":"Joseph","payment_checks":null,"authentication_data":null},"billing":null},"payment_token":"token_sSXeJfjv8NC8B2kIqy8p","shipping":null,"billing":{"address":{"city":"San Francisco","country":"US","line1":"1467","line2":"Harrison Street","line3":"Harrison Street","zip":"94107","state":"California","first_name":null,"last_name":null},"phone":{"number":"803-456-3456","country_code":"+1"},"email":null},"order_details":null,"email":null,"name":null,"phone":null,"return_url":null,"authentication_type":"no_three_ds","statement_descriptor_name":null,"statement_descriptor_suffix":null,"next_action":null,"cancellation_reason":null,"error_code":null,"error_message":null,"unified_code":null,"unified_message":null,"payment_experience":null,"payment_method_type":"credit","connector_label":null,"business_country":null,"business_label":"default","business_sub_label":null,"allowed_payment_method_types":null,"ephemeral_key":null,"manual_retry_allowed":false,"connector_transaction_id":"1113796407","frm_message":null,"metadata":null,"connector_metadata":null,"feature_metadata":null,"reference_id":"1113796407","payment_link":null,"profile_id":"pro_JUQS8T69eoIbB9FKlUP7","surcharge_details":null,"attempt_count":1,"merchant_decision":null,"merchant_connector_id":"mca_A5S5IzkBHgErgiEnk3f4","incremental_authorization_allowed":null,"authorization_count":null,"incremental_authorizations":null,"external_authentication_details":null,"external_3ds_authentication_attempted":false,"expires_on":"2025-07-24T11:59:49.856Z","fingerprint":null,"browser_info":{"os_type":null,"language":null,"time_zone":null,"ip_address":"::1","os_version":null,"user_agent":null,"color_depth":null,"device_model":null,"java_enabled":null,"screen_width":null,"accept_header":null,"screen_height":null,"accept_language":"en","java_script_enabled":null},"payment_method_id":null,"payment_method_status":null,"updated":"2025-07-24T11:45:09.865Z","split_payments":null,"frm_metadata":null,"extended_authorization_applied":null,"capture_before":null,"merchant_order_reference_id":null,"order_tax_amount":null,"connector_mandate_id":null,"card_discovery":"manual","force_3ds_challenge":false,"force_3ds_challenge_trigger":false,"issuer_error_code":null,"issuer_error_message":null,"is_iframe_redirection_enabled":null,"whole_connector_response":null}
```

