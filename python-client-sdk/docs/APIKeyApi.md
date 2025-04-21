# hyperswitch.APIKeyApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_an_api_key**](APIKeyApi.md#create_an_api_key) | **POST** /api_keys/{merchant_id} | API Key - Create
[**list_all_api_keys_associated_with_a_merchant_account**](APIKeyApi.md#list_all_api_keys_associated_with_a_merchant_account) | **GET** /api_keys/{merchant_id}/list | API Key - List
[**retrieve_an_api_key**](APIKeyApi.md#retrieve_an_api_key) | **GET** /api_keys/{merchant_id}/{key_id} | API Key - Retrieve
[**revoke_an_api_key**](APIKeyApi.md#revoke_an_api_key) | **DELETE** /api_keys/{merchant_id}/{key_id} | API Key - Revoke
[**update_an_api_key**](APIKeyApi.md#update_an_api_key) | **POST** /api_keys/{merchant_id}/{key_id} | API Key - Update


# **create_an_api_key**
> CreateApiKeyResponse create_an_api_key(merchant_id, create_api_key_request)

API Key - Create

Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
displayed only once on creation, so ensure you store it securely.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.create_api_key_request import CreateApiKeyRequest
from hyperswitch.models.create_api_key_response import CreateApiKeyResponse
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
    api_instance = hyperswitch.APIKeyApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the merchant account
    create_api_key_request = hyperswitch.CreateApiKeyRequest() # CreateApiKeyRequest | 

    try:
        # API Key - Create
        api_response = api_instance.create_an_api_key(merchant_id, create_api_key_request)
        print("The response of APIKeyApi->create_an_api_key:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling APIKeyApi->create_an_api_key: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the merchant account | 
 **create_api_key_request** | [**CreateApiKeyRequest**](CreateApiKeyRequest.md)|  | 

### Return type

[**CreateApiKeyResponse**](CreateApiKeyResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | API Key created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_api_keys_associated_with_a_merchant_account**
> List[RetrieveApiKeyResponse] list_all_api_keys_associated_with_a_merchant_account(merchant_id, limit=limit, skip=skip)

API Key - List

List all the API Keys associated to a merchant account.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.retrieve_api_key_response import RetrieveApiKeyResponse
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
    api_instance = hyperswitch.APIKeyApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the merchant account
    limit = 56 # int | The maximum number of API Keys to include in the response (optional)
    skip = 56 # int | The number of API Keys to skip when retrieving the list of API keys. (optional)

    try:
        # API Key - List
        api_response = api_instance.list_all_api_keys_associated_with_a_merchant_account(merchant_id, limit=limit, skip=skip)
        print("The response of APIKeyApi->list_all_api_keys_associated_with_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling APIKeyApi->list_all_api_keys_associated_with_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the merchant account | 
 **limit** | **int**| The maximum number of API Keys to include in the response | [optional] 
 **skip** | **int**| The number of API Keys to skip when retrieving the list of API keys. | [optional] 

### Return type

[**List[RetrieveApiKeyResponse]**](RetrieveApiKeyResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of API Keys retrieved successfully |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_an_api_key**
> RetrieveApiKeyResponse retrieve_an_api_key(merchant_id, key_id)

API Key - Retrieve

Retrieve information about the specified API Key.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.retrieve_api_key_response import RetrieveApiKeyResponse
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
    api_instance = hyperswitch.APIKeyApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the merchant account
    key_id = 'key_id_example' # str | The unique identifier for the API Key

    try:
        # API Key - Retrieve
        api_response = api_instance.retrieve_an_api_key(merchant_id, key_id)
        print("The response of APIKeyApi->retrieve_an_api_key:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling APIKeyApi->retrieve_an_api_key: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the merchant account | 
 **key_id** | **str**| The unique identifier for the API Key | 

### Return type

[**RetrieveApiKeyResponse**](RetrieveApiKeyResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | API Key retrieved |  -  |
**404** | API Key not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **revoke_an_api_key**
> RevokeApiKeyResponse revoke_an_api_key(merchant_id, key_id)

API Key - Revoke

Revoke the specified API Key. Once revoked, the API Key can no longer be used for
authenticating with our APIs.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.revoke_api_key_response import RevokeApiKeyResponse
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
    api_instance = hyperswitch.APIKeyApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the merchant account
    key_id = 'key_id_example' # str | The unique identifier for the API Key

    try:
        # API Key - Revoke
        api_response = api_instance.revoke_an_api_key(merchant_id, key_id)
        print("The response of APIKeyApi->revoke_an_api_key:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling APIKeyApi->revoke_an_api_key: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the merchant account | 
 **key_id** | **str**| The unique identifier for the API Key | 

### Return type

[**RevokeApiKeyResponse**](RevokeApiKeyResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | API Key revoked |  -  |
**404** | API Key not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_an_api_key**
> RetrieveApiKeyResponse update_an_api_key(merchant_id, key_id, update_api_key_request)

API Key - Update

Update information for the specified API Key.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.retrieve_api_key_response import RetrieveApiKeyResponse
from hyperswitch.models.update_api_key_request import UpdateApiKeyRequest
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
    api_instance = hyperswitch.APIKeyApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the merchant account
    key_id = 'key_id_example' # str | The unique identifier for the API Key
    update_api_key_request = hyperswitch.UpdateApiKeyRequest() # UpdateApiKeyRequest | 

    try:
        # API Key - Update
        api_response = api_instance.update_an_api_key(merchant_id, key_id, update_api_key_request)
        print("The response of APIKeyApi->update_an_api_key:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling APIKeyApi->update_an_api_key: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the merchant account | 
 **key_id** | **str**| The unique identifier for the API Key | 
 **update_api_key_request** | [**UpdateApiKeyRequest**](UpdateApiKeyRequest.md)|  | 

### Return type

[**RetrieveApiKeyResponse**](RetrieveApiKeyResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | API Key updated |  -  |
**404** | API Key not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

