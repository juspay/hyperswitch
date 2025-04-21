# hyperswitch.PaymentsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancel_a_payment**](PaymentsApi.md#cancel_a_payment) | **POST** /payments/{payment_id}/cancel | Payments - Cancel
[**capture_a_payment**](PaymentsApi.md#capture_a_payment) | **POST** /payments/{payment_id}/capture | Payments - Capture
[**complete_authorize_a_payment**](PaymentsApi.md#complete_authorize_a_payment) | **POST** /{payment_id}/complete_authorize | Payments - Complete Authorize
[**confirm_a_payment**](PaymentsApi.md#confirm_a_payment) | **POST** /payments/{payment_id}/confirm | Payments - Confirm
[**create_a_payment**](PaymentsApi.md#create_a_payment) | **POST** /payments | Payments - Create
[**create_post_session_tokens_for_a_payment**](PaymentsApi.md#create_post_session_tokens_for_a_payment) | **POST** /payments/{payment_id}/post_session_tokens | Payments - Post Session Tokens
[**create_session_tokens_for_a_payment**](PaymentsApi.md#create_session_tokens_for_a_payment) | **POST** /payments/session_tokens | Payments - Session token
[**increment_authorized_amount_for_a_payment**](PaymentsApi.md#increment_authorized_amount_for_a_payment) | **POST** /payments/{payment_id}/incremental_authorization | Payments - Incremental Authorization
[**initiate_external_authentication_for_a_payment**](PaymentsApi.md#initiate_external_authentication_for_a_payment) | **POST** /payments/{payment_id}/3ds/authentication | Payments - External 3DS Authentication
[**list_all_payments**](PaymentsApi.md#list_all_payments) | **GET** /payments/list | Payments - List
[**retrieve_a_payment**](PaymentsApi.md#retrieve_a_payment) | **GET** /payments/{payment_id} | Payments - Retrieve
[**retrieve_a_payment_link**](PaymentsApi.md#retrieve_a_payment_link) | **GET** /payment_link/{payment_link_id} | Payments Link - Retrieve
[**update_a_payment**](PaymentsApi.md#update_a_payment) | **POST** /payments/{payment_id} | Payments - Update
[**update_metadata_for_a_payment**](PaymentsApi.md#update_metadata_for_a_payment) | **POST** /payments/{payment_id}/update_metadata | Payments - Update Metadata


# **cancel_a_payment**
> cancel_a_payment(payment_id, payments_cancel_request)

Payments - Cancel

A Payment could can be cancelled when it is in one of these statuses: `requires_payment_method`, `requires_capture`, `requires_confirmation`, `requires_customer_action`.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_cancel_request import PaymentsCancelRequest
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_cancel_request = {"cancellation_reason":"requested_by_customer"} # PaymentsCancelRequest | 

    try:
        # Payments - Cancel
        api_instance.cancel_a_payment(payment_id, payments_cancel_request)
    except Exception as e:
        print("Exception when calling PaymentsApi->cancel_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_cancel_request** | [**PaymentsCancelRequest**](PaymentsCancelRequest.md)|  | 

### Return type

void (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: Not defined

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment canceled |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **capture_a_payment**
> PaymentsResponse capture_a_payment(payment_id, payments_capture_request)

Payments - Capture

To capture the funds for an uncaptured payment

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_capture_request import PaymentsCaptureRequest
from hyperswitch.models.payments_response import PaymentsResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_capture_request = {"amount_to_capture":654} # PaymentsCaptureRequest | 

    try:
        # Payments - Capture
        api_response = api_instance.capture_a_payment(payment_id, payments_capture_request)
        print("The response of PaymentsApi->capture_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->capture_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_capture_request** | [**PaymentsCaptureRequest**](PaymentsCaptureRequest.md)|  | 

### Return type

[**PaymentsResponse**](PaymentsResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment captured |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **complete_authorize_a_payment**
> PaymentsResponse complete_authorize_a_payment(payment_id, payments_complete_authorize_request)

Payments - Complete Authorize

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.payments_complete_authorize_request import PaymentsCompleteAuthorizeRequest
from hyperswitch.models.payments_response import PaymentsResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_complete_authorize_request = hyperswitch.PaymentsCompleteAuthorizeRequest() # PaymentsCompleteAuthorizeRequest | 

    try:
        # Payments - Complete Authorize
        api_response = api_instance.complete_authorize_a_payment(payment_id, payments_complete_authorize_request)
        print("The response of PaymentsApi->complete_authorize_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->complete_authorize_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_complete_authorize_request** | [**PaymentsCompleteAuthorizeRequest**](PaymentsCompleteAuthorizeRequest.md)|  | 

### Return type

[**PaymentsResponse**](PaymentsResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payments Complete Authorize Success |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **confirm_a_payment**
> PaymentsCreateResponseOpenApi confirm_a_payment(payment_id, payments_confirm_request)

Payments - Confirm

**Use this API to confirm the payment and forward the payment to the payment processor.**

Alternatively you can confirm the payment within the *Payments/Create* API by setting `confirm=true`. After confirmation, the payment could either:

1. fail with `failed` status or

2. transition to a `requires_customer_action` status with a `next_action` block or

3. succeed with either `succeeded` in case of automatic capture or `requires_capture` in case of manual capture

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_confirm_request import PaymentsConfirmRequest
from hyperswitch.models.payments_create_response_open_api import PaymentsCreateResponseOpenApi
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_confirm_request = {"customer_acceptance":{"acceptance_type":"online","accepted_at":"1963-05-03T04:07:52.723Z","online":{"ip_address":"127.0.0.1","user_agent":"amet irure esse"}},"payment_method":"card","payment_method_data":{"card":{"card_cvc":"123","card_exp_month":"10","card_exp_year":"25","card_holder_name":"joseph Doe","card_number":"4242424242424242"}},"payment_method_type":"credit"} # PaymentsConfirmRequest | 

    try:
        # Payments - Confirm
        api_response = api_instance.confirm_a_payment(payment_id, payments_confirm_request)
        print("The response of PaymentsApi->confirm_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->confirm_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_confirm_request** | [**PaymentsConfirmRequest**](PaymentsConfirmRequest.md)|  | 

### Return type

[**PaymentsCreateResponseOpenApi**](PaymentsCreateResponseOpenApi.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment confirmed |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_a_payment**
> PaymentsCreateResponseOpenApi create_a_payment(payments_create_request)

Payments - Create

**Creates a payment object when amount and currency are passed.**

This API is also used to create a mandate by passing the `mandate_object`.

Depending on the user journey you wish to achieve, you may opt to complete all the steps in a single request **by attaching a payment method, setting `confirm=true` and `capture_method = automatic`** in the *Payments/Create API* request.

Otherwise, To completely process a payment you will have to **create a payment, attach a payment method, confirm and capture funds**. For that you could use the following sequence of API requests -

1. Payments - Create

2. Payments - Update

3. Payments - Confirm

4. Payments - Capture.

You will require the 'API - Key' from the Hyperswitch dashboard to make the first call, and use the 'client secret' returned in this API along with your 'publishable key' to make subsequent API calls from your client.

This page lists the various combinations in which the Payments - Create API can be used and the details about the various fields in the requests and responses.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_create_request import PaymentsCreateRequest
from hyperswitch.models.payments_create_response_open_api import PaymentsCreateResponseOpenApi
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payments_create_request = {"amount":6540,"authentication_type":"three_ds","currency":"USD"} # PaymentsCreateRequest | 

    try:
        # Payments - Create
        api_response = api_instance.create_a_payment(payments_create_request)
        print("The response of PaymentsApi->create_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->create_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payments_create_request** | [**PaymentsCreateRequest**](PaymentsCreateRequest.md)|  | 

### Return type

[**PaymentsCreateResponseOpenApi**](PaymentsCreateResponseOpenApi.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment created |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_post_session_tokens_for_a_payment**
> PaymentsPostSessionTokensResponse create_post_session_tokens_for_a_payment(payments_post_session_tokens_request)

Payments - Post Session Tokens

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.payments_post_session_tokens_request import PaymentsPostSessionTokensRequest
from hyperswitch.models.payments_post_session_tokens_response import PaymentsPostSessionTokensResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payments_post_session_tokens_request = hyperswitch.PaymentsPostSessionTokensRequest() # PaymentsPostSessionTokensRequest | 

    try:
        # Payments - Post Session Tokens
        api_response = api_instance.create_post_session_tokens_for_a_payment(payments_post_session_tokens_request)
        print("The response of PaymentsApi->create_post_session_tokens_for_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->create_post_session_tokens_for_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payments_post_session_tokens_request** | [**PaymentsPostSessionTokensRequest**](PaymentsPostSessionTokensRequest.md)|  | 

### Return type

[**PaymentsPostSessionTokensResponse**](PaymentsPostSessionTokensResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Post Session Token is done |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_session_tokens_for_a_payment**
> PaymentsSessionResponse create_session_tokens_for_a_payment(payments_session_request)

Payments - Session token

Creates a session object or a session token for wallets like Apple Pay, Google Pay, etc. These tokens are used by Hyperswitch's SDK to initiate these wallets' SDK.

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.payments_session_request import PaymentsSessionRequest
from hyperswitch.models.payments_session_response import PaymentsSessionResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payments_session_request = hyperswitch.PaymentsSessionRequest() # PaymentsSessionRequest | 

    try:
        # Payments - Session token
        api_response = api_instance.create_session_tokens_for_a_payment(payments_session_request)
        print("The response of PaymentsApi->create_session_tokens_for_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->create_session_tokens_for_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payments_session_request** | [**PaymentsSessionRequest**](PaymentsSessionRequest.md)|  | 

### Return type

[**PaymentsSessionResponse**](PaymentsSessionResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment session object created or session token was retrieved from wallets |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **increment_authorized_amount_for_a_payment**
> PaymentsResponse increment_authorized_amount_for_a_payment(payment_id, payments_incremental_authorization_request)

Payments - Incremental Authorization

Authorized amount for a payment can be incremented if it is in status: requires_capture

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_incremental_authorization_request import PaymentsIncrementalAuthorizationRequest
from hyperswitch.models.payments_response import PaymentsResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_incremental_authorization_request = hyperswitch.PaymentsIncrementalAuthorizationRequest() # PaymentsIncrementalAuthorizationRequest | 

    try:
        # Payments - Incremental Authorization
        api_response = api_instance.increment_authorized_amount_for_a_payment(payment_id, payments_incremental_authorization_request)
        print("The response of PaymentsApi->increment_authorized_amount_for_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->increment_authorized_amount_for_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_incremental_authorization_request** | [**PaymentsIncrementalAuthorizationRequest**](PaymentsIncrementalAuthorizationRequest.md)|  | 

### Return type

[**PaymentsResponse**](PaymentsResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment authorized amount incremented |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **initiate_external_authentication_for_a_payment**
> PaymentsExternalAuthenticationResponse initiate_external_authentication_for_a_payment(payment_id, payments_external_authentication_request)

Payments - External 3DS Authentication

External 3DS Authentication is performed and returns the AuthenticationResponse

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.payments_external_authentication_request import PaymentsExternalAuthenticationRequest
from hyperswitch.models.payments_external_authentication_response import PaymentsExternalAuthenticationResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_external_authentication_request = hyperswitch.PaymentsExternalAuthenticationRequest() # PaymentsExternalAuthenticationRequest | 

    try:
        # Payments - External 3DS Authentication
        api_response = api_instance.initiate_external_authentication_for_a_payment(payment_id, payments_external_authentication_request)
        print("The response of PaymentsApi->initiate_external_authentication_for_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->initiate_external_authentication_for_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_external_authentication_request** | [**PaymentsExternalAuthenticationRequest**](PaymentsExternalAuthenticationRequest.md)|  | 

### Return type

[**PaymentsExternalAuthenticationResponse**](PaymentsExternalAuthenticationResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Authentication created |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_payments**
> List[PaymentListResponse] list_all_payments(customer_id, starting_after, ending_before, limit, created, created_lt, created_gt, created_lte, created_gte)

Payments - List

To list the *payments*

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_list_response import PaymentListResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    customer_id = 'customer_id_example' # str | The identifier for the customer
    starting_after = 'starting_after_example' # str | A cursor for use in pagination, fetch the next list after some object
    ending_before = 'ending_before_example' # str | A cursor for use in pagination, fetch the previous list before some object
    limit = 56 # int | Limit on the number of objects to return
    created = '2013-10-20T19:20:30+01:00' # datetime | The time at which payment is created
    created_lt = '2013-10-20T19:20:30+01:00' # datetime | Time less than the payment created time
    created_gt = '2013-10-20T19:20:30+01:00' # datetime | Time greater than the payment created time
    created_lte = '2013-10-20T19:20:30+01:00' # datetime | Time less than or equals to the payment created time
    created_gte = '2013-10-20T19:20:30+01:00' # datetime | Time greater than or equals to the payment created time

    try:
        # Payments - List
        api_response = api_instance.list_all_payments(customer_id, starting_after, ending_before, limit, created, created_lt, created_gt, created_lte, created_gte)
        print("The response of PaymentsApi->list_all_payments:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->list_all_payments: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The identifier for the customer | 
 **starting_after** | **str**| A cursor for use in pagination, fetch the next list after some object | 
 **ending_before** | **str**| A cursor for use in pagination, fetch the previous list before some object | 
 **limit** | **int**| Limit on the number of objects to return | 
 **created** | **datetime**| The time at which payment is created | 
 **created_lt** | **datetime**| Time less than the payment created time | 
 **created_gt** | **datetime**| Time greater than the payment created time | 
 **created_lte** | **datetime**| Time less than or equals to the payment created time | 
 **created_gte** | **datetime**| Time greater than or equals to the payment created time | 

### Return type

[**List[PaymentListResponse]**](PaymentListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Successfully retrieved a payment list |  -  |
**404** | No payments found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_payment**
> PaymentsResponse retrieve_a_payment(payment_id, payment_retrieve_body)

Payments - Retrieve

Retrieves a Payment. This API can also be used to get the status of a previously initiated payment or next action for an ongoing payment

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_retrieve_body import PaymentRetrieveBody
from hyperswitch.models.payments_response import PaymentsResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payment_retrieve_body = hyperswitch.PaymentRetrieveBody() # PaymentRetrieveBody | 

    try:
        # Payments - Retrieve
        api_response = api_instance.retrieve_a_payment(payment_id, payment_retrieve_body)
        print("The response of PaymentsApi->retrieve_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->retrieve_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payment_retrieve_body** | [**PaymentRetrieveBody**](PaymentRetrieveBody.md)|  | 

### Return type

[**PaymentsResponse**](PaymentsResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gets the payment with final status |  -  |
**404** | No payment found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_payment_link**
> RetrievePaymentLinkResponse retrieve_a_payment_link(payment_link_id, retrieve_payment_link_request)

Payments Link - Retrieve

To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.retrieve_payment_link_request import RetrievePaymentLinkRequest
from hyperswitch.models.retrieve_payment_link_response import RetrievePaymentLinkResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_link_id = 'payment_link_id_example' # str | The identifier for payment link
    retrieve_payment_link_request = hyperswitch.RetrievePaymentLinkRequest() # RetrievePaymentLinkRequest | 

    try:
        # Payments Link - Retrieve
        api_response = api_instance.retrieve_a_payment_link(payment_link_id, retrieve_payment_link_request)
        print("The response of PaymentsApi->retrieve_a_payment_link:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->retrieve_a_payment_link: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_link_id** | **str**| The identifier for payment link | 
 **retrieve_payment_link_request** | [**RetrievePaymentLinkRequest**](RetrievePaymentLinkRequest.md)|  | 

### Return type

[**RetrievePaymentLinkResponse**](RetrievePaymentLinkResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gets details regarding payment link |  -  |
**404** | No payment link found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_payment**
> PaymentsCreateResponseOpenApi update_a_payment(payment_id, payments_update_request)

Payments - Update

To update the properties of a *PaymentIntent* object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_create_response_open_api import PaymentsCreateResponseOpenApi
from hyperswitch.models.payments_update_request import PaymentsUpdateRequest
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment
    payments_update_request = {"amount":7654} # PaymentsUpdateRequest | 

    try:
        # Payments - Update
        api_response = api_instance.update_a_payment(payment_id, payments_update_request)
        print("The response of PaymentsApi->update_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->update_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 
 **payments_update_request** | [**PaymentsUpdateRequest**](PaymentsUpdateRequest.md)|  | 

### Return type

[**PaymentsCreateResponseOpenApi**](PaymentsCreateResponseOpenApi.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment updated |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_metadata_for_a_payment**
> PaymentsUpdateMetadataResponse update_metadata_for_a_payment(payments_update_metadata_request)

Payments - Update Metadata

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payments_update_metadata_request import PaymentsUpdateMetadataRequest
from hyperswitch.models.payments_update_metadata_response import PaymentsUpdateMetadataResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentsApi(api_client)
    payments_update_metadata_request = hyperswitch.PaymentsUpdateMetadataRequest() # PaymentsUpdateMetadataRequest | 

    try:
        # Payments - Update Metadata
        api_response = api_instance.update_metadata_for_a_payment(payments_update_metadata_request)
        print("The response of PaymentsApi->update_metadata_for_a_payment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentsApi->update_metadata_for_a_payment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payments_update_metadata_request** | [**PaymentsUpdateMetadataRequest**](PaymentsUpdateMetadataRequest.md)|  | 

### Return type

[**PaymentsUpdateMetadataResponse**](PaymentsUpdateMetadataResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Metadata updated successfully |  -  |
**400** | Missing mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

