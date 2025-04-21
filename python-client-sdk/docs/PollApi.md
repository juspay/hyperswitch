# hyperswitch.PollApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**retrieve_poll_status**](PollApi.md#retrieve_poll_status) | **GET** /poll/status/{poll_id} | Poll - Retrieve Poll Status


# **retrieve_poll_status**
> PollResponse retrieve_poll_status(poll_id)

Poll - Retrieve Poll Status

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.poll_response import PollResponse
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

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PollApi(api_client)
    poll_id = 'poll_id_example' # str | The identifier for poll

    try:
        # Poll - Retrieve Poll Status
        api_response = api_instance.retrieve_poll_status(poll_id)
        print("The response of PollApi->retrieve_poll_status:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PollApi->retrieve_poll_status: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **poll_id** | **str**| The identifier for poll | 

### Return type

[**PollResponse**](PollResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | The poll status was retrieved successfully |  -  |
**404** | Poll not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

