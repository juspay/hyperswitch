# hyperswitch.GsmApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_gsm_rule**](GsmApi.md#create_gsm_rule) | **POST** /gsm | Gsm - Create
[**delete_gsm_rule**](GsmApi.md#delete_gsm_rule) | **POST** /gsm/delete | Gsm - Delete
[**retrieve_gsm_rule**](GsmApi.md#retrieve_gsm_rule) | **POST** /gsm/get | Gsm - Get
[**update_gsm_rule**](GsmApi.md#update_gsm_rule) | **POST** /gsm/update | Gsm - Update


# **create_gsm_rule**
> GsmResponse create_gsm_rule(gsm_create_request)

Gsm - Create

Creates a GSM (Global Status Mapping) Rule. A GSM rule is used to map a connector's error message/error code combination during a particular payments flow/sub-flow to Hyperswitch's unified status/error code/error message combination. It is also used to decide the next action in the flow - retry/requeue/do_default

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.gsm_create_request import GsmCreateRequest
from hyperswitch.models.gsm_response import GsmResponse
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

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.GsmApi(api_client)
    gsm_create_request = hyperswitch.GsmCreateRequest() # GsmCreateRequest | 

    try:
        # Gsm - Create
        api_response = api_instance.create_gsm_rule(gsm_create_request)
        print("The response of GsmApi->create_gsm_rule:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling GsmApi->create_gsm_rule: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **gsm_create_request** | [**GsmCreateRequest**](GsmCreateRequest.md)|  | 

### Return type

[**GsmResponse**](GsmResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gsm created |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_gsm_rule**
> GsmDeleteResponse delete_gsm_rule(gsm_delete_request)

Gsm - Delete

Deletes a Gsm Rule

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.gsm_delete_request import GsmDeleteRequest
from hyperswitch.models.gsm_delete_response import GsmDeleteResponse
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

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.GsmApi(api_client)
    gsm_delete_request = hyperswitch.GsmDeleteRequest() # GsmDeleteRequest | 

    try:
        # Gsm - Delete
        api_response = api_instance.delete_gsm_rule(gsm_delete_request)
        print("The response of GsmApi->delete_gsm_rule:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling GsmApi->delete_gsm_rule: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **gsm_delete_request** | [**GsmDeleteRequest**](GsmDeleteRequest.md)|  | 

### Return type

[**GsmDeleteResponse**](GsmDeleteResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gsm deleted |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_gsm_rule**
> GsmResponse retrieve_gsm_rule(gsm_retrieve_request)

Gsm - Get

Retrieves a Gsm Rule

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.gsm_response import GsmResponse
from hyperswitch.models.gsm_retrieve_request import GsmRetrieveRequest
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

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.GsmApi(api_client)
    gsm_retrieve_request = hyperswitch.GsmRetrieveRequest() # GsmRetrieveRequest | 

    try:
        # Gsm - Get
        api_response = api_instance.retrieve_gsm_rule(gsm_retrieve_request)
        print("The response of GsmApi->retrieve_gsm_rule:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling GsmApi->retrieve_gsm_rule: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **gsm_retrieve_request** | [**GsmRetrieveRequest**](GsmRetrieveRequest.md)|  | 

### Return type

[**GsmResponse**](GsmResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gsm retrieved |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_gsm_rule**
> GsmResponse update_gsm_rule(gsm_update_request)

Gsm - Update

Updates a Gsm Rule

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.gsm_response import GsmResponse
from hyperswitch.models.gsm_update_request import GsmUpdateRequest
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

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.GsmApi(api_client)
    gsm_update_request = hyperswitch.GsmUpdateRequest() # GsmUpdateRequest | 

    try:
        # Gsm - Update
        api_response = api_instance.update_gsm_rule(gsm_update_request)
        print("The response of GsmApi->update_gsm_rule:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling GsmApi->update_gsm_rule: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **gsm_update_request** | [**GsmUpdateRequest**](GsmUpdateRequest.md)|  | 

### Return type

[**GsmResponse**](GsmResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Gsm updated |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

