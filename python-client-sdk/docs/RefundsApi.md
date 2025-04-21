# hyperswitch.RefundsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_refund**](RefundsApi.md#create_a_refund) | **POST** /refunds | Refunds - Create
[**list_all_refunds**](RefundsApi.md#list_all_refunds) | **POST** /refunds/list | Refunds - List
[**retrieve_a_refund**](RefundsApi.md#retrieve_a_refund) | **GET** /refunds/{refund_id} | Refunds - Retrieve
[**update_a_refund**](RefundsApi.md#update_a_refund) | **POST** /refunds/{refund_id} | Refunds - Update


# **create_a_refund**
> RefundResponse create_a_refund(refund_request)

Refunds - Create

Creates a refund against an already processed payment. In case of some processors, you can even opt to refund only a partial amount multiple times until the original charge amount has been refunded

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.refund_request import RefundRequest
from hyperswitch.models.refund_response import RefundResponse
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
    api_instance = hyperswitch.RefundsApi(api_client)
    refund_request = {"amount":654,"payment_id":"{{payment_id}}","refund_type":"instant"} # RefundRequest | 

    try:
        # Refunds - Create
        api_response = api_instance.create_a_refund(refund_request)
        print("The response of RefundsApi->create_a_refund:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RefundsApi->create_a_refund: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **refund_request** | [**RefundRequest**](RefundRequest.md)|  | 

### Return type

[**RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Refund created |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_refunds**
> RefundListResponse list_all_refunds(refund_list_request)

Refunds - List

Lists all the refunds associated with the merchant, or for a specific payment if payment_id is provided

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.refund_list_request import RefundListRequest
from hyperswitch.models.refund_list_response import RefundListResponse
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
    api_instance = hyperswitch.RefundsApi(api_client)
    refund_list_request = hyperswitch.RefundListRequest() # RefundListRequest | 

    try:
        # Refunds - List
        api_response = api_instance.list_all_refunds(refund_list_request)
        print("The response of RefundsApi->list_all_refunds:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RefundsApi->list_all_refunds: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **refund_list_request** | [**RefundListRequest**](RefundListRequest.md)|  | 

### Return type

[**RefundListResponse**](RefundListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of refunds |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_refund**
> RefundResponse retrieve_a_refund(refund_id)

Refunds - Retrieve

Retrieves a Refund. This may be used to get the status of a previously initiated refund

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.refund_response import RefundResponse
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
    api_instance = hyperswitch.RefundsApi(api_client)
    refund_id = 'refund_id_example' # str | The identifier for refund

    try:
        # Refunds - Retrieve
        api_response = api_instance.retrieve_a_refund(refund_id)
        print("The response of RefundsApi->retrieve_a_refund:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RefundsApi->retrieve_a_refund: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **refund_id** | **str**| The identifier for refund | 

### Return type

[**RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Refund retrieved |  -  |
**404** | Refund does not exist in our records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_refund**
> RefundResponse update_a_refund(refund_id, refund_update_request)

Refunds - Update

Updates the properties of a Refund object. This API can be used to attach a reason for the refund or metadata fields

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.refund_response import RefundResponse
from hyperswitch.models.refund_update_request import RefundUpdateRequest
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
    api_instance = hyperswitch.RefundsApi(api_client)
    refund_id = 'refund_id_example' # str | The identifier for refund
    refund_update_request = {"reason":"Paid by mistake"} # RefundUpdateRequest | 

    try:
        # Refunds - Update
        api_response = api_instance.update_a_refund(refund_id, refund_update_request)
        print("The response of RefundsApi->update_a_refund:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RefundsApi->update_a_refund: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **refund_id** | **str**| The identifier for refund | 
 **refund_update_request** | [**RefundUpdateRequest**](RefundUpdateRequest.md)|  | 

### Return type

[**RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Refund updated |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

