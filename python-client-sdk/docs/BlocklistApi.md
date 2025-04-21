# hyperswitch.BlocklistApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**block_a_fingerprint**](BlocklistApi.md#block_a_fingerprint) | **POST** /blocklist | 
[**list_blocked_fingerprints_of_a_particular_kind**](BlocklistApi.md#list_blocked_fingerprints_of_a_particular_kind) | **GET** /blocklist | 
[**toggle_blocklist_guard_for_a_particular_merchant**](BlocklistApi.md#toggle_blocklist_guard_for_a_particular_merchant) | **POST** /blocklist/toggle | 
[**unblock_a_fingerprint**](BlocklistApi.md#unblock_a_fingerprint) | **DELETE** /blocklist | 


# **block_a_fingerprint**
> BlocklistResponse block_a_fingerprint(blocklist_request)

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.blocklist_request import BlocklistRequest
from hyperswitch.models.blocklist_response import BlocklistResponse
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
    api_instance = hyperswitch.BlocklistApi(api_client)
    blocklist_request = hyperswitch.BlocklistRequest() # BlocklistRequest | 

    try:
        api_response = api_instance.block_a_fingerprint(blocklist_request)
        print("The response of BlocklistApi->block_a_fingerprint:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling BlocklistApi->block_a_fingerprint: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **blocklist_request** | [**BlocklistRequest**](BlocklistRequest.md)|  | 

### Return type

[**BlocklistResponse**](BlocklistResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Fingerprint Blocked |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_blocked_fingerprints_of_a_particular_kind**
> BlocklistResponse list_blocked_fingerprints_of_a_particular_kind(data_kind)

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.blocklist_data_kind import BlocklistDataKind
from hyperswitch.models.blocklist_response import BlocklistResponse
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
    api_instance = hyperswitch.BlocklistApi(api_client)
    data_kind = hyperswitch.BlocklistDataKind() # BlocklistDataKind | Kind of the fingerprint list requested

    try:
        api_response = api_instance.list_blocked_fingerprints_of_a_particular_kind(data_kind)
        print("The response of BlocklistApi->list_blocked_fingerprints_of_a_particular_kind:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling BlocklistApi->list_blocked_fingerprints_of_a_particular_kind: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **data_kind** | [**BlocklistDataKind**](.md)| Kind of the fingerprint list requested | 

### Return type

[**BlocklistResponse**](BlocklistResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Blocked Fingerprints |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **toggle_blocklist_guard_for_a_particular_merchant**
> ToggleBlocklistResponse toggle_blocklist_guard_for_a_particular_merchant(status)

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.toggle_blocklist_response import ToggleBlocklistResponse
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
    api_instance = hyperswitch.BlocklistApi(api_client)
    status = True # bool | Boolean value to enable/disable blocklist

    try:
        api_response = api_instance.toggle_blocklist_guard_for_a_particular_merchant(status)
        print("The response of BlocklistApi->toggle_blocklist_guard_for_a_particular_merchant:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling BlocklistApi->toggle_blocklist_guard_for_a_particular_merchant: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **status** | **bool**| Boolean value to enable/disable blocklist | 

### Return type

[**ToggleBlocklistResponse**](ToggleBlocklistResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Blocklist guard enabled/disabled |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **unblock_a_fingerprint**
> BlocklistResponse unblock_a_fingerprint(blocklist_request)

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.blocklist_request import BlocklistRequest
from hyperswitch.models.blocklist_response import BlocklistResponse
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
    api_instance = hyperswitch.BlocklistApi(api_client)
    blocklist_request = hyperswitch.BlocklistRequest() # BlocklistRequest | 

    try:
        api_response = api_instance.unblock_a_fingerprint(blocklist_request)
        print("The response of BlocklistApi->unblock_a_fingerprint:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling BlocklistApi->unblock_a_fingerprint: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **blocklist_request** | [**BlocklistRequest**](BlocklistRequest.md)|  | 

### Return type

[**BlocklistResponse**](BlocklistResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Fingerprint Unblocked |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

