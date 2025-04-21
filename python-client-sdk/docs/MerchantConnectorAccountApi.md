# hyperswitch.MerchantConnectorAccountApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_merchant_connector**](MerchantConnectorAccountApi.md#create_a_merchant_connector) | **POST** /accounts/{account_id}/connectors | Merchant Connector - Create
[**delete_a_merchant_connector**](MerchantConnectorAccountApi.md#delete_a_merchant_connector) | **DELETE** /accounts/{account_id}/connectors/{connector_id} | Merchant Connector - Delete
[**list_all_merchant_connectors**](MerchantConnectorAccountApi.md#list_all_merchant_connectors) | **GET** /accounts/{account_id}/connectors | Merchant Connector - List
[**retrieve_a_merchant_connector**](MerchantConnectorAccountApi.md#retrieve_a_merchant_connector) | **GET** /accounts/{account_id}/connectors/{connector_id} | Merchant Connector - Retrieve
[**update_a_merchant_connector**](MerchantConnectorAccountApi.md#update_a_merchant_connector) | **POST** /accounts/{account_id}/connectors/{connector_id} | Merchant Connector - Update


# **create_a_merchant_connector**
> MerchantConnectorResponse create_a_merchant_connector(merchant_connector_create)

Merchant Connector - Create

Creates a new Merchant Connector for the merchant account. The connector could be a payment processor/facilitator/acquirer or a provider of specialized services like Fraud/Accounting etc.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_connector_create import MerchantConnectorCreate
from hyperswitch.models.merchant_connector_response import MerchantConnectorResponse
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
    api_instance = hyperswitch.MerchantConnectorAccountApi(api_client)
    merchant_connector_create = {"connector_account_details":{"api_key":"{{adyen-api-key}}","auth_type":"BodyKey","key1":"{{adyen_merchant_account}}"},"connector_label":"EU_adyen","connector_name":"adyen","connector_type":"payment_processor"} # MerchantConnectorCreate | 

    try:
        # Merchant Connector - Create
        api_response = api_instance.create_a_merchant_connector(merchant_connector_create)
        print("The response of MerchantConnectorAccountApi->create_a_merchant_connector:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantConnectorAccountApi->create_a_merchant_connector: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_connector_create** | [**MerchantConnectorCreate**](MerchantConnectorCreate.md)|  | 

### Return type

[**MerchantConnectorResponse**](MerchantConnectorResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Connector Created |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_a_merchant_connector**
> MerchantConnectorDeleteResponse delete_a_merchant_connector(account_id, connector_id)

Merchant Connector - Delete

Delete or Detach a Merchant Connector from Merchant Account

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_connector_delete_response import MerchantConnectorDeleteResponse
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
    api_instance = hyperswitch.MerchantConnectorAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    connector_id = 56 # int | The unique identifier for the Merchant Connector

    try:
        # Merchant Connector - Delete
        api_response = api_instance.delete_a_merchant_connector(account_id, connector_id)
        print("The response of MerchantConnectorAccountApi->delete_a_merchant_connector:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantConnectorAccountApi->delete_a_merchant_connector: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **connector_id** | **int**| The unique identifier for the Merchant Connector | 

### Return type

[**MerchantConnectorDeleteResponse**](MerchantConnectorDeleteResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Connector Deleted |  -  |
**401** | Unauthorized request |  -  |
**404** | Merchant Connector does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_merchant_connectors**
> List[MerchantConnectorListResponse] list_all_merchant_connectors(account_id)

Merchant Connector - List

List Merchant Connector Details for the merchant

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_connector_list_response import MerchantConnectorListResponse
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
    api_instance = hyperswitch.MerchantConnectorAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account

    try:
        # Merchant Connector - List
        api_response = api_instance.list_all_merchant_connectors(account_id)
        print("The response of MerchantConnectorAccountApi->list_all_merchant_connectors:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantConnectorAccountApi->list_all_merchant_connectors: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 

### Return type

[**List[MerchantConnectorListResponse]**](MerchantConnectorListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Connector list retrieved successfully |  -  |
**401** | Unauthorized request |  -  |
**404** | Merchant Connector does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_merchant_connector**
> MerchantConnectorResponse retrieve_a_merchant_connector(account_id, connector_id)

Merchant Connector - Retrieve

Retrieves details of a Connector account

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_connector_response import MerchantConnectorResponse
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
    api_instance = hyperswitch.MerchantConnectorAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    connector_id = 56 # int | The unique identifier for the Merchant Connector

    try:
        # Merchant Connector - Retrieve
        api_response = api_instance.retrieve_a_merchant_connector(account_id, connector_id)
        print("The response of MerchantConnectorAccountApi->retrieve_a_merchant_connector:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantConnectorAccountApi->retrieve_a_merchant_connector: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **connector_id** | **int**| The unique identifier for the Merchant Connector | 

### Return type

[**MerchantConnectorResponse**](MerchantConnectorResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Connector retrieved successfully |  -  |
**401** | Unauthorized request |  -  |
**404** | Merchant Connector does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_merchant_connector**
> MerchantConnectorResponse update_a_merchant_connector(account_id, connector_id, merchant_connector_update)

Merchant Connector - Update

To update an existing Merchant Connector account. Helpful in enabling/disabling different payment methods and other settings for the connector

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.merchant_connector_response import MerchantConnectorResponse
from hyperswitch.models.merchant_connector_update import MerchantConnectorUpdate
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
    api_instance = hyperswitch.MerchantConnectorAccountApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    connector_id = 56 # int | The unique identifier for the Merchant Connector
    merchant_connector_update = {"connector_type":"payment_processor","payment_methods_enabled":[{"payment_method":"card"}]} # MerchantConnectorUpdate | 

    try:
        # Merchant Connector - Update
        api_response = api_instance.update_a_merchant_connector(account_id, connector_id, merchant_connector_update)
        print("The response of MerchantConnectorAccountApi->update_a_merchant_connector:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MerchantConnectorAccountApi->update_a_merchant_connector: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **connector_id** | **int**| The unique identifier for the Merchant Connector | 
 **merchant_connector_update** | [**MerchantConnectorUpdate**](MerchantConnectorUpdate.md)|  | 

### Return type

[**MerchantConnectorResponse**](MerchantConnectorResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Merchant Connector Updated |  -  |
**401** | Unauthorized request |  -  |
**404** | Merchant Connector does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

