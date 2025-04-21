# hyperswitch.MandatesApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**list_mandates_for_a_customer**](MandatesApi.md#list_mandates_for_a_customer) | **POST** /customers/{customer_id}/mandates | Mandates - Customer Mandates List
[**retrieve_a_mandate**](MandatesApi.md#retrieve_a_mandate) | **GET** /mandates/{mandate_id} | Mandates - Retrieve Mandate
[**revoke_a_mandate**](MandatesApi.md#revoke_a_mandate) | **POST** /mandates/revoke/{mandate_id} | Mandates - Revoke Mandate


# **list_mandates_for_a_customer**
> List[MandateResponse] list_mandates_for_a_customer()

Mandates - Customer Mandates List

Lists all the mandates for a particular customer id.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.mandate_response import MandateResponse
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
    api_instance = hyperswitch.MandatesApi(api_client)

    try:
        # Mandates - Customer Mandates List
        api_response = api_instance.list_mandates_for_a_customer()
        print("The response of MandatesApi->list_mandates_for_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MandatesApi->list_mandates_for_a_customer: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

[**List[MandateResponse]**](MandateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of retrieved mandates for a customer |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_mandate**
> MandateResponse retrieve_a_mandate(mandate_id)

Mandates - Retrieve Mandate

Retrieves a mandate created using the Payments/Create API

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.mandate_response import MandateResponse
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
    api_instance = hyperswitch.MandatesApi(api_client)
    mandate_id = 'mandate_id_example' # str | The identifier for mandate

    try:
        # Mandates - Retrieve Mandate
        api_response = api_instance.retrieve_a_mandate(mandate_id)
        print("The response of MandatesApi->retrieve_a_mandate:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MandatesApi->retrieve_a_mandate: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **mandate_id** | **str**| The identifier for mandate | 

### Return type

[**MandateResponse**](MandateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The mandate was retrieved successfully |  -  |
**404** | Mandate does not exist in our records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **revoke_a_mandate**
> MandateRevokedResponse revoke_a_mandate(mandate_id)

Mandates - Revoke Mandate

Revokes a mandate created using the Payments/Create API

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.mandate_revoked_response import MandateRevokedResponse
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
    api_instance = hyperswitch.MandatesApi(api_client)
    mandate_id = 'mandate_id_example' # str | The identifier for a mandate

    try:
        # Mandates - Revoke Mandate
        api_response = api_instance.revoke_a_mandate(mandate_id)
        print("The response of MandatesApi->revoke_a_mandate:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling MandatesApi->revoke_a_mandate: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **mandate_id** | **str**| The identifier for a mandate | 

### Return type

[**MandateRevokedResponse**](MandateRevokedResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The mandate was revoked successfully |  -  |
**400** | Mandate does not exist in our records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

