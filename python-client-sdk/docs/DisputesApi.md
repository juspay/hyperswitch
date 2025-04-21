# hyperswitch.DisputesApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**list_disputes**](DisputesApi.md#list_disputes) | **GET** /disputes/list | Disputes - List Disputes
[**retrieve_a_dispute**](DisputesApi.md#retrieve_a_dispute) | **GET** /disputes/{dispute_id} | Disputes - Retrieve Dispute


# **list_disputes**
> List[DisputeResponse] list_disputes(limit=limit, dispute_status=dispute_status, dispute_stage=dispute_stage, reason=reason, connector=connector, received_time=received_time, received_time_lt=received_time_lt, received_time_gt=received_time_gt, received_time_lte=received_time_lte, received_time_gte=received_time_gte)

Disputes - List Disputes

Lists all the Disputes for a merchant

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.dispute_response import DisputeResponse
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
    api_instance = hyperswitch.DisputesApi(api_client)
    limit = 56 # int | The maximum number of Dispute Objects to include in the response (optional)
    dispute_status = hyperswitch.DisputeStatus() # DisputeStatus | The status of dispute (optional)
    dispute_stage = hyperswitch.DisputeStage() # DisputeStage | The stage of dispute (optional)
    reason = 'reason_example' # str | The reason for dispute (optional)
    connector = 'connector_example' # str | The connector linked to dispute (optional)
    received_time = '2013-10-20T19:20:30+01:00' # datetime | The time at which dispute is received (optional)
    received_time_lt = '2013-10-20T19:20:30+01:00' # datetime | Time less than the dispute received time (optional)
    received_time_gt = '2013-10-20T19:20:30+01:00' # datetime | Time greater than the dispute received time (optional)
    received_time_lte = '2013-10-20T19:20:30+01:00' # datetime | Time less than or equals to the dispute received time (optional)
    received_time_gte = '2013-10-20T19:20:30+01:00' # datetime | Time greater than or equals to the dispute received time (optional)

    try:
        # Disputes - List Disputes
        api_response = api_instance.list_disputes(limit=limit, dispute_status=dispute_status, dispute_stage=dispute_stage, reason=reason, connector=connector, received_time=received_time, received_time_lt=received_time_lt, received_time_gt=received_time_gt, received_time_lte=received_time_lte, received_time_gte=received_time_gte)
        print("The response of DisputesApi->list_disputes:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DisputesApi->list_disputes: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **limit** | **int**| The maximum number of Dispute Objects to include in the response | [optional] 
 **dispute_status** | [**DisputeStatus**](.md)| The status of dispute | [optional] 
 **dispute_stage** | [**DisputeStage**](.md)| The stage of dispute | [optional] 
 **reason** | **str**| The reason for dispute | [optional] 
 **connector** | **str**| The connector linked to dispute | [optional] 
 **received_time** | **datetime**| The time at which dispute is received | [optional] 
 **received_time_lt** | **datetime**| Time less than the dispute received time | [optional] 
 **received_time_gt** | **datetime**| Time greater than the dispute received time | [optional] 
 **received_time_lte** | **datetime**| Time less than or equals to the dispute received time | [optional] 
 **received_time_gte** | **datetime**| Time greater than or equals to the dispute received time | [optional] 

### Return type

[**List[DisputeResponse]**](DisputeResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The dispute list was retrieved successfully |  -  |
**401** | Unauthorized request |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_dispute**
> DisputeResponse retrieve_a_dispute(dispute_id)

Disputes - Retrieve Dispute

Retrieves a dispute

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.dispute_response import DisputeResponse
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
    api_instance = hyperswitch.DisputesApi(api_client)
    dispute_id = 'dispute_id_example' # str | The identifier for dispute

    try:
        # Disputes - Retrieve Dispute
        api_response = api_instance.retrieve_a_dispute(dispute_id)
        print("The response of DisputesApi->retrieve_a_dispute:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DisputesApi->retrieve_a_dispute: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **dispute_id** | **str**| The identifier for dispute | 

### Return type

[**DisputeResponse**](DisputeResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The dispute was retrieved successfully |  -  |
**404** | Dispute does not exist in our records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

