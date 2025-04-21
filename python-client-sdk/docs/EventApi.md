# hyperswitch.EventApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**list_all_delivery_attempts_for_an_event**](EventApi.md#list_all_delivery_attempts_for_an_event) | **GET** /events/{merchant_id}/{event_id}/attempts | Events - Delivery Attempt List
[**list_all_events_associated_with_a_merchant_account_or_profile**](EventApi.md#list_all_events_associated_with_a_merchant_account_or_profile) | **POST** /events/{merchant_id} | Events - List
[**list_all_events_associated_with_a_profile**](EventApi.md#list_all_events_associated_with_a_profile) | **POST** /events/profile/list | Events - List
[**manually_retry_the_delivery_of_an_event**](EventApi.md#manually_retry_the_delivery_of_an_event) | **POST** /events/{merchant_id}/{event_id}/retry | Events - Manual Retry


# **list_all_delivery_attempts_for_an_event**
> List[EventRetrieveResponse] list_all_delivery_attempts_for_an_event(merchant_id, event_id)

Events - Delivery Attempt List

List all delivery attempts for the specified Event.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.event_retrieve_response import EventRetrieveResponse
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
    api_instance = hyperswitch.EventApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the Merchant Account.
    event_id = 'event_id_example' # str | The unique identifier for the Event

    try:
        # Events - Delivery Attempt List
        api_response = api_instance.list_all_delivery_attempts_for_an_event(merchant_id, event_id)
        print("The response of EventApi->list_all_delivery_attempts_for_an_event:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling EventApi->list_all_delivery_attempts_for_an_event: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the Merchant Account. | 
 **event_id** | **str**| The unique identifier for the Event | 

### Return type

[**List[EventRetrieveResponse]**](EventRetrieveResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of delivery attempts retrieved successfully |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_events_associated_with_a_merchant_account_or_profile**
> TotalEventsResponse list_all_events_associated_with_a_merchant_account_or_profile(merchant_id, event_list_constraints)

Events - List

List all Events associated with a Merchant Account or Profile.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.event_list_constraints import EventListConstraints
from hyperswitch.models.total_events_response import TotalEventsResponse
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
    api_instance = hyperswitch.EventApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the Merchant Account.
    event_list_constraints = {"created_after":"2023-01-01T00:00:00","created_before":"2023-01-31T23:59:59","event_classes":["payments","refunds"],"event_types":["payment_succeeded"],"is_delivered":true,"limit":5,"object_id":"{{object_id}}","offset":0,"profile_id":"{{profile_id}}"} # EventListConstraints | The constraints that can be applied when listing Events.

    try:
        # Events - List
        api_response = api_instance.list_all_events_associated_with_a_merchant_account_or_profile(merchant_id, event_list_constraints)
        print("The response of EventApi->list_all_events_associated_with_a_merchant_account_or_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling EventApi->list_all_events_associated_with_a_merchant_account_or_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the Merchant Account. | 
 **event_list_constraints** | [**EventListConstraints**](EventListConstraints.md)| The constraints that can be applied when listing Events. | 

### Return type

[**TotalEventsResponse**](TotalEventsResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of Events retrieved successfully |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_events_associated_with_a_profile**
> TotalEventsResponse list_all_events_associated_with_a_profile(event_list_constraints)

Events - List

List all Events associated with a Profile.

### Example


```python
import hyperswitch
from hyperswitch.models.event_list_constraints import EventListConstraints
from hyperswitch.models.total_events_response import TotalEventsResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.EventApi(api_client)
    event_list_constraints = {"created_after":"2023-01-01T00:00:00","created_before":"2023-01-31T23:59:59","event_classes":["payments","refunds"],"event_types":["payment_succeeded"],"is_delivered":true,"limit":5,"object_id":"{{object_id}}","offset":0,"profile_id":"{{profile_id}}"} # EventListConstraints | The constraints that can be applied when listing Events.

    try:
        # Events - List
        api_response = api_instance.list_all_events_associated_with_a_profile(event_list_constraints)
        print("The response of EventApi->list_all_events_associated_with_a_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling EventApi->list_all_events_associated_with_a_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **event_list_constraints** | [**EventListConstraints**](EventListConstraints.md)| The constraints that can be applied when listing Events. | 

### Return type

[**TotalEventsResponse**](TotalEventsResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of Events retrieved successfully |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **manually_retry_the_delivery_of_an_event**
> EventRetrieveResponse manually_retry_the_delivery_of_an_event(merchant_id, event_id)

Events - Manual Retry

Manually retry the delivery of the specified Event.

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.event_retrieve_response import EventRetrieveResponse
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
    api_instance = hyperswitch.EventApi(api_client)
    merchant_id = 'merchant_id_example' # str | The unique identifier for the Merchant Account.
    event_id = 'event_id_example' # str | The unique identifier for the Event

    try:
        # Events - Manual Retry
        api_response = api_instance.manually_retry_the_delivery_of_an_event(merchant_id, event_id)
        print("The response of EventApi->manually_retry_the_delivery_of_an_event:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling EventApi->manually_retry_the_delivery_of_an_event: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **merchant_id** | **str**| The unique identifier for the Merchant Account. | 
 **event_id** | **str**| The unique identifier for the Event | 

### Return type

[**EventRetrieveResponse**](EventRetrieveResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The delivery of the Event was attempted. Check the &#x60;response&#x60; field in the response payload to identify the status of the delivery attempt. |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

