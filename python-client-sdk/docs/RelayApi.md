# hyperswitch.RelayApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**relay_request**](RelayApi.md#relay_request) | **POST** /relay | Relay - Create
[**retrieve_a_relay_details**](RelayApi.md#retrieve_a_relay_details) | **GET** /relay/{relay_id} | Relay - Retrieve


# **relay_request**
> RelayResponse relay_request(x_profile_id, x_idempotency_key, relay_request)

Relay - Create

Creates a relay request.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.relay_request import RelayRequest
from hyperswitch.models.relay_response import RelayResponse
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
    api_instance = hyperswitch.RelayApi(api_client)
    x_profile_id = 'x_profile_id_example' # str | Profile ID for authentication
    x_idempotency_key = 'x_idempotency_key_example' # str | Idempotency Key for relay request
    relay_request = {"connector_id":"mca_5apGeP94tMts6rg3U3kR","connector_resource_id":"7256228702616471803954","data":{"refund":{"amount":6540,"currency":"USD"}},"type":"refund"} # RelayRequest | 

    try:
        # Relay - Create
        api_response = api_instance.relay_request(x_profile_id, x_idempotency_key, relay_request)
        print("The response of RelayApi->relay_request:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RelayApi->relay_request: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_profile_id** | **str**| Profile ID for authentication | 
 **x_idempotency_key** | **str**| Idempotency Key for relay request | 
 **relay_request** | [**RelayRequest**](RelayRequest.md)|  | 

### Return type

[**RelayResponse**](RelayResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Relay request |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_relay_details**
> RelayResponse retrieve_a_relay_details(x_profile_id)

Relay - Retrieve

Retrieves a relay details.

### Example

* Api Key Authentication (api_key):
* Api Key Authentication (ephemeral_key):

```python
import hyperswitch
from hyperswitch.models.relay_response import RelayResponse
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

# Configure API key authorization: ephemeral_key
configuration.api_key['ephemeral_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['ephemeral_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.RelayApi(api_client)
    x_profile_id = 'x_profile_id_example' # str | Profile ID for authentication

    try:
        # Relay - Retrieve
        api_response = api_instance.retrieve_a_relay_details(x_profile_id)
        print("The response of RelayApi->retrieve_a_relay_details:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling RelayApi->retrieve_a_relay_details: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_profile_id** | **str**| Profile ID for authentication | 

### Return type

[**RelayResponse**](RelayResponse.md)

### Authorization

[api_key](../README.md#api_key), [ephemeral_key](../README.md#ephemeral_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Relay Retrieved |  -  |
**404** | Relay details was not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

