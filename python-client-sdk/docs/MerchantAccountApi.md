# hyperswitch.MerchantAccountApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_merchant_account**](MerchantAccountApi.md#create_a_merchant_account) | **POST** /accounts | Merchant Account - Create
[**delete_a_merchant_account**](MerchantAccountApi.md#delete_a_merchant_account) | **DELETE** /accounts/{account_id} | Merchant Account - Delete
[**enable_disable_kv_for_a_merchant_account**](MerchantAccountApi.md#enable_disable_kv_for_a_merchant_account) | **POST** /accounts/{account_id}/kv | Merchant Account - KV Status
[**retrieve_a_merchant_account**](MerchantAccountApi.md#retrieve_a_merchant_account) | **GET** /accounts/{account_id} | Merchant Account - Retrieve
[**update_a_merchant_account**](MerchantAccountApi.md#update_a_merchant_account) | **POST** /accounts/{account_id} | Merchant Account - Update


# **create_a_merchant_account**
> MerchantAccountResponse create_a_merchant_account(merchant_account_create)

Merchant Account - Create

Create a new account for a *merchant* and the *merchant* could be a seller or retailer or client who likes to receive and send payments.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_account_create import MerchantAccountCreate
from hyperswitch.models.merchant_account_response import MerchantAccountResponse
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
    api_instance = hyperswitch.MerchantAccountApi(api_client)
    merchant_account_create = {"merchant_id":"merchant_abc"} # MerchantAccountCreate | 

    try:
        # Merchant Account - Create
        api_response = api_instance.create_a_merchant_account(merchant_account_create)
        print("The response of MerchantAccountApi->create_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantAccountApi->create_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_account_create** | [**MerchantAccountCreate**](MerchantAccountCreate.md)|  | 

### Return type

[**MerchantAccountResponse**](MerchantAccountResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Account Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_a_merchant_account**
> MerchantAccountDeleteResponse delete_a_merchant_account(account_id)

Merchant Account - Delete

Delete a *merchant* account

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_account_delete_response import MerchantAccountDeleteResponse
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
    api_instance = hyperswitch.MerchantAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account

    try:
        # Merchant Account - Delete
        api_response = api_instance.delete_a_merchant_account(account_id)
        print("The response of MerchantAccountApi->delete_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantAccountApi->delete_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 

### Return type

[**MerchantAccountDeleteResponse**](MerchantAccountDeleteResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Account Deleted |  -  |
**404** | Merchant account not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **enable_disable_kv_for_a_merchant_account**
> ToggleKVResponse enable_disable_kv_for_a_merchant_account(account_id, toggle_kv_request)

Merchant Account - KV Status

Toggle KV mode for the Merchant Account

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.toggle_kv_request import ToggleKVRequest
from hyperswitch.models.toggle_kv_response import ToggleKVResponse
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
    api_instance = hyperswitch.MerchantAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    toggle_kv_request = {"kv_enabled":"false"} # ToggleKVRequest | 

    try:
        # Merchant Account - KV Status
        api_response = api_instance.enable_disable_kv_for_a_merchant_account(account_id, toggle_kv_request)
        print("The response of MerchantAccountApi->enable_disable_kv_for_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantAccountApi->enable_disable_kv_for_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **toggle_kv_request** | [**ToggleKVRequest**](ToggleKVRequest.md)|  | 

### Return type

[**ToggleKVResponse**](ToggleKVResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | KV mode is enabled/disabled for Merchant Account |  -  |
**400** | Invalid data |  -  |
**404** | Merchant account not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_merchant_account**
> MerchantAccountResponse retrieve_a_merchant_account(account_id)

Merchant Account - Retrieve

Retrieve a *merchant* account details.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_account_response import MerchantAccountResponse
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
    api_instance = hyperswitch.MerchantAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account

    try:
        # Merchant Account - Retrieve
        api_response = api_instance.retrieve_a_merchant_account(account_id)
        print("The response of MerchantAccountApi->retrieve_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantAccountApi->retrieve_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 

### Return type

[**MerchantAccountResponse**](MerchantAccountResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Account Retrieved |  -  |
**404** | Merchant account not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_merchant_account**
> MerchantAccountResponse update_a_merchant_account(account_id, merchant_account_update)

Merchant Account - Update

Updates details of an existing merchant account. Helpful in updating merchant details such as email, contact details, or other configuration details like webhook, routing algorithm etc

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_account_response import MerchantAccountResponse
from hyperswitch.models.merchant_account_update import MerchantAccountUpdate
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
    api_instance = hyperswitch.MerchantAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    merchant_account_update = {"merchant_id":"merchant_abc","merchant_name":"merchant_name"} # MerchantAccountUpdate | 

    try:
        # Merchant Account - Update
        api_response = api_instance.update_a_merchant_account(account_id, merchant_account_update)
        print("The response of MerchantAccountApi->update_a_merchant_account:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantAccountApi->update_a_merchant_account: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **merchant_account_update** | [**MerchantAccountUpdate**](MerchantAccountUpdate.md)|  | 

### Return type

[**MerchantAccountResponse**](MerchantAccountResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Account Updated |  -  |
**404** | Merchant account not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

