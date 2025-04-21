# hyperswitch.PayoutsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancel_a_payout**](PayoutsApi.md#cancel_a_payout) | **POST** /payouts/{payout_id}/cancel | Payouts - Cancel
[**confirm_a_payout**](PayoutsApi.md#confirm_a_payout) | **POST** /payouts/{payout_id}/confirm | Payouts - Confirm
[**create_a_payout**](PayoutsApi.md#create_a_payout) | **POST** /payouts/create | Payouts - Create
[**filter_payouts_using_specific_constraints**](PayoutsApi.md#filter_payouts_using_specific_constraints) | **POST** /payouts/list | Payouts - List using filters
[**fulfill_a_payout**](PayoutsApi.md#fulfill_a_payout) | **POST** /payouts/{payout_id}/fulfill | Payouts - Fulfill
[**list_available_payout_filters**](PayoutsApi.md#list_available_payout_filters) | **POST** /payouts/filter | Payouts - List available filters
[**list_payouts_using_generic_constraints**](PayoutsApi.md#list_payouts_using_generic_constraints) | **GET** /payouts/list | Payouts - List
[**retrieve_a_payout**](PayoutsApi.md#retrieve_a_payout) | **GET** /payouts/{payout_id} | Payouts - Retrieve
[**update_a_payout**](PayoutsApi.md#update_a_payout) | **POST** /payouts/{payout_id} | Payouts - Update


# **cancel_a_payout**
> PayoutCreateResponse cancel_a_payout(payout_id, payout_cancel_request)

Payouts - Cancel

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_cancel_request import PayoutCancelRequest
from hyperswitch.models.payout_create_response import PayoutCreateResponse
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_id = 'payout_id_example' # str | The identifier for payout
    payout_cancel_request = hyperswitch.PayoutCancelRequest() # PayoutCancelRequest | 

    try:
        # Payouts - Cancel
        api_response = api_instance.cancel_a_payout(payout_id, payout_cancel_request)
        print("The response of PayoutsApi->cancel_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->cancel_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_id** | **str**| The identifier for payout | 
 **payout_cancel_request** | [**PayoutCancelRequest**](PayoutCancelRequest.md)|  | 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout cancelled |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **confirm_a_payout**
> PayoutCreateResponse confirm_a_payout(payout_id, payout_confirm_request)

Payouts - Confirm

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_confirm_request import PayoutConfirmRequest
from hyperswitch.models.payout_create_response import PayoutCreateResponse
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_id = 'payout_id_example' # str | The identifier for payout
    payout_confirm_request = hyperswitch.PayoutConfirmRequest() # PayoutConfirmRequest | 

    try:
        # Payouts - Confirm
        api_response = api_instance.confirm_a_payout(payout_id, payout_confirm_request)
        print("The response of PayoutsApi->confirm_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->confirm_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_id** | **str**| The identifier for payout | 
 **payout_confirm_request** | [**PayoutConfirmRequest**](PayoutConfirmRequest.md)|  | 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout updated |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_a_payout**
> PayoutCreateResponse create_a_payout(payouts_create_request)

Payouts - Create

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_create_response import PayoutCreateResponse
from hyperswitch.models.payouts_create_request import PayoutsCreateRequest
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payouts_create_request = hyperswitch.PayoutsCreateRequest() # PayoutsCreateRequest | 

    try:
        # Payouts - Create
        api_response = api_instance.create_a_payout(payouts_create_request)
        print("The response of PayoutsApi->create_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->create_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payouts_create_request** | [**PayoutsCreateRequest**](PayoutsCreateRequest.md)|  | 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout created |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **filter_payouts_using_specific_constraints**
> PayoutListResponse filter_payouts_using_specific_constraints(payout_list_filter_constraints)

Payouts - List using filters

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_list_filter_constraints import PayoutListFilterConstraints
from hyperswitch.models.payout_list_response import PayoutListResponse
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_list_filter_constraints = hyperswitch.PayoutListFilterConstraints() # PayoutListFilterConstraints | 

    try:
        # Payouts - List using filters
        api_response = api_instance.filter_payouts_using_specific_constraints(payout_list_filter_constraints)
        print("The response of PayoutsApi->filter_payouts_using_specific_constraints:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->filter_payouts_using_specific_constraints: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_list_filter_constraints** | [**PayoutListFilterConstraints**](PayoutListFilterConstraints.md)|  | 

### Return type

[**PayoutListResponse**](PayoutListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payouts filtered |  -  |
**404** | Payout not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **fulfill_a_payout**
> PayoutCreateResponse fulfill_a_payout(payout_id, payout_fulfill_request)

Payouts - Fulfill

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_create_response import PayoutCreateResponse
from hyperswitch.models.payout_fulfill_request import PayoutFulfillRequest
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_id = 'payout_id_example' # str | The identifier for payout
    payout_fulfill_request = hyperswitch.PayoutFulfillRequest() # PayoutFulfillRequest | 

    try:
        # Payouts - Fulfill
        api_response = api_instance.fulfill_a_payout(payout_id, payout_fulfill_request)
        print("The response of PayoutsApi->fulfill_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->fulfill_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_id** | **str**| The identifier for payout | 
 **payout_fulfill_request** | [**PayoutFulfillRequest**](PayoutFulfillRequest.md)|  | 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout fulfilled |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_available_payout_filters**
> PayoutListFilters list_available_payout_filters(time_range)

Payouts - List available filters

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_list_filters import PayoutListFilters
from hyperswitch.models.time_range import TimeRange
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    time_range = hyperswitch.TimeRange() # TimeRange | 

    try:
        # Payouts - List available filters
        api_response = api_instance.list_available_payout_filters(time_range)
        print("The response of PayoutsApi->list_available_payout_filters:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->list_available_payout_filters: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **time_range** | [**TimeRange**](TimeRange.md)|  | 

### Return type

[**PayoutListFilters**](PayoutListFilters.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Filters listed |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_payouts_using_generic_constraints**
> PayoutListResponse list_payouts_using_generic_constraints(customer_id, starting_after, ending_before, limit, created, time_range)

Payouts - List

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_list_response import PayoutListResponse
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    customer_id = 'customer_id_example' # str | The identifier for customer
    starting_after = 'starting_after_example' # str | A cursor for use in pagination, fetch the next list after some object
    ending_before = 'ending_before_example' # str | A cursor for use in pagination, fetch the previous list before some object
    limit = 'limit_example' # str | limit on the number of objects to return
    created = 'created_example' # str | The time at which payout is created
    time_range = 'time_range_example' # str | The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc).

    try:
        # Payouts - List
        api_response = api_instance.list_payouts_using_generic_constraints(customer_id, starting_after, ending_before, limit, created, time_range)
        print("The response of PayoutsApi->list_payouts_using_generic_constraints:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->list_payouts_using_generic_constraints: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The identifier for customer | 
 **starting_after** | **str**| A cursor for use in pagination, fetch the next list after some object | 
 **ending_before** | **str**| A cursor for use in pagination, fetch the previous list before some object | 
 **limit** | **str**| limit on the number of objects to return | 
 **created** | **str**| The time at which payout is created | 
 **time_range** | **str**| The time range for which objects are needed. TimeRange has two fields start_time and end_time from which objects can be filtered as per required scenarios (created_at, time less than, greater than etc). | 

### Return type

[**PayoutListResponse**](PayoutListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payouts listed |  -  |
**404** | Payout not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_payout**
> PayoutCreateResponse retrieve_a_payout(payout_id, force_sync=force_sync)

Payouts - Retrieve

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_create_response import PayoutCreateResponse
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_id = 'payout_id_example' # str | The identifier for payout
    force_sync = True # bool | Sync with the connector to get the payout details (defaults to false) (optional)

    try:
        # Payouts - Retrieve
        api_response = api_instance.retrieve_a_payout(payout_id, force_sync=force_sync)
        print("The response of PayoutsApi->retrieve_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->retrieve_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_id** | **str**| The identifier for payout | 
 **force_sync** | **bool**| Sync with the connector to get the payout details (defaults to false) | [optional] 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout retrieved |  -  |
**404** | Payout does not exist in our records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_payout**
> PayoutCreateResponse update_a_payout(payout_id, payout_update_request)

Payouts - Update

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payout_create_response import PayoutCreateResponse
from hyperswitch.models.payout_update_request import PayoutUpdateRequest
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
    api_instance = hyperswitch.PayoutsApi(api_client)
    payout_id = 'payout_id_example' # str | The identifier for payout
    payout_update_request = hyperswitch.PayoutUpdateRequest() # PayoutUpdateRequest | 

    try:
        # Payouts - Update
        api_response = api_instance.update_a_payout(payout_id, payout_update_request)
        print("The response of PayoutsApi->update_a_payout:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PayoutsApi->update_a_payout: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payout_id** | **str**| The identifier for payout | 
 **payout_update_request** | [**PayoutUpdateRequest**](PayoutUpdateRequest.md)|  | 

### Return type

[**PayoutCreateResponse**](PayoutCreateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payout updated |  -  |
**400** | Missing Mandatory fields |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

